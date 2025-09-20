use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

use hyper::{Request, Response};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;

use crate::load_balancing::*;

pub async fn forward(
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
        Err(_err) => {
            todo!()
            // eprintln!("Request failed: {:?}", err);
            // let mut res = Response::new(Full::new(Bytes::from("Internal Server Error")));
            // *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
            // Ok(res)
        }
    }
}
