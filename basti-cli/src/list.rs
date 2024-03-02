use crate::client::BastiClient;
use basti_common::task::TaskState;
use clap::Args;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Task state to filter by
    #[clap(long, required = false)]
    filter: Option<TaskState>,
}

pub async fn list_command(args: ListArgs, client: BastiClient) {
    let tasks = client.list(args.filter).await.unwrap();
    dbg!(tasks);
}
