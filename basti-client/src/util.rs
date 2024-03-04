use std::{collections::HashSet, env, hash::Hash, os::unix::process::CommandExt, process::Command};

pub fn reexec_with_watch(interval: f32) -> anyhow::Result<()> {
    let args = ["--color", "--no-rerun", "--no-title", "--no-wrap"]
        .into_iter()
        .map(String::from)
        .chain([format!("--interval={interval}"), "--exec".into()].into_iter())
        .chain(env::args().filter(|arg| arg != "--watch"));

    Err(Command::new("watch").args(args).exec().into())
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
