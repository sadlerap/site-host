use axum::{extract::Path, response::IntoResponse, routing::get, Router, Server};
use std::{error::Error, net::SocketAddr};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    run_server().await
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let app = Router::new()
        .route("/*path", get(serve_path));

    println!("Listening on http://{}", addr);

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

async fn serve_path(Path(path): Path<String>) -> impl IntoResponse {
    format!("Hello world!  Wanting to fetch {}\n", path)
}
