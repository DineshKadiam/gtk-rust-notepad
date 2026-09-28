use async_channel::Receiver;
use lazy_static::lazy_static;

use crate::async_bus::Bus;

pub mod data;

lazy_static! {
    static ref BUS: Bus<data::Event> = Bus::new();
}

pub fn push(notif: data::Notification) {
    BUS.broadcast(data::Event::Push(notif));
}

pub fn pop(id: String) {
    BUS.broadcast(data::Event::Pop(id));
}

pub fn subscribe() -> Receiver<data::Event> {
    BUS.add_rx()
}
