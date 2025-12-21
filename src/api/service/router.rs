use axum::routing::{get, post};

use super::endpoints::*;

pub fn router(state: super::State) -> axum::Router {
    axum::Router::new()
        .route("/routes/cargo_requests", post(create_cargo_request))
        .route("/routes/trips", post(create_trip))
        .route("/routes/cargo_requests/{id}", get(get_cargo_request))
        .route("/routes/trips/{id}", get(get_trip))
        .route(
            "/routes/cargo_requests/{id}/points",
            get(get_cargo_request_points),
        )
        .route("/routes/trips/{id}/points", get(get_trip_points))
        .route("/routes/trips/potential", post(get_potential_routes))
        .route("/routes/trips/merge", post(merge_routes))
        .with_state(state)
}
