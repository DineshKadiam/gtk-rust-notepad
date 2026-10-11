use async_broadcast::{Receiver, Sender};
use lazy_static::lazy_static;

use tokio::runtime::Runtime;

pub mod data;

lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
    static ref BUS: Sender<data::Event> = {
        let (mut tx, mut rx) = async_broadcast::broadcast(128);
        RUNTIME.spawn(async move {
            loop {
                let _ = rx.recv().await;
            }
        });
        tx.set_overflow(true);
        tx
    };
}

pub fn push(notif: data::Notification) {
    RUNTIME.spawn(async move {
        let _ = BUS.broadcast(data::Event::Push(notif)).await;
    });
}

pub fn pop(id: String) {
    RUNTIME.spawn(async move {
        let _ = BUS.broadcast(data::Event::Pop(id)).await;
    });
}

pub fn subscribe() -> Receiver<data::Event> {
    BUS.new_receiver()
}
