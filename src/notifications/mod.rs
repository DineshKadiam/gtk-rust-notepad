use std::sync::Arc;

use lazy_static::lazy_static;

use async_bus::AsyncBus;
use tokio::runtime::Runtime;

pub mod data;

pub type Receiver<T> = async_bus::AsyncBusReceiver<T>;

lazy_static! {
    static ref BUS: Arc<AsyncBus<data::Event>> = Arc::new(AsyncBus::unbounded());
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
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

pub async fn subscribe() -> Receiver<data::Event> {
    BUS.clone().new_rx().await
}
