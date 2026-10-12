use async_bus::{Bus, Receiver};
use lazy_static::lazy_static;

use tokio::runtime::Runtime;

pub mod data;

lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
    static ref BUS: Bus<data::Event> = Bus::new(128);
}

pub fn push(notif: data::Notification) {
    RUNTIME.spawn(async move {
        BUS.queue(data::Event::Push(notif)).await;
    });
}

pub fn pop(id: String) {
    RUNTIME.spawn(async move {
        BUS.queue(data::Event::Pop(id)).await;
    });
}

pub fn subscribe() -> Receiver<data::Event> {
    BUS.new_rx()
}
