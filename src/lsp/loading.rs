use std::sync::{
    RwLock,
    atomic::{AtomicUsize, Ordering},
};

use async_channel::{Receiver, Sender};

/// AtomicUsize that counts how many files are currently being indexed/loaded.
static COUNT: AtomicUsize = AtomicUsize::new(0);

static SENDERS: RwLock<Vec<Sender<()>>> = RwLock::new(vec![]);

/// Get number of files that are being indexed by any language server.
pub fn get() -> usize {
    COUNT.load(Ordering::Relaxed)
}

/// Returns true if there are files loading.
pub fn is_loading() -> bool {
    get() > 0
}

/// New listener on if INDEXING_COUNT changes
pub fn listen() -> Receiver<()> {
    let (tx, rx) = async_channel::unbounded();
    SENDERS.write().unwrap().push(tx);
    rx
}

fn notify() {
    SENDERS.read().unwrap().iter().for_each(|sender| {
        sender.send_blocking(()).unwrap();
    });
}

/// Mutate INDEXING_COUNT
pub fn add(amt: usize) {
    COUNT.fetch_add(amt, Ordering::Relaxed);
    notify();
}

/// Mutate INDEXING_COUNT
pub fn sub(amt: usize) {
    COUNT.fetch_sub(amt, Ordering::Relaxed);
    notify();
}
