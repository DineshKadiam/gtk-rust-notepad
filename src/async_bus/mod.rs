use std::sync::{Arc, OnceLock, RwLock};

use async_channel::{Receiver, Sender};
use tokio::{runtime::Runtime, sync::Mutex};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

macro_rules! rt {
    () => {
        RUNTIME.get_or_init(|| Runtime::new().unwrap())
    };
}

/// Bus struct; contains a vec of senders and a backlog vec
pub struct Bus<T>(Arc<Mutex<Vec<Sender<T>>>>, Arc<Mutex<Vec<T>>>);

impl<T: Clone + Send + Sync + 'static> Bus<T> {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(vec![])), Arc::new(Mutex::new(vec![])))
    }

    /// Add a listener and send backlog to it
    pub fn add_rx(&self) -> Receiver<T> {
        let (tx, rx) = async_channel::bounded(1);
        let senders = self.0.clone();
        self.send_backlog(tx.clone());
        rt!().spawn(async move {
            senders.lock().await.push(tx);
        });
        rx
    }

    /// Broadcast a value to every listener. Must be `Sync + Send`
    pub fn broadcast(&self, val: T) {
        let senders = self.0.clone();
        let events = self.1.clone();
        rt!().spawn(async move {
            events.lock().await.push(val.clone());
            for sender in senders.lock().await.iter() {
                sender.send(val.clone()).await.unwrap();
            }
        });
    }

    /// Sends the backlog to a sender
    fn send_backlog(&self, tx: Sender<T>) {
        let events = self.1.clone();
        rt!().spawn(async move {
            for event in events.lock().await.iter() {
                tx.send(event.clone()).await.unwrap();
            }
        });
    }
}
