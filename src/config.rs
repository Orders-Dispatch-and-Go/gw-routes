use std::env::VarError;

use anyhow::anyhow;

pub struct Config {
    pub postgres_url: String,
}

impl Config {
    pub fn env() -> anyhow::Result<Self> {
        let postgres_url = env("PG_URL")?;

        Ok(Self {
            postgres_url
        })
    }
}

fn env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|e| match e {
        VarError::NotPresent => anyhow!("{name} not set"),
        VarError::NotUnicode(_) => anyhow!("{name} value is not valid unicode"),
    })
}
