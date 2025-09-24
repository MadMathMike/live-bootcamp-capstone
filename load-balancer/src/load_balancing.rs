use std::net::SocketAddr;
use std::sync::Arc;
use std::usize;

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

#[derive(Debug, Clone, Copy, Default)]
pub enum LoadBalancingAlgorithm {
    #[default]
    RoundRobin,
    LeastConnections,
}

#[derive(Default)]
pub struct Pool {
    round_robin_counter: usize,
    hosts: Vec<Arc<Host>>,
    algorithm: LoadBalancingAlgorithm,
}

impl Pool {
    pub fn determine_algorithm(&mut self) -> LoadBalancingAlgorithm {
        // This is a low number to make testing easier
        // TODO: make this configurable via Pool::new
        const CUTOFF: usize = 10;

        self.algorithm = if self
            .hosts
            .iter()
            .any(|host| Host::count_connections(&host) > CUTOFF)
        {
            LoadBalancingAlgorithm::LeastConnections
        } else {
            LoadBalancingAlgorithm::RoundRobin
        };

        self.algorithm
    }

    pub fn next_host(&mut self) -> Option<Arc<Host>> {
        if self.hosts.len() == 0 {
            return None;
        }

        let host = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // TODO: uh oh! Without eventually resetting the counter back to zero,
                // I think we're in for a panic from int overflow error
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

    pub fn add_hosts(&mut self, addresses: Vec<&SocketAddr>) {
        for addr in addresses {
            if !self.hosts.iter().any(|h| h.address == *addr) {
                self.hosts.push(Arc::new(Host::new((*addr).clone())));
            }
        }
    }

    pub fn remove_hosts(&mut self, addresses: Vec<&SocketAddr>) {
        self.hosts.retain(|h| !addresses.contains(&&h.address));
    }
}
