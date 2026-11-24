//! `ilvunod`: serves the runner's local door on 127.0.0.1. `ILVUNO_PORT` picks the port (default 4737).

use anyhow::{bail, Context, Result};
use api::state::ProviderBuilder;
use cli::{ops, providers};
use runner::Home;
use std::sync::Arc;
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 4737;

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
    let build: ProviderBuilder<providers::AnyProvider> = Arc::new(providers::build);
    // This opens and migrates the database, then records any lost turns before the listener exists.
    let (app, recovered) = api::startup_router(home, build)?;
    if recovered > 0 {
        println!("paused {recovered} run(s) left working by a stopped runner");
    }
    let listener = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => listener,
        Err(err) => bail!("cannot listen on 127.0.0.1:{port}: {err}"),
    };
    println!("ilvunod {} listening on http://127.0.0.1:{port}", env!("CARGO_PKG_VERSION"));
    axum::serve(listener, app).await?;
    Ok(())
}
