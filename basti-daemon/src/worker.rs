use etcd_client::Client;
use std::time::Duration;
use tokio::{task::JoinSet, time::sleep};

pub async fn run(amount: usize, name: &str, etcd: Client) {
    let mut join_set = JoinSet::new();

    for id in 1..=amount {
        join_set.spawn(worker(format!("{name}-{id}"), etcd.clone()));
    }

    join_set.detach_all();
}

async fn worker(name: String, etcd: Client) {
    loop {
        eprintln!("Hello from worker {name}");
        sleep(Duration::from_secs(5)).await;
    }
}
