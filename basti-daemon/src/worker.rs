use crate::ops::{acquire_task, list_tasks};
use anyhow::Result;
use async_channel::{Receiver, Sender};
use basti_common::task::{Task, TaskState};
use etcd_client::Client;
use std::{num::NonZeroUsize, time::Duration};
use tokio::{task::JoinSet, time::sleep};

#[tracing::instrument(skip_all)]
pub async fn run(amount: NonZeroUsize, client: Client, node_name: String) {
    let mut join_set = JoinSet::new();
    let (sender, receiver) = async_channel::bounded(1);

    for _ in 0..amount.get() {
        join_set.spawn(worker(client.clone(), receiver.clone()));
    }

    join_set.detach_all();

    let feed_task = async {
        loop {
            match feed_workers(client.clone(), &sender, node_name.clone()).await {
                Ok(_) => sleep(Duration::from_secs(1)).await,
                Err(_) => {
                    tracing::warn!("Failed to feed workers, waiting 5 seconds...");
                    sleep(Duration::from_secs(5)).await;
                }
            };
        }
    };

    let requeue_task = async {
        loop {
            match requeue_tasks(client.clone()).await {
                Ok(_) => sleep(Duration::from_secs(1)).await,
                Err(_) => {
                    tracing::warn!("Failed to requeue tasks, waiting 5 seconds...");
                    sleep(Duration::from_secs(5)).await;
                }
            };
        }
    };

    tokio::join!(feed_task, requeue_task);
}

#[tracing::instrument(skip_all, err(Debug))]
async fn feed_workers(mut client: Client, sender: &Sender<Task>, node_name: String) -> Result<()> {
    'outer: loop {
        let mut tasks = list_tasks(&mut client, Some(TaskState::Queued), None).await?;

        if tasks.is_empty() {
            break;
        }

        tasks.sort_unstable_by_key(|task| task.details.priority);

        for task in tasks.iter().rev() {
            if let Ok(task) = acquire_task(&mut client, &task.key, node_name.clone()).await {
                tracing::info!("Acquired task {}.", task.key.id);
                sender.send(task).await?;
                break 'outer;
            }
        }
    }

    Ok(())
}

async fn worker(mut client: Client, receiver: Receiver<Task>) {
    while let Ok(task) = receiver.recv().await {
        const ONE_SECOND: Duration = Duration::from_secs(1);

        if task.details.duration.is_zero() {
            // client.delete(task.key.to_string(), None).await
        }
    }
}

#[tracing::instrument(skip_all, err(Debug))]
async fn requeue_tasks(mut client: Client) -> Result<()> {
    list_tasks(&mut client, None, None).await?;
    Ok(())
}

// async fn requeue_tasks(etcd: &mut Client, worker_name: &str) -> Result<()> {
//     let now = Utc::now();

//     let tasks = fetch_tasks(
//         etcd,
//         Some(TaskState::Running),
//         GetOptions::default().with_sort(SortTarget::Mod, SortOrder::Ascend),
//     )
//     .await?;

//     if tasks.is_empty() {
//         return Ok(());
//     }

//     for task in tasks {
//         let time_diff = now - task.details.last_update;
//         if time_diff < TimeDelta::seconds(10) {
//             continue;
//         }

//         match set_task_queued(etcd, &task).await {
//             Ok(_) => eprintln!(
//                 "INFO: {} set running task {} back to queued, no update in {}s",
//                 worker_name,
//                 task.key.id,
//                 time_diff.num_seconds()
//             ),
//             Err(error) => {
//                 eprintln!(
//                     "WARN: {} failed to set running task {} back to queued, no update in {}s",
//                     worker_name,
//                     task.key.id,
//                     time_diff.num_seconds()
//                 );
//                 error.log();
//             }
//         }
//     }

//     Ok(())
// }
