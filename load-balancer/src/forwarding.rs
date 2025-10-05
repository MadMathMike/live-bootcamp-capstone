use std::sync::Arc;
use std::time::Instant;

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;

use crate::host::Host;

pub async fn forward(
    mut request: Request<hyper::body::Incoming>,
    target_host: Option<Arc<Host>>,
    client: Client<HttpConnector, hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    if target_host.is_none() {
        return Ok(service_unavailable_response());
    }

    let target_host = target_host.unwrap();

    let uri_string = format!(
        "http://{}{}",
        target_host.address,
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

    let request_forwarded = builder.body(request.into_body()).unwrap();

    let start = Instant::now();

    // Forward the request and return the response
    match client.request(request_forwarded).await {
        Ok(res) => {
            target_host
                .response_times
                .write()
                .await
                .push_back(start.elapsed().as_millis());

            let (parts, body) = res.into_parts();

            Ok(Response::from_parts(parts, body.boxed()))
        }
        Err(err) => {
            eprintln!("Error forwarding request {:?}", err);

            // TODO: Also consider tracking error against host.
            // Not sure what types of errors in making the request
            // should tell us that the host itself is bad though.

            Ok(service_unavailable_response())
        }
    }
}

fn service_unavailable_response() -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = Full::new(Bytes::new()).map_err(|e| match e {}).boxed();

    let res = Response::builder()
        .status(hyper::StatusCode::SERVICE_UNAVAILABLE)
        .body(body)
        .unwrap();

    res
}
