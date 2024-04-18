use std::num::NonZeroUsize;

use chrono::{TimeDelta, Utc};
use etcd_client::KvClient;
use futures::future;
use tokio::{
    sync::mpsc,
    time::{sleep, Duration},
};

use basti_types::{Task, TaskState, WorkerName};

use crate::{
    ops::{
        acquire_task, find_task, finish_task, list_priorities, list_tasks, progress_task,
        requeue_task,
    },
    shutdown_signal,
};

const WORK_TIMEOUT_DELTA: TimeDelta = TimeDelta::seconds(10);
const WORK_FEEDBACK_INTERVAL: Duration = Duration::from_secs(5);

pub async fn run(amount: NonZeroUsize, client: KvClient, name: WorkerName) {
    let (work_sender, work_receiver) = async_channel::bounded(1);
    let (work_request_sender, mut work_request_receiver) = mpsc::channel(amount.get());

    let worker_handles = (0..amount.get()).map(|_| {
        let task_receiver: async_channel::Receiver<(Task, i64)> = work_receiver.clone();
        let work_request_sender: mpsc::Sender<()> = work_request_sender.clone();
        let mut client = client.clone();
        async move {
            loop {
                work_request_sender.send(()).await.unwrap();
                let (task, revision) = task_receiver.recv().await.unwrap();
                if work_on_task(&mut client, task, revision).await.is_err() {
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    let find_work_handle = {
        let mut client = client.clone();
        async move {
            work_request_receiver.recv().await.unwrap();
            loop {
                match find_work(&mut client, name.clone()).await {
                    Err(_) => sleep(Duration::from_secs(5)).await,
                    Ok(None) => sleep(Duration::from_millis(500)).await,
                    Ok(Some(work)) => {
                        work_sender.send(work).await.unwrap();
                        work_request_receiver.recv().await.unwrap();
                    }
                };
            }
        }
    };

    let requeue_tasks_handle = {
        let mut client = client.clone();
        async move {
            loop {
                match requeue_tasks(&mut client).await {
                    Err(_) => sleep(Duration::from_secs(5)).await,
                    Ok(()) => sleep(Duration::from_millis(500)).await,
                };
            }
        }
    };

    let joined_handles = async {
        tokio::join!(
            future::join_all(worker_handles),
            find_work_handle,
            requeue_tasks_handle
        )
    };

    tokio::select! {
        () = shutdown_signal() => {}
        _ = joined_handles => unreachable!()
    }
}

#[tracing::instrument(skip_all, err(Display))]
async fn work_on_task(
    client: &mut KvClient,
    mut task: Task,
    mut revision: i64,
) -> anyhow::Result<()> {
    let task_id = task.key.id;

    while !task.value.remaining.is_zero() {
        let work_duration = if task.value.remaining >= WORK_FEEDBACK_INTERVAL {
            WORK_FEEDBACK_INTERVAL
        } else {
            task.value.remaining
        };

        tracing::info!(
            id = %task_id,
            event = "working",
            amount = format!(
                "{}.{:03}s",
                work_duration.as_secs(),
                work_duration.subsec_millis()
            ),
        );

        sleep(work_duration).await;

        (task, revision) =
            if let Some(update) = progress_task(client, task, revision, work_duration).await? {
                update
            } else {
                tracing::warn!(id = %task_id, event = "stolen");
                return Ok(());
            };
    }

    if let Some(()) = finish_task(client, &task.key, revision).await? {
        let time_taken = (Utc::now() - task.value.created_at).to_std()?;
        tracing::info!(
            id = %task_id,
            event = "finished",
            total = format!(
                "{}.{:03}s",
                time_taken.as_secs(),
                time_taken.subsec_millis()
            ),
        );
    } else {
        tracing::warn!(id = %task_id, event = "stolen");
    }

    Ok(())
}

#[tracing::instrument(skip_all, err(Display))]
async fn find_work(client: &mut KvClient, name: WorkerName) -> anyhow::Result<Option<(Task, i64)>> {
    let priorities = list_priorities(client, 10).await?;

    for priority in priorities {
        let Some((task, revision)) = find_task(client, priority.id, &[TaskState::Queued]).await?
        else {
            continue;
        };

        if let Some((task, revision)) = acquire_task(client, task, revision, name.clone()).await? {
            tracing::info!(id = %task.key.id, event = "acquired");
            return Ok(Some((task, revision)));
        }

        tracing::info!(
            id = %priority.id,
            event = "stolen"
        );
    }

    Ok(None)
}

#[tracing::instrument(skip_all, err(Display))]
async fn requeue_tasks(client: &mut KvClient) -> anyhow::Result<()> {
    let tasks = list_tasks(client, Some(TaskState::Running), 10).await?;
    let now = Utc::now();

    for (task, revision) in tasks {
        if now - task.value.updated_at < WORK_TIMEOUT_DELTA {
            continue;
        }

        let task_id = task.key.id;
        match requeue_task(client, task, revision).await? {
            Some(_) => tracing::info!(id = %task_id, event = "requeued"),
            None => {
                tracing::info!(id = %task_id, event = "stolen");
            }
        }
    }

    Ok(())
}
