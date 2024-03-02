use crate::client::Client;
use anyhow::Result;
use async_channel::{Receiver, Sender};
use basti_common::task::Task;
use std::{num::NonZeroUsize, time::Duration};
use tokio::{task::JoinSet, time::sleep};

#[tracing::instrument(skip(client), level = "info")]
pub async fn run(amount: NonZeroUsize, client: Client) -> Result<()> {
    let mut pool = Pool::spawn(amount, client.clone()).await;
    pool.detach();

    let mut client = client.new_connection().await?;

    Ok(())
}

#[derive(Debug)]
struct Pool {
    join_set: JoinSet<()>,
    sender: Sender<Task>,
}

impl Pool {
    async fn spawn(size: NonZeroUsize, client: Client) -> Self {
        let mut join_set = JoinSet::new();
        let (sender, receiver) = async_channel::bounded(1);

        for _ in 0..size.get() {
            join_set.spawn(worker(client.clone(), receiver.clone()));
        }

        Self { join_set, sender }
    }

    async fn queue(&self, task: Task) {
        self.sender.send(task).await.unwrap()
    }

    fn detach(&mut self) {
        self.join_set.detach_all();
    }
}

async fn worker(mut client: Client, receiver: Receiver<Task>) {
    while let Ok(task) = receiver.recv().await {
        dbg!(task);
        sleep(Duration::from_secs(60)).await;
    }
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
