use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use sqlx::postgres::types::PgPoint;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::map_service;

use super::types::*;

pub type Result<T> = std::result::Result<T, ErrorResponse>;

async fn create_route(
    client: &map_service::Client,
    pool: &sqlx::PgPool,
    from: &Station,
    to: &Station,
    is_request: bool
) -> Result<Uuid> {
    let mut tx = pool.begin().await.map_err(|e| ErrorResponse::new(format!("error starting transaction: {e}")))?;

    let station_ids: Vec<i32> = sqlx::query_scalar("
        INSERT INTO station (address, coords)
        VALUES ($1, $2), ($3, $4)
        RETURNING id;
    ")
        .bind(&from.address)
        .bind(PgPoint { x: from.coords.lat, y: from.coords.lon })
        .bind(&to.address)
        .bind(PgPoint { x: to.coords.lat, y: to.coords.lon })
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    let query = if is_request {
        "INSERT INTO request (id, source, destination)
        VALUES (gen_random_uuid(), $1, $2)
        RETURNING id;"
    } else {
        "INSERT INTO trip (id, source, destination)
        VALUES (gen_random_uuid(), $1, $2)
        RETURNING id;"
    };

    let id: Uuid = sqlx::query_scalar(query)
        .bind(&station_ids[0])
        .bind(&station_ids[1])
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    let route = client.create_route(map_service::CreateRouteRequest {
        stops: vec![[from.coords.lat, from.coords.lon], [to.coords.lat, to.coords.lon]],
    })
        .await
        .map_err(|e| ErrorResponse::new(format!("map service returned error: {e}")))?;

    sqlx::query("
        INSERT INTO segment (s1, s2, points, distance, time)
        VALUES ($1, $2, $3, $4, $5);
    ")
        .bind(station_ids[0])
        .bind(station_ids[1])
        .bind(route.way.into_iter().map(|[x, y]| PgPoint { x, y }).collect::<Vec<_>>())
        .bind(route.distance as i32)
        .bind(route.duration as i32)
        .execute(&mut *tx)    
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    tx.commit().await.map_err(|e| ErrorResponse::new(format!("error commiting transaction: {e}")))?;

    Ok(id)
}

pub async fn create_cargo_request(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<CreateRouteRequest>
) -> Result<Json<CreateRouteResponse>> {
    let id = create_route(&client, &pool, &r.from_station, &r.to_station, true).await?;
    Ok(Json(CreateRouteResponse { id }))
}

pub async fn create_trip(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<CreateRouteRequest>
) -> Result<Json<CreateRouteResponse>> {
    let id = create_route(&client, &pool, &r.from_station, &r.to_station, false).await?;
    Ok(Json(CreateRouteResponse { id }))
}

pub async fn get_cargo_request(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetWaypointsRequest>
) -> Result<Json<GetWaypointsResponse>> {
    let info: (String, PgPoint, String, PgPoint, i32, i32) = sqlx::query_as("
        SELECT 
            s_source.address AS source_address,
            s_source.coords AS source_coords,
            s_dest.address AS destination_address,
            s_dest.coords AS destination_coords,
            seg.distance,
            seg.time
        FROM request r
        INNER JOIN station s_source ON r.source = s_source.id
        INNER JOIN station s_dest ON r.destination = s_dest.id
        LEFT JOIN segment seg ON seg.s1 = r.source AND seg.s2 = r.destination
        WHERE r.id = $1;
    ")
        .bind(&r.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    let (src_addr, src_coords, dst_addr, dst_coords, distance, time) = info;

    let response = GetWaypointsResponse { 
        stations: vec![
            Waypoint {
                station: Station {
                    address: src_addr,
                    coords: Coords {
                        lat: src_coords.x,
                        lon: src_coords.y,
                    },
                },
                distance: 0,
                trip_time: 0,
            },
            Waypoint {
                station: Station {
                    address: dst_addr,
                    coords: Coords {
                        lat: dst_coords.x,
                        lon: dst_coords.y,
                    },
                },
                distance: distance as u64,
                trip_time: time as u64,
            },
        ], 
    };

    Ok(Json(response))
}

pub async fn get_trip(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetWaypointsRequest>
) -> Result<Json<GetWaypointsResponse>> {
    let segments: Vec<(String, PgPoint, String, PgPoint, i32, i32)> = sqlx::query_as("
        SELECT 
            s_source.address AS source_address,
            s_source.coords AS source_coords,
            s_dest.address AS destination_address,
            s_dest.coords AS destination_coords,
            seg.distance,
            seg.time
        FROM path p1
        INNER JOIN path p2 ON p1.trip_id = p2.trip_id AND p2.index = p1.index + 1
        INNER JOIN station s_source ON p1.station_id = s_source.id
        INNER JOIN station s_dest ON p2.station_id = s_dest.id
        LEFT JOIN segment seg ON seg.s1 = p1.station_id AND seg.s2 = p2.station_id
        WHERE p1.trip_id = $1
        ORDER BY p1.index;       
    ")
        .bind(&r.id)
        .fetch_all(&pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    let mut waypoints = Vec::new();

    waypoints.push(Waypoint {
        station: Station {
            address: segments[0].0.clone(),
            coords: Coords {
                lat: segments[0].1.x,
                lon: segments[0].1.y,
            },
        },

        distance: 0,
        trip_time: 0,
    });

    for segment in segments {
        waypoints.push(Waypoint {
            station: Station {
                address: segment.2.clone(),
                coords: Coords {
                    lat: segment.3.x,
                    lon: segment.3.y,
                },
            },

            distance: segment.4 as u64,
            trip_time: segment.5 as u64,
        });
    }

    Ok(Json(GetWaypointsResponse {
        stations: waypoints,
    }))
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
