use axum::extract::{Json, Path, State};
use sqlx::postgres::types::PgPoint;
use uuid::Uuid;

use crate::api::map_service;

use super::types::*;

pub type Result<T> = std::result::Result<T, ErrorResponse>;

async fn create_route(
    client: &map_service::Client,
    pool: &sqlx::PgPool,
    from: &Station,
    to: &Station,
    is_request: bool,
) -> Result<Uuid> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ErrorResponse::new(format!("error starting transaction: {e}")))?;

    for station in [from, to] {
        let station_id: Option<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM station WHERE id = $1;")
        .bind(&station.id)
        .fetch_optional(&mut *tx)
        .await?;

        if station_id.is_none() {
            sqlx::query(
                "INSERT INTO station (id, address, coords)
                VALUES ($1, $2, $3);",
            )
            .bind(station.id)
            .bind(&station.address)
            .bind(PgPoint {
                x: station.coords.lat,
                y: station.coords.lon,
            })
            .execute(&mut *tx)
            .await?;
        }
    }

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
        .bind(from.id)
        .bind(to.id)
        .fetch_one(&mut *tx)
        .await?;

    if sqlx::query("SELECT 1 FROM segment WHERE s1 = $1 and s2 = $2")
        .bind(&from.id)
        .bind(&to.id)
        .fetch_optional(&mut *tx)
        .await?
        .is_none() 
    {
        let route = client
            .create_route(map_service::CreateRouteRequest {
                stops: vec![
                    [from.coords.lat, from.coords.lon],
                    [to.coords.lat, to.coords.lon],
                ],
            })
            .await
            .map_err(|e| ErrorResponse::new(format!("map service returned error: {e}")))?;

        sqlx::query(
            "INSERT INTO segment (s1, s2, points, distance, time)
            VALUES ($1, $2, $3, $4, $5);",
        )
        .bind(from.id)
        .bind(to.id)
        .bind(
            route
                .way
                .into_iter()
                .map(|[x, y]| PgPoint { x, y })
                .collect::<Vec<_>>(),
        )
        .bind(route.distance as i32)
        .bind(route.duration as i32)
        .execute(&mut *tx)
        .await?;
    }

    if !is_request {
        sqlx::query(
            "INSERT INTO path (trip_id, station_id, index) VALUES ($1, $2, 0), ($1, $3, 1);",
        )
        .bind(id)
        .bind(from.id)
        .bind(to.id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ErrorResponse::new(format!("error commiting transaction: {e}")))?;

    Ok(id)
}

pub async fn create_cargo_request(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<CreateRouteRequest>,
) -> Result<Json<CreateRouteResponse>> {
    let id = create_route(&client, &pool, &r.from_station, &r.to_station, true).await?;
    Ok(Json(CreateRouteResponse { id }))
}

pub async fn create_trip(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<CreateRouteRequest>,
) -> Result<Json<CreateRouteResponse>> {
    let id = create_route(&client, &pool, &r.from_station, &r.to_station, false).await?;
    Ok(Json(CreateRouteResponse { id }))
}

