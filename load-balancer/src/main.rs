use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use std::usize;

use circular_buffer::CircularBuffer;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::client;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

async fn forward(
    mut request: Request<hyper::body::Incoming>,
    target_host: Arc<Host>,
    client: Client<HttpConnector, hyper::body::Incoming>,
) -> Result<Response<hyper::body::Incoming>, Infallible> {
    let uri_string = format!(
        "http://{}{}",
        target_host.connection,
        request
            .uri()
            .path_and_query()
            .map(|x| x.as_str())
            .unwrap_or("")
    );

    request.headers_mut().remove("host");

    let mut builder = Request::builder().uri(uri_string).method(request.method());

    for (key, value) in request.headers().iter() {
        builder = builder.header(key, value);
    }

    let request_forwarded = builder.body(request.into_body()).unwrap(); // TODO

    let start = Instant::now();

    // Forward the request and return the response
    match client.request(request_forwarded).await {
        Ok(res) => {
            target_host
                .response_times
                .write()
                .await
                .push_back(start.elapsed().as_millis());
            Ok(res)
        }
        Err(err) => {
            todo!()
            // eprintln!("Request failed: {:?}", err);
            // let mut res = Response::new(Full::new(Bytes::from("Internal Server Error")));
            // *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
            // Ok(res)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = Arc::new(tokio::sync::RwLock::new(Pool::new(vec![
        "localhost:3002".to_owned(),
        "localhost:3001".to_owned(),
        "localhost:3003".to_owned(),
    ])));

    let config_addr = SocketAddr::from(([127, 0, 0, 1], 2999));
    let config_listener = TcpListener::bind(config_addr).await?;
    let pool_clone = pool.clone();
    tokio::task::spawn(async move {
        // We start a loop to continuously accept incoming connections
        loop {
            let (_, _) = config_listener.accept().await.unwrap();
            let mut pool = pool_clone.write().await;
            pool.cycle_algorithm();
            for host in pool.hosts.iter() {
                let response_times = host.response_times.read().await;
                let avg = if response_times.len() > 0 {
                    Some(response_times.iter().sum::<u128>() / response_times.len() as u128)
                } else {
                    None
                };
                println!("{} response time average {avg:?}", host.connection);
            }
            println!("{:?}", pool.algorithm);
        }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    let client = client::legacy::Builder::new(TokioExecutor::new()).build(HttpConnector::new());

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let client = client.clone();
        let host = pool.write().await.next().unwrap();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| forward(req, host.clone(), client.clone())),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

#[derive(Clone)]
pub struct Host {
    connection: String,
    response_times: Arc<RwLock<CircularBuffer<100, u128>>>,
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
    hosts: Vec<Arc<Host>>,
    algorithm: LoadBalancingAlgorithm,
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
