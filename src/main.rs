use std::{net::SocketAddr, str::FromStr, sync::LazyLock};

use axum::Router;
use config::Config;
use router::routers;

mod config;
mod handler;
mod router;
mod fund;
mod db;

// 出现错误直接 panic!
static GLOBAL_CONFIG: LazyLock<Config> = LazyLock::new(|| Config::load_config());

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
    .with_target(false)
    .compact()
    .init();

    let config = &*GLOBAL_CONFIG;

    let app = Router::new().merge(routers());

    let addr = SocketAddr::from_str(&format!("{}:{}", config.server.addr, config.server.port))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
