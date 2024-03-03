use crate::ops::{
    revision_based::{try_acquire_task, try_finish_task, try_progress_task, try_requeue_task},
    simple::list_tasks,
};
use anyhow::Result;
use async_channel::{Receiver, Sender};
use basti_common::task::*;
use chrono::Utc;
use etcd_client::{GetOptions, KvClient, SortOrder, SortTarget};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinSet, time::sleep};

#[tracing::instrument(skip_all)]
pub async fn run_detached(amount: NonZeroUsize, client: KvClient, node_name: String) {
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
                let task_id = task.key.id;
                if work_on_task(&mut client, task, revision).await.is_err() {
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
                Ok(queue_empty) => {
                    if queue_empty {
                        sleep(Duration::from_millis(500)).await
                    }
                }
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
                Ok(queue_empty) => {
                    if queue_empty {
                        sleep(Duration::from_millis(500)).await
                    }
                }
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
async fn work_on_task(client: &mut KvClient, mut task: Task, mut revision: i64) -> Result<()> {
    while !task.value.remaining.is_zero() {
        const ONE_SECOND: Duration = Duration::from_secs(1);

        let work_duration = if task.value.remaining >= ONE_SECOND {
            ONE_SECOND
        } else {
            task.value.remaining
        };

        tracing::info!(
            "Working on {} for {}.{:03}s",
            task.key.id,
            work_duration.as_secs(),
            work_duration.subsec_millis()
        );

        sleep(work_duration).await;
        (task, revision) = try_progress_task(client, task, revision, work_duration).await?;
    }

    try_finish_task(client, &task, revision).await?;

    let time_taken = (Utc::now() - task.value.created_at).to_std()?;
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
    client: &mut KvClient,
    sender: &Sender<(Task, i64)>,
    node_name: String,
) -> Result<bool> {
    'outer: loop {
        let tasks = list_tasks(
            client,
            Some(TaskState::Queued),
            Some(
                GetOptions::default()
                    .with_sort(SortTarget::Value, SortOrder::Ascend)
                    .with_limit(100),
            ),
        )
        .await?;

        if tasks.is_empty() {
            return Ok(true);
        }

        for (task, revision) in tasks.into_iter() {
            let task_id = task.key.id;
            tracing::info!("Trying to acquire task {task_id}");
            match try_acquire_task(client, task, revision, node_name.clone()).await {
                Ok((task, revision)) => {
                    tracing::info!("Acquired task {}.", task.key.id);
                    sender.send((task, revision)).await?;
                    break 'outer;
                }
                Err(error) => tracing::warn!("Could not acquire task {}: {:?}", task_id, error),
            }
        }
    }

    Ok(false)
}

#[tracing::instrument(skip_all, err(Debug))]
async fn requeue_tasks(client: &mut KvClient) -> Result<bool> {
    let now = Utc::now();
    let tasks = list_tasks(
        client,
        Some(TaskState::Running),
        Some(
            GetOptions::default()
                .with_sort(SortTarget::Mod, SortOrder::Ascend)
                .with_limit(100),
        ),
    )
    .await?;

    if tasks.is_empty() {
        return Ok(true);
    }

    for (task, revision) in tasks {
        const TEN_SECONDS: Duration = Duration::from_secs(10);
        if (now - task.value.last_update).to_std()? > TEN_SECONDS {
            tracing::info!("Re-queueing task {}", task.key.id);
            try_requeue_task(client, task, revision).await?;
        }
    }

    Ok(false)
}
