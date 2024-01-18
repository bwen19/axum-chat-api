use server::{make_app, Config};
use std::net::SocketAddr;
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Read environment variables from '.env'
    dotenvy::dotenv().expect("failed to read .env file");

    // Initialize the logger
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=debug,sqlx::query=error,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse our configuration from the environment
    let config = Config::from_env();

    let addr: SocketAddr = config
        .server_addr
        .parse()
        .expect("Can not parse server address");
    tracing::info!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    let app = make_app(config).await;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("failed to start server");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}
