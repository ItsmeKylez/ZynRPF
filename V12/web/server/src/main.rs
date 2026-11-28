use std::time::Duration;
use zyn_server::{app, config::Config, store::WorkspaceStore};

#[tokio::main]
async fn main() {
    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("configuration error: {e}");
            std::process::exit(2);
        }
    };
    let port: u16 = match std::env::var("PORT") {
        Ok(p) => p.trim().parse().unwrap_or_else(|_| {
            eprintln!("configuration error: PORT must be a number between 0 and 65535, got {p:?}");
            std::process::exit(2);
        }),
        Err(_) => 8080,
    };
    let store = WorkspaceStore::new(cfg).unwrap_or_else(|e| {
        eprintln!("cannot prepare the workspace directory: {}", e.code());
        std::process::exit(2);
    });
    let state = app::AppState::new(store);
    let janitor = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            let st = janitor.clone();
            let _ = tokio::task::spawn_blocking(move || st.sweep()).await;
        }
    });
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap_or_else(|e| {
        eprintln!("cannot bind 0.0.0.0:{port}: {e}");
        std::process::exit(1);
    });
    eprintln!("zyn-server listening on 0.0.0.0:{port}");
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    if let Err(e) = axum::serve(listener, app::router(state)).with_graceful_shutdown(shutdown).await {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
