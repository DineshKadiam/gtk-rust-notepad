use async_broadcast::{Receiver, Sender};
use lazy_static::lazy_static;

use tokio::runtime::Runtime;

pub mod data;

macro_rules! bus {
    () => {{
        let (tx, mut rx) = async_broadcast::broadcast(128);
        // Put receiver in endless hell so the channel never dies
        tokio::runtime::Runtime::new().unwrap().spawn(async move {
            loop {
                let _ = rx.recv().await;
            }
        });
        // (Customise tx here)
        tx
    }};
}

lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
    static ref BUS: Sender<data::Event> = bus!();
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
