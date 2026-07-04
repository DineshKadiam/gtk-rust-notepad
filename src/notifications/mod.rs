use std::sync::RwLock;

use bus::{Bus, BusReader};
use lazy_static::lazy_static;

pub mod data;

lazy_static! {
    static ref BUS: RwLock<Bus<data::Event>> = RwLock::new(Bus::new(8));
}

pub fn push(notif: data::Notification) {
    BUS.write().unwrap().broadcast(data::Event::Push(notif));
}

pub fn subscribe() -> BusReader<data::Event> {
    BUS.write().unwrap().add_rx()
}
