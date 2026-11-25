//! `ilvunod`: serves the runner's local door on 127.0.0.1. `ILVUNO_PORT` picks the port (default 4737).

use anyhow::{bail, Context, Result};
use api::state::{AppState, ProviderBuilder};
use cli::{ops, providers};
use runner::{Home, Store};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 4737;
const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

fn bind_address(port: u16) -> SocketAddr {
    SocketAddr::new(LOOPBACK, port)
}

#[tokio::main]
async fn main() -> Result<()> {
    let port = match std::env::var("ILVUNO_PORT") {
        Ok(text) => text.trim().parse::<u16>().with_context(|| format!("ILVUNO_PORT is not a port number: {text:?}"))?,
        Err(_) => DEFAULT_PORT,
    };
    let home = ops::home_from_env();
    std::fs::create_dir_all(home.root()).with_context(|| format!("create {}", home.root().display()))?;
    // Absolute, so every run folder the API hands out is the same wherever ilvunod was started.
    let root = std::fs::canonicalize(home.root())?;
    let home = Home::new(root);
    // Opening once migrates the database now, so a broken home fails at start, not on the first request.
    Store::open(&home.db_path())?;

    let build: ProviderBuilder<providers::AnyProvider> = Arc::new(providers::build);
    let app = api::router(AppState::new(home, build));
    let address = bind_address(port);
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(err) => bail!("cannot listen on {address}: {err}"),
    };
    println!("ilvunod {} listening on http://{address}", env!("CARGO_PKG_VERSION"));
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bind_address;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn api_bind_address_is_loopback() {
        let listener = TcpListener::bind(bind_address(0)).await.unwrap();
        assert!(listener.local_addr().unwrap().ip().is_loopback());
    }
}
