use std::sync::{OnceLock, RwLock};

use async_channel::{Receiver, Sender};
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Bus struct; contains a vec of senders and a backlog vec
pub struct Bus<T>(RwLock<Vec<Sender<T>>>, RwLock<Vec<T>>);

impl<T: Clone + Send + Sync + 'static> Bus<T> {
    pub const fn new() -> Self {
        Self(RwLock::new(vec![]), RwLock::new(vec![]))
    }

    /// Add a listener and send backlog to it
    pub fn add_rx(&self) -> Receiver<T> {
        let (tx, rx) = async_channel::bounded(1);
        self.0.write().unwrap().push(tx);
        self.send_backlog();
        rx
    }

    /// Broadcast a value to every listener. Must be `Sync + Send` (sorry aaa)
    /// in order to add to the backlog for new listeners
    pub fn broadcast(&self, val: T) {
        self.0.read().unwrap().iter().for_each(|sender| {
            sender.send_blocking(val.clone()).unwrap();
        });
        self.1.write().unwrap().push(val);
    }

    /// Sends the backlog to the last (newest) sender
    /// Abuses Rust's borrow checker. Horrible
    fn send_backlog(&self) {
        let rt = RUNTIME.get_or_init(|| Runtime::new().unwrap());
        let tx = self.0.read().unwrap().last().unwrap().clone();
        self.1.read().unwrap().iter().for_each(|val| {
            let val = val.clone();
            let tx = tx.clone();
            rt.spawn(async move {
                tx.send(val).await.unwrap();
                drop(tx);
            });
        });
    }
}
