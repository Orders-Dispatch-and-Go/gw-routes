use axum::Json;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use sqlx::postgres::types::PgPoint;

use crate::api::map_service::client::Client;
use crate::api::map_service::{self, client};

use super::types::*;

pub fn router(state: super::State) -> axum::Router {
    axum::Router::new()
        .route("/api/create_route", post(create_route))
        .route("/api/route", get(get_route))
        .route("/api/create_merger_options", post(create_merge_options))
        .route("/api/merge_template", post(merge_template))
        .route("/api/merge_id", post(merge_route))
        .route("/api/remove_stops", post(remove_stations))
        .with_state(state)
}

async fn create_route(
    State(client): State<client::Client>,
    State(db): State<sqlx::PgPool>,
    Json(r): Json<CreateRouteRequest>,
) -> Result<Json<CreateRouteResponse>, StatusCode> {
    let response = client
        .create_route(map_service::types::CreateRouteRequest { stops: r.stops })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO route (waypoints)
        VALUES $1
        RETURNING id;"#,
    )
    .bind(
        response
            .way
            .into_iter()
            .map(|[x, y]| PgPoint { x, y })
            .collect::<Vec<_>>(),
    )
    .fetch_one(&db)
    .await
    .map_err(|e| {
        log::error!("db error while creating route: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = CreateRouteResponse {
        route_id: id,
        graph: response.graph,
    };

    Ok(Json(response))
}

async fn get_route(
    State(db): State<sqlx::PgPool>,
    Query(q): Query<GetRouteRequest>,
) -> Result<Json<GetRouteResponse>, StatusCode> {
    let waypoints: Vec<PgPoint> = sqlx::query_scalar(
        r#"SELECT waypoints
        FROM route
        WHERE id = $1;"#,
    )
    .bind(&q.route_id)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        log::error!("db error while getting route: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let waypoints = waypoints
        .into_iter()
        .map(|PgPoint { x, y }| [x, y])
        .collect::<Vec<_>>();

    let response = GetRouteResponse {
        way: waypoints,
        stations: Vec::new(), /* TODO */
    };

    Ok(Json(response))
}

async fn create_merge_options(
    Json(r): Json<CreateMergeOptionsRequest>,
) -> Json<CreateMergeOptionsResponse> {
    todo!()
}

async fn merge_template(Json(r): Json<MergeTemplateRequest>) -> Json<MergeResponse> {
    todo!()
}

async fn merge_route(Json(r): Json<MergeRouteRequest>) -> Json<MergeResponse> {
    todo!()
}

async fn remove_stations(
    State(db): State<sqlx::PgPool>,
    Json(r): Json<RemoveStationsRequest>
) -> Result<Json<RemoveStationsResponse>, StatusCode> {
    let waypoints: Vec<PgPoint> = sqlx::query_scalar(
        r#"SELECT waypoints
        FROM route
        WHERE id = $1;"#,
    )
    .bind(&r.route_id)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        log::error!("db error while getting route waypoints: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let new_route_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO route (waypoints)
        VALUES ($1)
        RETURNING id;"#,
    )
    .bind(&waypoints)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        log::error!("db error while creating new route: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
 
    let graph: Vec<(i64, i32)> = sqlx::query_as(
        r#"SELECT station_id, ord
        FROM graph
        WHERE route_id = $1 AND station_id != ALL($2)
        ORDER BY ord;"#,
    )
    .bind(&r.route_id)
    .bind(&r.graph)
    .fetch_all(&db)
    .await
    .map_err(|e| {
        log::error!("db error while getting graph entries: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    for (station_id, ord) in &graph {
        sqlx::query(
            r#"INSERT INTO graph (route_id, station_id, ord)
            VALUES ($1, $2, $3);"#,
        )
        .bind(&new_route_id)
        .bind(station_id)
        .bind(ord)
        .execute(&db)
        .await
        .map_err(|e| {
            log::error!("db error while inserting graph entry: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    Ok(Json(RemoveStationsResponse {
        route_id: new_route_id,
        graph: graph.into_iter().map(|(id, _)| id).collect(),
    }))
}
