use serde::{Deserialize, Serialize};

pub mod map_service;
pub mod service;

#[derive(Serialize, Deserialize)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Station {
    pub id: i64,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
}
