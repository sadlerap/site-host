use axum::http::HeaderValue;
use axum::response::{Headers, IntoResponse, Response};
use axum::{extract::Path, routing::get, Router, Server};
use hyper::header::HeaderName;
use hyper::StatusCode;
use std::io::ErrorKind;
use std::{error::Error, net::SocketAddr};
use tokio::fs::{canonicalize, read};
use tracing::{debug, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    for var in std::env::vars() {
        println!("{}={}", var.0, var.1);
    }
    let host = dbg!(std::env::var("SIMPLEST_COLLECTOR_SERVICE_HOST")?);
    let port = dbg!(std::env::var("SIMPLEST_COLLECTOR_SERVICE_PORT_HTTP_C_BINARY_TRFT")?.parse::<u16>()?);
    let endpoint = format!("http://{}:{}/api/traces", host, port);
    debug!(%endpoint, "Jaeger");
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_collector_endpoint(endpoint)
        .install_batch(opentelemetry::runtime::Tokio)?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(telemetry)
        .init();

    run_server().await?;

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}

#[tracing::instrument]
async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let app = Router::new().route("/*path", get(serve_path));


    let cwd = &std::env::current_dir().expect("current directory undefined");
    let cwd_path = cwd.display();
    debug!(%cwd_path, "Serving");

    info!("Listening on http://{}", addr);

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument]
async fn serve_path(Path(path): Path<String>) -> Response {
    info!(%path, "Handing request");
    let cwd = &std::env::current_dir().expect("current directory undefined");

    let mut real_path = cwd.clone();

    let path = path.strip_prefix('/').unwrap_or(&path);
    real_path.push(&path);
    if real_path.is_dir() {
        real_path.push("index.html");
    }

    let real_path = match canonicalize(&real_path).await {
        Ok(p) => p,
        Err(e) => {
            let path = real_path.display();
            if e.kind() == ErrorKind::NotFound {
                warn!(%path, "Requested path not found");
                return StatusCode::NOT_FOUND.into_response();
            }
            warn!(%path, "Unable to canonicalize path");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if !real_path.starts_with(cwd) {
        warn!(%path, "Client requested forbidden path");
        return (StatusCode::FORBIDDEN, "<h1>Forbidden</h1>\n").into_response();
    }

    let data = if let Ok(vec) = read(&real_path).await {
        vec
    } else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let response = data.into_response();

    // If we can't guess the mime type of the data we're sending, let the client guess it
    if let Some(mime) = mime_guess::from_path(&real_path).first() {
        if let Ok(value) = HeaderValue::from_str(mime.essence_str()) {
            (
                Headers(vec![(HeaderName::from_static("content-type"), value)]),
                response,
            )
                .into_response()
        } else {
            response
        }
    } else {
        response
    }
}
