pub mod device_client;
pub mod discovery;
pub mod media_renderer;
pub mod media_server;
pub mod parser;
pub mod types;

use std::sync::{mpsc::Sender, Mutex};

use lazy_static::lazy_static;
use types::Event;

lazy_static! {
    static ref BROADCAST_EVENT: Mutex<Option<Sender<Event>>> = Mutex::new(None);
}
