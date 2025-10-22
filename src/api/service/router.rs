use axum::Json;
use axum::extract::Query;
use axum::routing::{get, post};

use super::types::*;

pub fn router(db: crate::db::Database) -> axum::Router<crate::db::Database> {
    axum::Router::new()
        .route("/api/create_route", post(create_route))
        .route("/api/route", get(get_route))
        .route("/api/create_merger_options", post(create_merge_options))
        .route("/api/merge_template", post(merge_template))
        .route("/api/merge_id", post(merge_route))
        .route("/api/remove_stops", post(remove_stations))
        .with_state(db)
}

async fn create_route(r: Json<CreateRouteRequest>) -> Json<CreateRouteResponse> {
    todo!()
}

async fn get_route(r: Query<GetRouteRequest>) -> Json<GetRouteResponse> {
    todo!()
}

async fn create_merge_options(
    r: Json<CreateMergeOptionsRequest>,
) -> Json<CreateMergeOptionsResponse> {
    todo!()
}

async fn merge_template(r: Json<MergeTemplateRequest>) -> Json<MergeResponse> {
    todo!()
}

async fn merge_route(r: Json<MergeRouteRequest>) -> Json<MergeResponse> {
    todo!()
}

async fn remove_stations(r: Json<RemoveStationsRequest>) -> Json<RemoveStationsResponse> {
    todo!()
}
