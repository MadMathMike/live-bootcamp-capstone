use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::client;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;

async fn forward(
    mut request: Request<hyper::body::Incoming>,
    target_host: String,
    client: Client<HttpConnector, hyper::body::Incoming>,
) -> Result<Response<hyper::body::Incoming>, Infallible> {
    let uri_string = format!(
        "http://{}{}",
        target_host,
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

    // Forward the request and return the response
    match client.request(request_forwarded).await {
        Ok(res) => Ok(res),
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
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    let connector = HttpConnector::new();
    let executor = TokioExecutor::new();
    let client = client::legacy::Builder::new(executor).build(connector);

    let mut pool = Pool::new(vec![
        "localhost:3001".to_owned(),
        "localhost:3002".to_owned(),
        "localhost:3003".to_owned(),
    ]);

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let client = client.clone();

        let host = pool.next().unwrap();

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
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

pub struct Pool {
    round_robin_counter: usize,
    hosts: Vec<String>,
}

impl Pool {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            round_robin_counter: 0,
            hosts,
        }
    }
}

impl Iterator for Pool {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.round_robin_counter += 1;
        let host_index = self.round_robin_counter % self.hosts.len();

        if self.hosts.len() > 0 {
            Some(self.hosts.get(host_index).unwrap().clone())
        } else {
            None
        }
    }
}
