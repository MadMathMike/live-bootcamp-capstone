use std::net::SocketAddr;
use std::sync::Arc;

use circular_buffer::CircularBuffer;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Host {
    pub address: SocketAddr,
    pub response_times: Arc<RwLock<CircularBuffer<100, u128>>>,
}

impl Host {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            response_times: Arc::new(RwLock::new(CircularBuffer::new())),
        }
    }
}
