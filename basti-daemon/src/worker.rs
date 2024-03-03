use crate::ops::{acquire_task, finish_task, list_tasks, progress_task, requeue_task};
use anyhow::Result;
use async_channel::{Receiver, Sender};
use basti_common::task::{Task, TaskState};
use chrono::Utc;
use etcd_client::Client;
use std::{num::NonZeroUsize, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinSet, time::sleep};

#[tracing::instrument(skip_all)]
pub async fn run_detached(amount: NonZeroUsize, client: Client, node_name: String) {
    let mut join_set = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(amount.get()));
    let (sender, receiver) = async_channel::bounded(1);

    for _ in 0..amount.get() {
        let semaphore = semaphore.clone();
        let receiver: Receiver<(Task, i64)> = receiver.clone();
        let mut client = client.clone();
        join_set.spawn(async move {
            while let Ok((task, revision)) = receiver.recv().await {
                let _permit = semaphore.acquire().await.unwrap();
                let task_id = task.key.id.clone();
                if let Err(_) = work_on_task(&mut client, task, revision).await {
                    tracing::warn!("Lost work on task {task_id}")
                }
            }
        });
    }

    let mut feeding_client = client.clone();
    let mut requeueing_client = client;

    join_set.spawn(async move {
        let semaphore = semaphore.clone();
        loop {
            let _permit = semaphore.acquire().await.unwrap();
            match feed_workers(&mut feeding_client, &sender, node_name.clone()).await {
                Ok(_) => sleep(Duration::from_secs(1)).await,
                Err(_) => {
                    tracing::warn!("Failed to feed workers, waiting 5 seconds");
                    sleep(Duration::from_secs(5)).await;
                }
            };
        }
    });

    join_set.spawn(async move {
        loop {
            match requeue_tasks(&mut requeueing_client).await {
                Ok(_) => sleep(Duration::from_secs(1)).await,
                Err(_) => {
                    tracing::warn!("Failed to requeue tasks, waiting 5 seconds");
                    sleep(Duration::from_secs(5)).await;
                }
            };
        }
    });

    join_set.detach_all();
}

#[tracing::instrument(skip_all, err(Debug))]
async fn work_on_task(client: &mut Client, mut task: Task, mut revision: i64) -> Result<()> {
    while !task.details.remaining.is_zero() {
        const ONE_SECOND: Duration = Duration::from_secs(1);

        let work_duration = if task.details.remaining >= ONE_SECOND {
            ONE_SECOND
        } else {
            task.details.remaining
        };

        tracing::info!(
            "Working on {} for {}.{:03}s",
            task.key.id,
            work_duration.as_secs(),
            work_duration.subsec_millis()
        );

        sleep(work_duration).await;
        (task, revision) = progress_task(client, task, revision, work_duration).await?;
    }

    finish_task(client, &task, revision).await?;

    let time_taken = (Utc::now() - task.details.created_at).to_std()?;
    tracing::info!(
        "Finished task {} after {}.{:03}s",
        task.key.id,
        time_taken.as_secs(),
        time_taken.subsec_millis()
    );

    Ok(())
}

#[tracing::instrument(skip_all, err(Debug))]
async fn feed_workers(
    client: &mut Client,
    sender: &Sender<(Task, i64)>,
    node_name: String,
) -> Result<()> {
    'outer: loop {
        let mut tasks = list_tasks(client, Some(TaskState::Queued), None).await?;

        if tasks.is_empty() {
            break;
        }

        tasks.sort_by(|(a, _), (b, _)| a.details.cmp(&b.details));

        for (task, revision) in tasks.into_iter() {
            let task_id = task.key.id.clone();
            tracing::info!("Trying to acquire task {task_id}");
            match acquire_task(client, task, revision, node_name.clone()).await {
                Ok((task, revision)) => {
                    tracing::info!("Acquired task {}.", task.key.id);
                    sender.send((task, revision)).await?;
                    break 'outer;
                }
                Err(error) => tracing::warn!("Could not acquire task {}: {:?}", task_id, error),
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all, err(Debug))]
async fn requeue_tasks(client: &mut Client) -> Result<()> {
    let now = Utc::now();
    let tasks = list_tasks(client, Some(TaskState::Running), None).await?;

    for (task, revision) in tasks {
        const TEN_SECONDS: Duration = Duration::from_secs(10);
        if (now - task.details.last_update).to_std()? > TEN_SECONDS {
            tracing::info!("Re-queueing task {}", task.key.id);
            requeue_task(client, task, revision).await?;
        }
    }

    Ok(())
}
