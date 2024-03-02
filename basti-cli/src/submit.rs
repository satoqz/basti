use std::time::Duration;

use crate::client::BastiClient;
use clap::Args;

#[derive(Debug, Args)]
pub struct SubmitArgs {
    /// Task priority, higher = higher priority, 0 = lowest priority
    #[clap(long, default_value = "0")]
    priority: u32,
    /// Task duration in seconds
    #[clap(long, default_value = "0")]
    seconds: u64,
    /// Additional task duration in milliseconds
    #[clap(long, default_value = "0")]
    millis: u64,
}

pub async fn submit_command(args: SubmitArgs, client: BastiClient) {
    let task = client
        .submit(
            Duration::from_secs(args.seconds) + Duration::from_millis(args.millis),
            args.priority,
        )
        .await
        .unwrap();
    dbg!(task);
}
