use std::env::VarError;

use anyhow::anyhow;

const ENV_POSTGRES_URL: &str = "PG_URL";
const ENV_LISTEN_PORT: &str = "LISTEN_PORT";
const ENV_MAP_SERVICE_ADDR: &str = "MAP_SERVICE_ADDR";

pub const REQUIRED_VARIABLES: [&str; 2] = [ENV_POSTGRES_URL, ENV_MAP_SERVICE_ADDR];

const DEFAULT_LISTEN_PORT: u16 = 9616;

pub struct Config {
    pub pg_url: String,
    pub listen_port: u16,
    pub map_service_addr: String,
}

impl Config {
    pub fn env() -> anyhow::Result<Self> {
        let postgres_url = env("PG_URL")?;
        let map_service_addr = env("MAP_SERVICE_ADDR")?;
        let listen_port = env("LISTEN_PORT")
            .and_then(|v| v.parse().map_err(Into::into))
            .unwrap_or(DEFAULT_LISTEN_PORT);

        Ok(Self {
            pg_url: postgres_url,
            listen_port,
            map_service_addr,
        })
    }

    pub fn log(&self) {
        log::info!("CONFIG:");
        log::info!("POSTGRES URL:        {}", self.pg_url);
        log::info!("LISTEN PORT:         {}", self.listen_port);
        log::info!("MAP SERVICE ADDRESS: {}", self.map_service_addr);
    }
}

fn env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|e| match e {
        VarError::NotPresent => anyhow!("{name} not set"),
        VarError::NotUnicode(_) => anyhow!("{name} value is not valid unicode"),
    })
}
