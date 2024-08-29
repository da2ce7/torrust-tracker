use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) trait GenNextKey {
    type Key;

    fn next_id(&self) -> Option<Self::Key>;
}

#[derive(Default)]
pub(super) struct KeyGenerator {
    counter: AtomicUsize,
}

impl GenNextKey for KeyGenerator {
    type Key = usize;

    fn next_id(&self) -> Option<Self::Key> {
        Some(self.counter.fetch_add(1, Ordering::SeqCst))
    }
}
