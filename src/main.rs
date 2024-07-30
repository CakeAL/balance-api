use std::{net::SocketAddr, str::FromStr};

use axum::Router;
use router::routers;

mod config;
mod handler;
mod router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load_config(); // 出现错误直接 panic!

    let app = Router::new().merge(routers());

    let addr = SocketAddr::from_str(&format!("{}:{}", config.server.addr, config.server.port))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
