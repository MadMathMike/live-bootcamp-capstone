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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum LoadBalancingAlgorithm {
    #[default]
    RoundRobin,
    LeastConnections,
}

#[derive(Default)]
pub struct Pool {
    concurrent_connections_cutover: usize,
    next_host_index: usize,
    hosts: Vec<Arc<Host>>,
    algorithm: LoadBalancingAlgorithm,
}

impl Pool {
    pub fn new(concurrent_connections_cutover: usize) -> Self {
        let mut pool = Self::default();
        pool.concurrent_connections_cutover = concurrent_connections_cutover;
        pool
    }

    pub fn determine_algorithm(&mut self) -> LoadBalancingAlgorithm {
        self.algorithm = if self
            .hosts
            .iter()
            .any(|host| Host::count_connections(&host) > self.concurrent_connections_cutover)
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
                self.next_host_index = (self.next_host_index + 1) % self.hosts.len();
                self.hosts.get(self.next_host_index)
            }
            LoadBalancingAlgorithm::LeastConnections => self.hosts.iter().min_by(|host1, host2| {
                Host::count_connections(host1).cmp(&Host::count_connections(host2))
            }),
        };

        Some(host.unwrap().clone())
    }

    pub fn add_hosts(&mut self, addresses: Vec<SocketAddr>) {
        for addr in addresses {
            if !self.hosts.iter().any(|h| h.address == addr) {
                self.hosts.push(Arc::new(Host::new(addr)));
            }
        }
    }

    pub fn remove_hosts(&mut self, addresses: Vec<SocketAddr>) {
        self.hosts.retain(|h| !addresses.contains(&h.address));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    fn make_localhost_addr(port: u16) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
    }

    #[test]
    fn test_host_count_connections() {
        let addr = make_localhost_addr(8080);
        let host = Arc::new(Host::new(addr));

        // Initially, strong count should be 1 (the original arc)
        assert_eq!(Host::count_connections(&host), 1);

        // Clone the host
        let host2 = host.clone();
        assert_eq!(Host::count_connections(&host), 2);
        assert_eq!(Host::count_connections(&host2), 2);

        // Drop one clone
        drop(host2);
        assert_eq!(Host::count_connections(&host), 1);
    }

    #[test]
    fn test_pool_new() {
        let pool = Pool::new(10);
        assert_eq!(pool.concurrent_connections_cutover, 10);
        assert_eq!(pool.algorithm, LoadBalancingAlgorithm::RoundRobin);
    }

    #[test]
    fn test_pool_add_hosts() {
        let mut pool = Pool::new(10);
        let addr1 = make_localhost_addr(8080);
        let addr2 = make_localhost_addr(8081);

        pool.add_hosts(vec![addr1, addr2]);

        assert_eq!(pool.hosts.len(), 2);
        assert!(pool.hosts.iter().any(|h| h.address == addr1));
        assert!(pool.hosts.iter().any(|h| h.address == addr2));

        // Adding duplicate should not increase size
        pool.add_hosts(vec![addr1]);
        assert_eq!(pool.hosts.len(), 2);
    }

    #[test]
    fn test_pool_remove_hosts() {
        let mut pool = Pool::new(10);
        let addr1 = make_localhost_addr(8080);
        let addr2 = make_localhost_addr(8081);

        pool.add_hosts(vec![addr1, addr2]);
        assert_eq!(pool.hosts.len(), 2);

        pool.remove_hosts(vec![addr1]);
        assert_eq!(pool.hosts.len(), 1);
        assert_eq!(pool.hosts[0].address, addr2);
    }

    #[test]
    fn test_pool_determine_algorithm_defaults_to_round_robin() {
        let mut pool = Pool::new(10);
        let addr = make_localhost_addr(8080);
        pool.add_hosts(vec![addr]);

        assert_eq!(
            pool.determine_algorithm(),
            LoadBalancingAlgorithm::RoundRobin
        );
    }

    #[test]
    fn test_pool_determine_algorithm_switches_to_least_connections() {
        let mut pool = Pool::new(1);
        let addr = make_localhost_addr(8080);
        pool.add_hosts(vec![addr]);

        // Simulate an open connection by getting a host
        let _clone1 = pool.next_host();

        assert_eq!(
            pool.determine_algorithm(),
            LoadBalancingAlgorithm::LeastConnections
        );
    }

    #[test]
    fn test_pool_next_host_round_robin() {
        let mut pool = Pool::new(10);
        let addr1 = make_localhost_addr(8080);
        let addr2 = make_localhost_addr(8081);

        pool.add_hosts(vec![addr1, addr2]);

        let host1 = pool.next_host().unwrap();
        let host2 = pool.next_host().unwrap();

        assert_ne!(host1.address, host2.address);

        // Should cycle back
        let host3 = pool.next_host().unwrap();
        assert_eq!(host1.address, host3.address);
    }

    #[test]
    fn test_pool_next_host_least_connections() {
        let mut pool = Pool::new(1);
        let addr1 = make_localhost_addr(8080);
        let addr2 = make_localhost_addr(8081);

        pool.add_hosts(vec![addr1, addr2]);

        // Simulate higher connection count on first host
        let _host1 = pool.next_host().unwrap();
        let host2 = pool.next_host().unwrap();

        let host2_addr = host2.address.clone();
        drop(host2);
        pool.next_host().unwrap();

        assert_eq!(
            pool.determine_algorithm(),
            LoadBalancingAlgorithm::LeastConnections
        );

        let next_host = pool.next_host().unwrap();

        // Least connection host should return the host without an "open connection"
        assert_eq!(next_host.address, host2_addr);
    }

    #[test]
    fn test_pool_next_host_empty() {
        let mut pool = Pool::new(10);
        assert!(pool.next_host().is_none());
    }
}
