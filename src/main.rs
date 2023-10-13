use std::collections::HashMap;
use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use http_body_util::combinators::BoxBody;
use http_body_util::{Full, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rand::rngs::OsRng;
use rand::Rng;
use serde::Serialize;
use tokio::net::TcpListener;

fn config(_: Request<hyper::body::Incoming>) -> Result<Response<BoxBodyT>> {
    let urls = HashMap::from([
        ("small_download_url", "http://localhost:3000/api/v1/small"),
        ("large_download_url", "http://localhost:3000/api/v1/large"),
        ("upload_url", "http://localhost:3000/api/v1/upload"),
    ]);
    let config = Config { version: 1, urls };
    let json = serde_json::to_string(&config).expect("Unable to serialize");
    Ok(Response::new(full(Bytes::from(json))))
}

fn small(_: Request<Incoming>) -> Result<Response<BoxBodyT>> {
    let mut rng = rand::thread_rng();
    let one_byte = Vec::from([rng.gen()]);
    Ok(Response::new(full(one_byte)))
}

async fn large(_: Request<Incoming>) -> Result<Response<BoxBodyT>> {
    // create an "infinite" stream of random bytes
    // and "take" 8GB from the stream
    let mut rng = OsRng::default();
    // each "repeat" will generate 256KB of random data
    let random_byte_stream = futures_util::stream::repeat_with(move || {
        let mut two_fifty_six_kb = [0u8; 256 * 1024];
        rng.fill(&mut two_fifty_six_kb[..]);
        Ok(Frame::data(Bytes::from(Vec::from(two_fifty_six_kb))))
    });
    let body = StreamBody::new(random_byte_stream.take(8 * 4 * 1024));
    Ok(Response::builder().body(BoxBody::new(body)).unwrap())
}

#[derive(Serialize, Debug)]
struct Config {
    version: u8,
    urls: HashMap<&'static str, &'static str>,
}

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type BoxBodyT = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

async fn nq_service(req: Request<Incoming>) -> Result<Response<BoxBodyT>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/api/v1/config") => config(req),
        (&Method::GET, "/api/v1/small") => small(req),
        (&Method::GET, "/api/v1/large") => large(req).await,
        (&Method::POST, "/api/v1/upload") => upload(req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(full(NOTFOUND))
            .unwrap()),
    }
}

async fn upload(req: Request<Incoming>) -> Result<Response<BoxBodyT>> {
    let mut drain = futures::sink::drain();
    drain.send(req.into_body()).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(full(EMPTY))
        .unwrap())
}

static EMPTY: &[u8] = b"";
static NOTFOUND: &[u8] = b"Not Found";

use http_body_util::BodyExt;
fn full<T: Into<Bytes>>(chunk: T) -> BoxBodyT {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            let service = service_fn(move |req| nq_service(req));
            // Q: If H2 uses TokioExecutor, is it still beneficial for the service to be spawned in a tokio task?
            if let Err(err) = http2::Builder::new(TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
