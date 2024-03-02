use etcd_client::Client;
use std::time::Duration;
use tokio::{task::JoinSet, time::sleep};

pub async fn run(amount: usize, etcd: Client) {
    let mut join_set = JoinSet::new();

    for id in 0..amount {
        join_set.spawn(worker(id, etcd.clone()));
    }

    join_set.detach_all();
}

async fn worker(id: usize, etcd: Client) {
    loop {
        eprintln!("Hello from worker {id}");
        sleep(Duration::from_secs(5)).await;
    }
}
