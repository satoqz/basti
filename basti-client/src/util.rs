use std::{collections::HashSet, hash::Hash};

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
