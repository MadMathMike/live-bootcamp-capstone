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

#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
}

pub struct Pool {
    round_robin_counter: usize,
    good_hosts: Vec<Arc<Host>>,
    bad_hosts: Vec<Arc<Host>>,
    algorithm: LoadBalancingAlgorithm,
}

impl Pool {
    pub fn new(hosts: Vec<SocketAddr>) -> Self {
        Self {
            round_robin_counter: 0,
            good_hosts: hosts
                .into_iter()
                .map(|address| Arc::new(Host::new(address)))
                .collect(),
            bad_hosts: Vec::default(),
            algorithm: LoadBalancingAlgorithm::RoundRobin,
        }
    }

    pub fn determine_algorithm(&mut self) -> LoadBalancingAlgorithm {
        // This is a low number to make testing easier
        // TODO: make this configurable via Pool::new
        const CUTOFF: usize = 10;

        self.algorithm = if self
            .good_hosts
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
        if self.good_hosts.len() == 0 {
            return None;
        }

        let host = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // TODO: uh oh! Without eventually resetting the counter back to zero,
                // I think we're in for a panic from int overflow error
                self.round_robin_counter += 1;
                let index = self.round_robin_counter % self.good_hosts.len();
                self.good_hosts.get(index).unwrap().clone()
            }
            LoadBalancingAlgorithm::LeastConnections => self
                .good_hosts
                .iter()
                .min_by(|host1, host2| {
                    Host::count_connections(host1).cmp(&Host::count_connections(host2))
                })
                .unwrap()
                .clone(),
        };

        Some(host)
    }

    pub fn set_good_hosts(&mut self, good_addresses: &[SocketAddr]) {
        let mut new_good_hosts: Vec<Arc<Host>> = self
            .bad_hosts
            .iter()
            .filter(|host| good_addresses.contains(&host.address))
            .map(|host| Arc::new(Host::new(host.address)))
            .collect();

        let mut new_bad_hosts: Vec<Arc<Host>> = self
            .good_hosts
            .iter()
            .filter(|host| !good_addresses.contains(&host.address))
            .map(|host| Arc::new(Host::new(host.address)))
            .collect();

        self.good_hosts
            .retain(|host| good_addresses.contains(&host.address));
        self.good_hosts.append(&mut new_good_hosts);

        self.bad_hosts
            .retain(|host| !good_addresses.contains(&host.address));
        self.bad_hosts.append(&mut new_bad_hosts);
    }
}

impl IntoIterator for &Pool {
    type Item = SocketAddr;
    type IntoIter = std::vec::IntoIter<SocketAddr>;

    /// Returns all known host addresses, including the addresses of bad hosts
    fn into_iter(self) -> Self::IntoIter {
        let good_hosts = self.good_hosts.iter().map(|host| host.address.clone());
        let bad_hosts = self.bad_hosts.iter().map(|host| host.address.clone());

        good_hosts.chain(bad_hosts).collect::<Vec<_>>().into_iter()
    }
}
