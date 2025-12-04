use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Coords {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Station {
    pub id: uuid::Uuid,
    pub address: String,
    pub coords: Coords,
}

#[derive(Serialize, Deserialize)]
pub struct GetStationRequest {
    pub id: uuid::Uuid,
}

pub struct GetStationResponse {
    pub station: Station,
}

#[derive(Serialize, Deserialize)]
pub struct CreateRouteRequest {
    #[serde(rename = "fromStation")]
    pub from_station: Station,

    #[serde(rename = "toStation")]
    pub to_station: Station,
}

#[derive(Serialize, Deserialize)]
pub struct CreateRouteResponse {
    pub id: uuid::Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct GetWaypointsRequest {
    pub id: uuid::Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct Waypoint {
    pub station: Station,
    pub distance: u64,

    #[serde(rename = "tripTime")]
    pub trip_time: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetWaypointsResponse {
    pub stations: Vec<Waypoint>,
}

#[derive(Serialize, Deserialize)]
pub struct GetPointsRequest {
    pub id: uuid::Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct GetPointsResponse {
    pub points: Vec<[f64; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct GetPotentialRoutesRequest {
    #[serde(rename = "tripRouteId")]
    pub trip: uuid::Uuid,

    #[serde(rename = "cargoRequestRouteIds")]
    pub cargo_requests: Vec<uuid::Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct GetPotentialRoutesResponse {
    #[serde(rename = "routeIds")]
    pub requests: Vec<uuid::Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct MergeRoutesRequest {
  #[serde(rename = "tripRouteId")]
  pub trip: uuid::Uuid,

  #[serde(rename = "cargoRequestRouteId")]
  pub requests: Vec<uuid::Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct MergeRoutesResponse {
    #[serde(rename = "routeId")]
    pub route: uuid::Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct RemoveStationsRequest {
    #[serde(rename = "deleteStationIds")]
    pub delete_stations: Vec<uuid::Uuid>,

    #[serde(rename = "tripId")]
    pub trip: uuid::Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
