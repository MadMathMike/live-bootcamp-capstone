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

    pub fn count_connections(host: &Arc<Host>) -> usize {
        // This reference count doesn't actually contain the number of active connections
        // to the host, but it correlates, which is good enough for the purposes of the
        // "least connections" algorithm.
        //
        // How it works:
        // As the host is cloned for each new request, the internal counter of the Arc
        // will increment. And when the request completes, the arc clone is dropped,
        // which decrements the strong count.
        Arc::strong_count(host)
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
            LoadBalancingAlgorithm::LeastConnections => self
                .hosts
                .iter()
                .min_by(|host1, host2| {
                    Host::count_connections(host1).cmp(&Host::count_connections(host2))
                })
                .unwrap()
                .clone(),
        };

        Some(host)
    }
}
