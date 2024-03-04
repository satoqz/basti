use anyhow::anyhow;
use std::{
    collections::HashSet, env, hash::Hash, io::ErrorKind, os::unix::process::CommandExt,
    process::Command,
};

pub fn reexec_with_watch(interval: f32) -> anyhow::Result<()> {
    let args = ["--color", "--no-rerun", "--no-title", "--no-wrap"]
        .into_iter()
        .map(String::from)
        .chain([format!("--interval={interval}"), "--exec".into()])
        .chain(env::args().filter(|arg| arg != "--watch"));

    let err = Command::new("watch").args(args).exec();
    Err(match err.kind() {
        ErrorKind::NotFound => anyhow!("`watch` utility not found in PATH"),
        _ => err.into(),
    })
}

pub trait Compact {
    fn compact(self) -> Self;
}

impl<T: Eq + Hash> Compact for Vec<T> {
    fn compact(self) -> Self {
        self.into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    }
}
