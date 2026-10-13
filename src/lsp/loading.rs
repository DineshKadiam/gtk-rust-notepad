use std::sync::atomic::{AtomicUsize, Ordering};

use async_bus::{Bus, Receiver};
use lazy_static::lazy_static;

use crate::RUNTIME;

/// AtomicUsize that counts how many files are currently being indexed/loaded.
static COUNT: AtomicUsize = AtomicUsize::new(0);
lazy_static! {
    static ref BUS: Bus<usize> = Bus::new(1);
}

/// Returns true if there are files loading.
pub fn is_loading() -> bool {
    COUNT.load(Ordering::Relaxed) > 0
}

/// New listener on if INDEXING_COUNT changes
pub fn listen() -> Receiver<usize> {
    BUS.new_rx()
}

fn notify(amt: usize) {
    RUNTIME.spawn(async move { BUS.queue(amt).await });
}

/// Mutate INDEXING_COUNT
pub fn add(amt: usize) {
    let prev = COUNT.fetch_add(amt, Ordering::Relaxed);
    notify(prev + amt);
}

/// Mutate INDEXING_COUNT
pub fn sub(amt: usize) {
    let prev = COUNT.fetch_sub(amt, Ordering::Relaxed);
    notify(prev - amt);
}
