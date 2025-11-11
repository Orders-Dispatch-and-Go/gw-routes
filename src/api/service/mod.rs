pub mod router;
pub mod types;

use crate::api::map_service;
use crate::db;

#[derive(Clone)]
pub struct State {
    pub db: db::Database,
    pub client: map_service::client::Client,
}

impl State {
    pub fn new(db: crate::db::Database, client: map_service::client::Client) -> Self {
        Self { db, client }
    }
}

impl axum::extract::FromRef<State> for map_service::client::Client {
    fn from_ref(input: &State) -> Self {
        input.client.clone()
    }
}

impl axum::extract::FromRef<State> for sqlx::PgPool {
    fn from_ref(input: &State) -> Self {
        input.db.pool.clone()
    }
}
