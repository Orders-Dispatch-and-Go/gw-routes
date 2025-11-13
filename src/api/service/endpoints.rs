use axum::extract::{Json, Path};
use axum::http::StatusCode;

use super::types::*;

pub type Result<T> = std::result::Result<T, (StatusCode, Json<ErrorResponse>)>;

pub async fn create_cargo_request(
    Json(r): Json<CreateRouteRequest>
) -> Result<Json<CreateRouteResponse>> {
    todo!()
}

pub async fn create_trip(
    Json(r): Json<CreateRouteRequest>
) -> Result<Json<CreateRouteResponse>> {
    todo!()
}

pub async fn get_cargo_request(
    Path(r): Path<GetWaypointsRequest>
) -> Result<Json<GetWaypointsResponse>> {
    todo!()
}

pub async fn get_trip(
    Path(r): Path<GetWaypointsRequest>
) -> Result<Json<GetWaypointsResponse>> {
    todo!()
}

pub async fn get_cargo_request_points(
    Path(r): Path<GetPointsRequest>
) -> Result<Json<GetPointsResponse>> {
    todo!()
}

pub async fn get_trip_points(
    Path(r): Path<GetPointsRequest>
) -> Result<Json<GetPointsResponse>> {
    todo!()
}

pub async fn get_potential_routes(
    Json(r): Json<GetPotentialRoutesRequest>
) -> Result<Json<GetPotentialRoutesResponse>> {
    todo!()
}

pub async fn merge_routes(
    Json(r): Json<MergeRoutesRequest>
) -> Result<Json<MergeRoutesResponse>> {
    todo!()
}

pub async fn remove_stations(
    Json(r): Json<RemoveStationsRequest>
) -> Result<()> {
    todo!()
}
