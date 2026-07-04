use std::sync::RwLock;

use async_channel::{Receiver, Sender};

pub struct Bus<T>(RwLock<Vec<Sender<T>>>);

impl<T: Clone> Bus<T> {
    pub const fn new() -> Self {
        Bus(RwLock::new(vec![]))
    }

    pub fn add_rx(&self) -> Receiver<T> {
        let (tx, rx) = async_channel::bounded(1);
        self.0.write().unwrap().push(tx);
        rx
    }

    pub fn broadcast(&self, val: T) {
        self.0.read().unwrap().iter().for_each(|sender| {
            sender.send_blocking(val.clone()).unwrap();
        });
    }
}
