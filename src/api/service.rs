use serde::{Deserialize, Serialize};

use super::Station;

#[derive(Serialize, Deserialize)]
pub struct CreateRouteRequest {
    pub stops: Vec<[f64; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateRouteResponse {
    pub route_id: i64,
    pub graph: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetRouteRequest {
    pub route_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct GetRouteResponse {
    pub way: Vec<[f64; 2]>,
    pub stations: Vec<Station>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateMergeOptionsRequest {
    pub request_route_id: i64,
    pub trips_routes: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateMergeOptionsResponse {
    pub merger_templates: Vec<[i64; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct MergeTemplateRequest {
    pub template_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct MergeRouteRequest {
    pub trip_route_id: i64,
    pub request_routes: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct MergeResponse {
    pub route_id: i64,
    pub graph: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct RemoveStationsRequest {
    pub route_id: i64,
    pub graph: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct RemoveStationsResponse {
    pub route_id: i64,
    pub graph: Vec<i64>,
}
