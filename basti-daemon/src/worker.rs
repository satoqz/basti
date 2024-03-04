use crate::ops::{
    acquire_task, find_task, finish_task, list_priorities, list_tasks, progress_task, requeue_task,
    MaybeRevisionError, Revision,
};
use basti_task::{Task, TaskState};
use chrono::{TimeDelta, Utc};
use etcd_client::KvClient;
use std::{num::NonZeroUsize, time::Duration};
use tokio::{sync::mpsc, task::JoinSet, time::sleep};

const WORK_TIMEOUT_DELTA: TimeDelta = TimeDelta::seconds(10);
const WORK_FEEDBACK_INTERVAL: Duration = Duration::from_secs(5);

pub async fn run_detached(amount: NonZeroUsize, client: KvClient, node_name: String) {
    let mut join_set = JoinSet::new();

    let (work_sender, work_receiver) = async_channel::bounded(1);
    let (work_request_sender, mut work_request_receiver) = mpsc::channel(amount.get());

    for _ in 0..amount.get() {
        let task_receiver: async_channel::Receiver<(Task, Revision)> = work_receiver.clone();
        let work_request_sender: mpsc::Sender<()> = work_request_sender.clone();
        let mut client = client.clone();
        join_set.spawn(async move {
            loop {
                work_request_sender.send(()).await.unwrap();
                let (task, revision) = task_receiver.recv().await.unwrap();
                let task_id = task.key.id;
                if let Err(error) = work_on_task(&mut client, task, revision).await {
                    tracing::error!("Failed to work on task {task_id}: {error:?}")
                }
            }
        });
    }

    let mut find_work_client = client.clone();
    let mut requeue_tasks_client = client;

    join_set.spawn(async move {
        work_request_receiver.recv().await.unwrap();
        loop {
            match find_work(&mut find_work_client, node_name.clone()).await {
                Err(error) => {
                    tracing::error!("Failed to find work: {error:?}");
                    sleep(Duration::from_secs(5)).await;
                }
                Ok(None) => sleep(Duration::from_millis(500)).await,
                Ok(Some(work)) => {
                    work_sender.send(work).await.unwrap();
                    work_request_receiver.recv().await.unwrap();
                }
            };
        }
    });

    join_set.spawn(async move {
        loop {
            match requeue_tasks(&mut requeue_tasks_client).await {
                Err(error) => {
                    tracing::error!("Failed to requeue tasks: {error:?}");
                    sleep(Duration::from_secs(5)).await;
                }
                Ok(queue_empty) => {
                    if queue_empty {
                        sleep(Duration::from_millis(500)).await
                    }
                }
            };
        }
    });

    join_set.detach_all();
}

async fn work_on_task(
    client: &mut KvClient,
    mut task: Task,
    mut revision: Revision,
) -> anyhow::Result<()> {
    let task_id = task.key.id;

    while !task.value.remaining.is_zero() {
        let work_duration = if task.value.remaining >= WORK_FEEDBACK_INTERVAL {
            WORK_FEEDBACK_INTERVAL
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
        (task, revision) = match progress_task(client, task, revision, work_duration).await {
            Err(MaybeRevisionError::Other(error)) => return Err(error),
            Err(MaybeRevisionError::BadRevision) => {
                tracing::info!("Could not progress task {task_id}, it was modified by someone else (either requeued or canceled)");
                return Ok(());
            }
            Ok(update) => update,
        };
    }

    match finish_task(client, &task.key ,revision).await {
        Err(MaybeRevisionError::Other(error)) => return Err(error),
        Err(MaybeRevisionError::BadRevision) => tracing::info!(
            "Could not finish task {}, it was modified by someone else (either requeued or canceled)",
            task.key.id
        ),
        Ok(_) => {
            let time_taken = (Utc::now() - task.value.created_at).to_std()?;
            tracing::info!(
                "Finished task {} after {}.{:03}s",
                task.key.id,
                time_taken.as_secs(),
                time_taken.subsec_millis()
            );
        }
    };

    Ok(())
}

async fn find_work(
    client: &mut KvClient,
    node_name: String,
) -> anyhow::Result<Option<(Task, Revision)>> {
    let priorities = list_priorities(client, 10).await?;

    for priority in priorities.into_iter() {
        tracing::info!("Trying to find matching task for priority {}", priority.id);
        let Some((task, revision)) = find_task(client, priority.id).await? else {
            tracing::warn!("Could not find task matching priorty {}", priority.id);
            continue;
        };

        if task.key.state != TaskState::Queued {
            tracing::info!(
                "Could not acquire task {}, it was modified by someone else",
                priority.id
            );
            continue;
        }

        tracing::info!("Trying to acquire task {}", priority.id);
        match acquire_task(client, task, revision, node_name.clone()).await {
            Err(MaybeRevisionError::BadRevision) => tracing::info!(
                "Could not acquire task {}, it was modified by someone else",
                priority.id
            ),
            Err(MaybeRevisionError::Other(error)) => {
                tracing::error!("Failed to acquire task {}: {:?}", priority.id, error)
            }
            Ok((task, revision)) => {
                tracing::info!("Acquired task {}", task.key.id);
                return Ok(Some((task, revision)));
            }
        }
    }

    Ok(None)
}

async fn requeue_tasks(client: &mut KvClient) -> anyhow::Result<bool> {
    let now = Utc::now();

    let tasks = list_tasks(client, Some(TaskState::Running), 10).await?;
    if tasks.is_empty() {
        return Ok(true);
    }

    for (task, revision) in tasks {
        if now - task.value.last_update < WORK_TIMEOUT_DELTA {
            continue;
        }

        let task_id = task.key.id;
        tracing::info!("Trying to requeue task {task_id}");
        match requeue_task(client, task, revision).await {
            Err(MaybeRevisionError::BadRevision) => {
                tracing::info!("Could not requeue task {task_id}, it was modified by someone else")
            }
            Err(MaybeRevisionError::Other(error)) => {
                tracing::error!("Failed to requeue task {task_id}: {error:?}")
            }
            Ok(_) => tracing::info!("Re-queued task {task_id}"),
        }
    }

    Ok(false)
}
