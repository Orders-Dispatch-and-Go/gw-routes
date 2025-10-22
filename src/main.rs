use gw_routes::config::{Config, REQUIRED_VARIABLES};
use gw_routes::db::Database;
use gw_routes::schema::SCHEMA;

#[tokio::main]
async fn main() {
    env_logger::init();

    if let Err(e) = run().await {
        log::error!("FATAL: {e}");
    }
}

async fn run() -> anyhow::Result<()> {
    let config = Config::env().inspect_err(|e| {
        log::error!(
            "config: {e}. Check all required environment variables ({}) are set.",
            REQUIRED_VARIABLES.join(", ")
        );
    })?;

    config.log();

    let database = Database::connect(&config.pg_url).await?;
    sqlx::query(SCHEMA).execute(&database.pool).await?;
    log::info!("Connected to database ({})", config.pg_url);

    let client = gw_routes::api::map_service::client::Client::new(&config.map_service_addr)?;
    let state = gw_routes::api::service::State::new(database, client);

    let listen_addr = format!("0.0.0.0:{}", config.listen_port);
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;

    let router = gw_routes::api::service::router::router(state);

    log::info!("Serving on {listen_addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
