mod load_balancing;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::client;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;

use crate::load_balancing::*;

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
