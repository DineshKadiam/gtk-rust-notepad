use async_channel::Receiver;

use crate::async_bus::Bus;

pub mod data;

static BUS: Bus<data::Event> = Bus::new();

pub fn push(notif: data::Notification) {
    BUS.broadcast(data::Event::Push(notif));
}

pub fn subscribe() -> Receiver<data::Event> {
    BUS.add_rx()
}
