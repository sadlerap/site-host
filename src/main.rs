use axum::response::{IntoResponse, Response};
use axum::{extract::Path, routing::get, Router, Server};
use hyper::StatusCode;
use std::{error::Error, net::SocketAddr};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tracing::{debug, info, warn};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    run_server().await
}

#[tracing::instrument]
async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let app = Router::new()
        .route("/*path", get(serve_path))
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()));

    info!("Listening on http://{}", addr);

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[tracing::instrument]
async fn serve_path(Path(path): Path<String>) -> Response {
    debug!(%path, "Handing request");
    let cwd = &std::env::current_dir().expect("current directory undefined");
    let cwd_path = cwd.display();
    debug!(%cwd_path, "Serving from");
    let mut real_path = cwd.clone();
    real_path.push(match path.as_str() {
        "/" => "index.html",
        _ => path.strip_prefix("/").unwrap_or(&path),
    });

    if real_path.is_dir() {
        real_path.push("index.html");
    }

    let real_path = if let Ok(p) = tokio::fs::canonicalize(&real_path).await {
        p
    } else {
        let path = real_path.display();
        warn!(%path, "Unable to canonicalize path!");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if !real_path.starts_with(cwd) {
        return (StatusCode::FORBIDDEN, "<h1>Forbidden</h1>\n").into_response();
    }

    let data = if let Ok(vec) = tokio::fs::read(dbg!(real_path)).await {
        vec
    } else {
        return StatusCode::NOT_FOUND.into_response();
    };

    data.into_response()
}
