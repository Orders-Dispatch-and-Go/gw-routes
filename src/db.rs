#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::PgPool,
}

impl Database {
    pub async fn connect(pg_url: &str) -> anyhow::Result<Self> {
        let pool = sqlx::PgPool::connect(pg_url).await?;

        Ok(Database { pool })
    }
}
