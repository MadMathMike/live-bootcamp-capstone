use std::sync::Arc;
use std::usize;

use circular_buffer::CircularBuffer;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Host {
    pub connection: String,
    pub response_times: Arc<RwLock<CircularBuffer<100, u128>>>,
}

impl Host {
    pub fn new(connection: String) -> Self {
        Self {
            connection: connection,
            response_times: Arc::new(RwLock::new(CircularBuffer::new())),
        }
    }
}

#[derive(Debug)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
}

pub struct Pool {
    round_robin_counter: usize,
    pub hosts: Vec<Arc<Host>>,
    pub algorithm: LoadBalancingAlgorithm,
}

impl Pool {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            round_robin_counter: 0,
            hosts: hosts
                .into_iter()
                .map(|host| Arc::new(Host::new(host)))
                .collect(),
            algorithm: LoadBalancingAlgorithm::RoundRobin,
        }
    }

    pub fn cycle_algorithm(&mut self) {
        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                self.algorithm = LoadBalancingAlgorithm::LeastConnections
            }
            LoadBalancingAlgorithm::LeastConnections => {
                self.algorithm = LoadBalancingAlgorithm::RoundRobin
            }
        }
    }
}

impl Iterator for Pool {
    type Item = Arc<Host>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.hosts.len() == 0 {
            return None;
        }

        let host = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // Round robin
                self.round_robin_counter += 1;
                let index = self.round_robin_counter % self.hosts.len();
                self.hosts.get(index).unwrap().clone()
            }
            LoadBalancingAlgorithm::LeastConnections => {
                let mut least_connections_host: Option<Arc<Host>> = None;
                let mut least_connection_count = usize::MAX;
                for host in self.hosts.iter() {
                    // This reference count doesn't actually contain the number of active connections
                    // to the host, but it correlates. As the host is cloned for each new request,
                    // the internal counter of the Arc will increment. And when the request completes,
                    // the clone is dropped, which decrements.
                    let connection_count = Arc::strong_count(host);

                    if connection_count < least_connection_count {
                        least_connection_count = connection_count;
                        least_connections_host = Some(host.clone());
                    }
                }

                least_connections_host.unwrap()
            }
        };

        Some(host)
    }
}
