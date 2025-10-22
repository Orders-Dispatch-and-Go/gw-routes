use serde::{Deserialize, Serialize};

use crate::api::types::Coord;

#[derive(Serialize, Deserialize)]
pub struct CreateRouteRequest {
    pub stops: Vec<[f64; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateRouteResponse {
    pub way: Vec<[f64; 2]>,
    pub graph: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct FindStationRequest {
    pub id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct FindStationResponse {
    pub address: String,

    #[serde(flatten)]
    pub coord: Coord,
}
