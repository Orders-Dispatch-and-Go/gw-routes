use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreateRouteRequest {
    pub stops: Vec<[f64; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateRouteResponse {
    pub way: Vec<[f64; 2]>,
    pub distance: f64,
    pub duration: f64,
}