pub async fn get_cargo_request(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetWaypointsRequest>,
) -> Result<Json<GetWaypointsResponse>> {
    let info: Option<(Uuid, String, PgPoint, Uuid, String, PgPoint, i32, i32)> = sqlx::query_as(
        "SELECT 
            s_source.id AS source_id,
            s_source.address AS source_address,
            s_source.coords AS source_coords,
            s_dest.id AS destination_id,
            s_dest.address AS destination_address,
            s_dest.coords AS destination_coords,
            seg.distance,
            seg.time
        FROM request r
        INNER JOIN station s_source ON r.source = s_source.id
        INNER JOIN station s_dest ON r.destination = s_dest.id
        LEFT JOIN segment seg ON seg.s1 = r.source AND seg.s2 = r.destination
        WHERE r.id = $1;",
    )
    .bind(&r.id)
    .fetch_optional(&pool)
    .await?;

    let Some(info) = info else {
        return Err(ErrorResponse::new(format!(
            "cannot find cargo request with id {}",
            r.id
        )));
    };

    let (src_id, src_addr, src_coords, dst_id, dst_addr, dst_coords, distance, time) = info;

    let response = GetWaypointsResponse {
        stations: vec![
            Waypoint {
                station: Station {
                    id: src_id,
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
                    id: dst_id,
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
    Path(r): Path<GetWaypointsRequest>,
) -> Result<Json<GetWaypointsResponse>> {
    let segments: Vec<(Uuid, String, PgPoint, Uuid, String, PgPoint, i32, i32)> = sqlx::query_as(
        "SELECT 
            s_source.id AS source_id,
            s_source.address AS source_address,
            s_source.coords AS source_coords,
            s_dest.id AS destonation_id,
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
        ORDER BY p1.index;",
    )
    .bind(&r.id)
    .fetch_all(&pool)
    .await?;

    if segments.is_empty() {
        return Err(ErrorResponse::new(format!(
            "cannot find trip with id {}",
            r.id
        )));
    }

    let mut waypoints = Vec::new();

    waypoints.push(Waypoint {
        station: Station {
            id: segments[0].0,
            address: segments[0].1.clone(),
            coords: Coords {
                lat: segments[0].2.x,
                lon: segments[0].2.y,
            },
        },

        distance: 0,
        trip_time: 0,
    });

    for segment in segments {
        waypoints.push(Waypoint {
            station: Station {
                id: segment.3,
                address: segment.4.clone(),
                coords: Coords {
                    lat: segment.5.x,
                    lon: segment.5.y,
                },
            },

            distance: segment.6 as u64,
            trip_time: segment.7 as u64,
        });
    }

    Ok(Json(GetWaypointsResponse {
        stations: waypoints,
    }))
}

async fn fetch_request_points(pool: &sqlx::PgPool, request_id: &Uuid) -> Result<Vec<[f64; 2]>> {
    let pg_points: Option<Vec<PgPoint>> = sqlx::query_scalar(
        "SELECT     
            seg.points
        FROM request r        
        INNER JOIN station s_source ON r.source = s_source.id        
        INNER JOIN station s_dest ON r.destination = s_dest.id        
        LEFT JOIN segment seg ON seg.s1 = r.source AND seg.s2 = r.destination        
        WHERE r.id = $1;",
    )
    .bind(request_id)
    .fetch_optional(pool)
    .await?;

    let Some(pg_points) = pg_points else {
        return Err(ErrorResponse::new(format!(
            "there are no points for request id {}",
            request_id
        )));
    };

    let points = pg_points.into_iter().map(|p| [p.x, p.y]).collect();

    Ok(points)
}

pub async fn get_cargo_request_points(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetPointsRequest>,
) -> Result<Json<GetPointsResponse>> {
    let points = fetch_request_points(&pool, &r.id).await?;

    if points.is_empty() {
        return Err(ErrorResponse::new(format!(
            "cannot find cargo request points for id {}",
            r.id
        )));
    }

    Ok(Json(GetPointsResponse { points }))
}

async fn fetch_trip_points(pool: &sqlx::PgPool, trip_id: &Uuid) -> Result<Vec<[f64; 2]>> {
    let pg_points: Option<Vec<PgPoint>> = sqlx::query_scalar(
        "SELECT array_agg(point ORDER BY p1.index, idx) AS flat_points
        FROM path p1
        JOIN path p2 ON p1.trip_id = p2.trip_id AND p2.index = p1.index + 1
        LEFT JOIN segment seg ON seg.s1 = p1.station_id AND seg.s2 = p2.station_id
        CROSS JOIN LATERAL unnest(COALESCE(seg.points, '{}')) WITH ORDINALITY AS points(point, idx)
        WHERE p1.trip_id = $1;",
    )
    .bind(trip_id)
    .fetch_one(pool)
    .await?;

    let Some(pg_points) = pg_points else {
        return Err(ErrorResponse::new(format!(
            "there are no points for trip id {}",
            trip_id
        )));
    };

    let points = pg_points.into_iter().map(|p| [p.x, p.y]).collect();

    Ok(points)
}

fn distance(p1: &PgPoint, p2: &PgPoint) -> f64 {
    const R: f64 = 6371.0;

    let lat1 = p1.y.to_radians();
    let lon1 = p1.x.to_radians();
    let lat2 = p2.y.to_radians();
    let lon2 = p2.x.to_radians();

    let dlat = lat2 - lat1;
    let dlon = lon2 - lon1;

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);

    R * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())
}

pub async fn get_trip_points(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetPointsRequest>,
) -> Result<Json<GetPointsResponse>> {
    let points = fetch_trip_points(&pool, &r.id).await?;

    if points.is_empty() {
        return Err(ErrorResponse::new(format!(
            "cannot find trip points for id {}",
            r.id
        )));
    }

    Ok(Json(GetPointsResponse { points }))
}

pub async fn get_potential_routes(
    State(pool): State<sqlx::PgPool>,
    Json(r): Json<GetPotentialRoutesRequest>,
) -> Result<Json<GetPotentialRoutesResponse>> {
    let mut trip_stations: Vec<PgPoint> = sqlx::query_scalar(
        "SELECT s.coords
        FROM path p
        INNER JOIN station s ON p.station_id = s.id
        WHERE p.trip_id = $1
        ORDER BY p.index;",
    )
    .bind(&r.trip)
    .fetch_all(&pool)
    .await?;

    if trip_stations.is_empty() {
        return Err(ErrorResponse::new(format!(
            "cannot find trip with id {}",
            r.trip
        )));
    }

    let mut route_ids = Vec::new();

    for id in &r.cargo_requests {
        let request: Option<(PgPoint, PgPoint)> = sqlx::query_as(
            "SELECT 
                s_source.coords AS source_coords,
                s_dest.coords AS destination_coords
            FROM request r
            INNER JOIN station s_source ON r.source = s_source.id
            INNER JOIN station s_dest ON r.destination = s_dest.id
            WHERE r.id = $1;",
        )
        .bind(id)
        .fetch_optional(&pool)
        .await?;

        let Some(request) = request else {
            return Err(ErrorResponse::new(format!(
                "cannot find cargo request with id {}",
                id
            )));
        };

        let (insert_src_idx, _) = trip_stations
            .windows(2)
            .map(|stations| {
                distance(&stations[0], &request.0) + distance(&request.0, &stations[1])
                    - distance(&stations[0], &stations[1])
            })
            .enumerate()
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .unwrap();

        trip_stations.insert(insert_src_idx + 1, request.0);

        let (insert_dst_idx, _) = trip_stations
            .windows(2)
            .map(|stations| {
                distance(&stations[0], &request.1) + distance(&request.1, &stations[1])
                    - distance(&stations[0], &stations[1])
            })
            .enumerate()
            .skip(insert_src_idx + 1)
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .unwrap();

        trip_stations.insert(insert_dst_idx + 1, request.1);

        let res_distance = distance(
            &trip_stations[insert_src_idx],
            &trip_stations[insert_src_idx + 1],
        ) + distance(
            &trip_stations[insert_src_idx + 1],
            &trip_stations[insert_src_idx + 2],
        ) - distance(
            &trip_stations[insert_src_idx],
            &trip_stations[insert_src_idx + 2],
        ) + distance(
            &trip_stations[insert_dst_idx],
            &trip_stations[insert_dst_idx + 1],
        ) + distance(
            &trip_stations[insert_dst_idx + 1],
            &trip_stations[insert_dst_idx + 2],
        ) - distance(
            &trip_stations[insert_dst_idx],
            &trip_stations[insert_dst_idx + 2],
        );

        trip_stations.remove(insert_dst_idx + 1);
        trip_stations.remove(insert_src_idx + 1);

        route_ids.push((id, res_distance));
    }

    route_ids.sort_by(|a, b| a.1.total_cmp(&b.1));
    route_ids.retain(|(_, distance)| *distance < 10000.0);

    Ok(Json(GetPotentialRoutesResponse {
        requests: route_ids.into_iter().map(|(id, _)| id.clone()).collect(),
    }))
}

async fn get_request_stations(
    pool: &sqlx::PgPool,
    id: &Uuid,
) -> Result<Option<(uuid::Uuid, PgPoint, uuid::Uuid, PgPoint)>> {
    let stations = sqlx::query_as(
        "SELECT 
            s_src.id AS src_station_id,
            s_src.coords AS src_coords,
            s_dst.id AS dst_station_id,
            s_dst.coords AS dst_coords
        FROM request r
        INNER JOIN station s_src ON r.source = s_src.id
        INNER JOIN station s_dst ON r.destination = s_dst.id
        WHERE r.id = $1;",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(stations)
}

pub async fn merge_routes(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<MergeRoutesRequest>,
) -> Result<Json<MergeRoutesResponse>> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ErrorResponse::new(format!("error starting transaction: {e}")))?;

    let mut trip_stations: Vec<(uuid::Uuid, PgPoint)> = sqlx::query_as(
        "SELECT s.id, s.coords
        FROM path p
        INNER JOIN station s ON p.station_id = s.id
        WHERE p.trip_id = $1
        ORDER BY p.index;",
    )
    .bind(&r.trip)
    .fetch_all(&mut *tx)
    .await?;

    if trip_stations.is_empty() {
        return Err(ErrorResponse::new(format!(
            "cannot find trip with id {}",
            r.trip
        )));
    }

    for request in &r.requests {
        let Some((req_src_id, req_src_coords, req_dst_id, req_dst_coords)) =
            get_request_stations(&pool, request).await?
        else {
            return Err(ErrorResponse::new(format!(
                "cannot find cargo request with id {}",
                request
            )));
        };

        let (insert_src_idx, _) = trip_stations
            .windows(2)
            .map(|stations| {
                distance(&stations[0].1, &req_src_coords)
                    + distance(&req_src_coords, &stations[1].1)
                    - distance(&stations[0].1, &stations[1].1)
            })
            .enumerate()
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .unwrap();

        trip_stations.insert(insert_src_idx + 1, (req_src_id, req_src_coords));

        let (insert_dst_idx, _) = trip_stations
            .windows(2)
            .map(|stations| {
                distance(&stations[0].1, &req_dst_coords)
                    + distance(&req_dst_coords, &stations[1].1)
                    - distance(&stations[0].1, &stations[1].1)
            })
            .enumerate()
            .skip(insert_src_idx + 1)
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .unwrap();

        trip_stations.insert(insert_dst_idx + 1, (req_dst_id, req_dst_coords));
    }

    let new_trip_id: Uuid = sqlx::query_scalar(
        "INSERT INTO trip (id, source, destination)
        VALUES (gen_random_uuid(), $1, $2)
        RETURNING id;",
    )
    .bind(trip_stations[0].0)
    .bind(trip_stations[trip_stations.len() - 1].0)
    .fetch_one(&mut *tx)
    .await?;

    // 7. Insert path entries
    for (index, station) in trip_stations.iter().enumerate() {
        sqlx::query(
            "INSERT INTO path (trip_id, station_id, index)
            VALUES ($1, $2, $3);",
        )
        .bind(&new_trip_id)
        .bind(station.0)
        .bind(index as i32)
        .execute(&mut *tx)
        .await?;
    }

    for i in 0..trip_stations.len() - 1 {
        let s1 = trip_stations[i].0;
        let s2 = trip_stations[i + 1].0;

        let existing: Option<(i32, i32)> =
            sqlx::query_as("SELECT distance, time FROM segment WHERE s1 = $1 AND s2 = $2;")
                .bind(s1)
                .bind(s2)
                .fetch_optional(&mut *tx)
                .await?;

        if existing.is_none() {
            let s1_coords: PgPoint = sqlx::query_scalar("SELECT coords FROM station WHERE id = $1;")
                .bind(s1)
                .fetch_one(&mut *tx)
                .await?;

            let s2_coords: PgPoint = sqlx::query_scalar("SELECT coords FROM station WHERE id = $1;")
                .bind(s2)
                .fetch_one(&mut *tx)
                .await?;

            let route = client
                .create_route(map_service::CreateRouteRequest {
                    stops: vec![[s1_coords.x, s1_coords.y], [s2_coords.x, s2_coords.y]],
                })
                .await
                .map_err(|e| ErrorResponse::new(format!("map service returned error: {e}")))?;

            sqlx::query(
                "INSERT INTO segment (s1, s2, points, distance, time)
                VALUES ($1, $2, $3, $4, $5);",
            )
            .bind(s1)
            .bind(s2)
            .bind(
                route
                    .way
                    .into_iter()
                    .map(|[x, y]| PgPoint { x, y })
                    .collect::<Vec<_>>(),
            )
            .bind(route.distance as i32)
            .bind(route.duration as i32)
            .execute(&mut *tx)
            .await?;
        }
    }

    for request in &r.requests {
        sqlx::query("UPDATE request SET trip_id = $1 WHERE id = $2;")
            .bind(&new_trip_id)
            .bind(request)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit()
        .await
        .map_err(|e| ErrorResponse::new(format!("error committing transaction: {e}")))?;

    Ok(Json(MergeRoutesResponse { route: new_trip_id }))
}
