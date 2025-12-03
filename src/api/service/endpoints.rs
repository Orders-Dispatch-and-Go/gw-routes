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
        INSERT INTO station (id, address, coords)
        VALUES (gen_random_uuid(), $1, $2), (gen_random_uuid(), $3, $4)
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
    let info: (Uuid, String, PgPoint, Uuid, String, PgPoint, i32, i32) = sqlx::query_as("
        SELECT 
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
        WHERE r.id = $1;
    ")
        .bind(&r.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

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
    Path(r): Path<GetWaypointsRequest>
) -> Result<Json<GetWaypointsResponse>> {
    let segments: Vec<(Uuid, String, PgPoint, Uuid, String, PgPoint, i32, i32)> = sqlx::query_as("
        SELECT 
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
        ORDER BY p1.index;       
    ")
        .bind(&r.id)
        .fetch_all(&pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

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

async fn fetch_request_points(
    pool: &sqlx::PgPool,
    request_id: &Uuid
) -> Result<Vec<[f64; 2]>> {
    let pg_points: Vec<PgPoint> = sqlx::query_scalar("
        SELECT     
            seg.points
        FROM request r        
        INNER JOIN station s_source ON r.source = s_source.id        
        INNER JOIN station s_dest ON r.destination = s_dest.id        
        LEFT JOIN segment seg ON seg.s1 = r.source AND seg.s2 = r.destination        
        WHERE r.id = $1;
    ")
        .bind(request_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    let points = pg_points
        .into_iter()
        .map(|p| [p.x, p.y])
        .collect();

    Ok(points)
}

pub async fn get_cargo_request_points(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetPointsRequest>
) -> Result<Json<GetPointsResponse>> {
    let points = fetch_request_points(&pool, &r.id).await?;
    Ok(Json(GetPointsResponse { points }))
}

async fn fetch_trip_points(
    pool: &sqlx::PgPool,
    trip_id: &Uuid
) -> Result<Vec<[f64; 2]>> {
    let pg_points: Vec<PgPoint> = sqlx::query_scalar("
        SELECT array_agg(ARRAY[point[0], point[1]] ORDER BY p1.index, idx) AS flat_points
        FROM path p1
        JOIN path p2 ON p1.trip_id = p2.trip_id AND p2.index = p1.index + 1
        LEFT JOIN segment seg ON seg.s1 = p1.station_id AND seg.s2 = p2.station_id
        CROSS JOIN LATERAL unnest(COALESCE(seg.points, '{}')) WITH ORDINALITY AS points(point, idx)
        WHERE p1.trip_id = $1;
    ")
        .bind(trip_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    let points = pg_points
        .into_iter()
        .map(|p| [p.x, p.y])
        .collect();

    Ok(points)
}

pub async fn get_trip_points(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetPointsRequest>
) -> Result<Json<GetPointsResponse>> {
    let points = fetch_trip_points(&pool, &r.id).await?;
    Ok(Json(GetPointsResponse { points }))
}

pub async fn get_potential_routes(
    State(pool): State<sqlx::PgPool>,
    Json(r): Json<GetPotentialRoutesRequest>
) -> Result<Json<GetPotentialRoutesResponse>> {
    let points_to_coords = |points: &[[f64; 2]]| {
        points
            .iter()
            .map(|p| crate::types::Coord { lat: p[0], lon: p[1] })
            .collect::<Vec<_>>()
    };

    let request_points = points_to_coords(&fetch_trip_points(&pool, &r.cargo_request).await?);

    let mut trips = Vec::new();

    for trip in &r.trips {
        let points = fetch_trip_points(&pool, trip).await?;
        trips.push((trip.clone(), points_to_coords(&points)));
    }

    trips.sort_unstable_by(|(_, points1), (_, points2)| {
        let d1 = crate::ffi::distance(&points1, &request_points);
        let d2 = crate::ffi::distance(&points2, &request_points);

        d1.total_cmp(&d2)
    });

    /* TODO: remove trips with distance higher than MAX_DISTANCE */

    let ids = trips
        .into_iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();

    Ok(Json(GetPotentialRoutesResponse { trips: ids }))
}

async fn get_request_stations(
    pool: &sqlx::PgPool,
    id: &Uuid
) -> Result<(i32, PgPoint, i32, PgPoint)> {
    let stations = sqlx::query_as("
        SELECT 
            s_src.id AS src_station_id,
            s_src.coords AS src_coords,
            s_dst.id AS dst_station_id,
            s_dst.coords AS dst_coords
        FROM request r
        INNER JOIN station s_src ON r.source = s_src.id
        INNER JOIN station s_dst ON r.destination = s_dst.id
        WHERE r.id = $1;
    ")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    Ok(stations)
}

async fn get_trip_stations(
    pool: &sqlx::PgPool,
    id: &Uuid
) -> Result<Vec<(i32, PgPoint, i32, PgPoint)>> {
    let pairs: Vec<(i32, PgPoint, i32, PgPoint)> = sqlx::query_as("
        SELECT 
            s_src.id AS src_station_id,
            s_src.coords AS src_coords,
            s_dst.id AS dst_station_id,
            s_dst.coords AS dst_coords
        FROM trip t
        INNER JOIN station s_src ON t.source = s_src.id
        INNER JOIN station s_dst ON t.destination = s_dst.id
        WHERE t.id = $1

        UNION ALL

        SELECT 
            s_src.id AS src_station_id,
            s_src.coords AS src_coords,
            s_dst.id AS dst_station_id,
            s_dst.coords AS dst_coords
        FROM request r
        INNER JOIN station s_src ON r.source = s_src.id
        INNER JOIN station s_dst ON r.destination = s_dst.id
        WHERE r.trip_id = $1;
    ")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    Ok(pairs)
}

pub async fn merge_routes(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<MergeRoutesRequest>
) -> Result<Json<MergeRoutesResponse>> {
    let mut tx = pool.begin().await.map_err(|e| ErrorResponse::new(format!("error starting transaction: {e}")))?;

    // 1. Get request's source and destination stations
    let (req_src_id, req_src_coords, req_dst_id, req_dst_coords) = get_request_stations(&pool, &r.cargo_request).await?;

    // 2. Get all stations in the trip in order
    let trip_stations: Vec<(i32, PgPoint)> = sqlx::query_as("
        SELECT s.id, s.coords
        FROM path p
        INNER JOIN station s ON p.station_id = s.id
        WHERE p.trip_id = $1
        ORDER BY p.index;
    ")
        .bind(&r.trip)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    if trip_stations.is_empty() {
        return Err(ErrorResponse::new("trip has no stations"));
    }

    // Helper function to calculate distance
    let distance = |p1: &PgPoint, p2: &PgPoint| -> f64 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        (dx * dx + dy * dy).sqrt()
    };

    // 3. Find closest station in trip to request source
    let (insert_src_idx, _) = trip_stations
        .iter()
        .enumerate()
        .min_by(|(_, (_, coords1)), (_, (_, coords2))| {
            distance(coords1, &req_src_coords).total_cmp(&distance(coords2, &req_src_coords))
        })
        .ok_or_else(|| ErrorResponse::new("no stations in trip"))?;

    // 4. Find closest station after source insertion point to request destination
    let (insert_dst_idx, _) = trip_stations
        .iter()
        .enumerate()
        .skip(insert_src_idx + 1)
        .min_by(|(_, (_, coords1)), (_, (_, coords2))| {
            distance(coords1, &req_dst_coords).total_cmp(&distance(coords2, &req_dst_coords))
        })
        .ok_or_else(|| ErrorResponse::new("no valid destination insertion point"))?;

    // 5. Build new station order
    let mut new_stations = Vec::new();
    
    // Add stations before source insertion
    for i in 0..=insert_src_idx {
        new_stations.push(trip_stations[i].0);
    }
    
    // Add request source
    new_stations.push(req_src_id);
    
    // Add stations between source and destination insertion points
    for i in (insert_src_idx + 1)..=insert_dst_idx {
        new_stations.push(trip_stations[i].0);
    }
    
    // Add request destination
    new_stations.push(req_dst_id);
    
    // Add remaining stations after destination insertion
    for i in (insert_dst_idx + 1)..trip_stations.len() {
        new_stations.push(trip_stations[i].0);
    }

    // 6. Create new trip
    let new_trip_id: Uuid = sqlx::query_scalar("
        INSERT INTO trip (id, source, destination)
        VALUES (gen_random_uuid(), $1, $2)
        RETURNING id;
    ")
        .bind(new_stations[0])
        .bind(new_stations[new_stations.len() - 1])
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    // 7. Insert path entries
    for (index, &station_id) in new_stations.iter().enumerate() {
        sqlx::query("
            INSERT INTO path (trip_id, station_id, index)
            VALUES ($1, $2, $3);
        ")
            .bind(&new_trip_id)
            .bind(station_id)
            .bind(index as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;
    }

    // 8. Create segments between consecutive stations
    for i in 0..new_stations.len() - 1 {
        let s1 = new_stations[i];
        let s2 = new_stations[i + 1];

        // Check if segment already exists
        let existing: Option<(i32, i32)> = sqlx::query_as("
            SELECT distance, time FROM segment WHERE s1 = $1 AND s2 = $2;
        ")
            .bind(s1)
            .bind(s2)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

        if existing.is_none() {
            // Get coordinates for both stations
            let coords: Vec<PgPoint> = sqlx::query_scalar("
                SELECT coords FROM station WHERE id = ANY($1);
            ")
                .bind(&[s1, s2])
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

            // Create route via map service
            let route = client.create_route(map_service::CreateRouteRequest {
                stops: vec![[coords[0].x, coords[0].y], [coords[1].x, coords[1].y]],
            })
                .await
                .map_err(|e| ErrorResponse::new(format!("map service returned error: {e}")))?;

            // Insert segment
            sqlx::query("
                INSERT INTO segment (s1, s2, points, distance, time)
                VALUES ($1, $2, $3, $4, $5);
            ")
                .bind(s1)
                .bind(s2)
                .bind(route.way.into_iter().map(|[x, y]| PgPoint { x, y }).collect::<Vec<_>>())
                .bind(route.distance as i32)
                .bind(route.duration as i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;
        }
    }

    // 9. Update request to reference the new trip
    sqlx::query("
        UPDATE request SET trip_id = $1 WHERE id = $2;
    ")
        .bind(&new_trip_id)
        .bind(&r.cargo_request)
        .execute(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    tx.commit().await.map_err(|e| ErrorResponse::new(format!("error committing transaction: {e}")))?;

    Ok(Json(MergeRoutesResponse { route: new_trip_id }))
}

pub async fn remove_stations(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<RemoveStationsRequest>
) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| ErrorResponse::new(format!("error starting transaction: {e}")))?;

    // Get station IDs to delete from the requests
    let mut stations_to_delete = Vec::new();
    for req_id in &r.delete_stations {
        let (src_id, _, dst_id, _): (i32, PgPoint, i32, PgPoint) = sqlx::query_as("
            SELECT 
                s_src.id, s_src.coords,
                s_dst.id, s_dst.coords
            FROM request req
            INNER JOIN station s_src ON req.source = s_src.id
            INNER JOIN station s_dst ON req.destination = s_dst.id
            WHERE req.id = $1 AND req.trip_id = $2;
        ")
            .bind(req_id)
            .bind(&r.trip)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

        stations_to_delete.push(src_id);
        stations_to_delete.push(dst_id);
    }

    if stations_to_delete.is_empty() {
        return Err(ErrorResponse::new("no stations to delete"));
    }

    // Get current path
    let mut path: Vec<(i32, i32)> = sqlx::query_as("
        SELECT station_id, index
        FROM path
        WHERE trip_id = $1
        ORDER BY index;
    ")
        .bind(&r.trip)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    // Remove deleted stations from path
    path.retain(|(station_id, _)| !stations_to_delete.contains(station_id));

    if path.len() < 2 {
        return Err(ErrorResponse::new("cannot delete all stations from trip"));
    }

    // Delete old path entries
    sqlx::query("DELETE FROM path WHERE trip_id = $1;")
        .bind(&r.trip)
        .execute(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    // Insert new path with updated indices
    for (index, (station_id, _)) in path.iter().enumerate() {
        sqlx::query("
            INSERT INTO path (trip_id, station_id, index)
            VALUES ($1, $2, $3);
        ")
            .bind(&r.trip)
            .bind(station_id)
            .bind(index as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;
    }

    // Update trip source and destination
    sqlx::query("
        UPDATE trip
        SET source = $1, destination = $2
        WHERE id = $3;
    ")
        .bind(path[0].0)
        .bind(path[path.len() - 1].0)
        .bind(&r.trip)
        .execute(&mut *tx)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    // Create new segments for consecutive stations
    for i in 0..path.len() - 1 {
        let s1 = path[i].0;
        let s2 = path[i + 1].0;

        // Check if segment already exists
        let existing: Option<i32> = sqlx::query_scalar("
            SELECT 1 FROM segment WHERE s1 = $1 AND s2 = $2;
        ")
            .bind(s1)
            .bind(s2)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

        if existing.is_none() {
            // Get coordinates for both stations
            let coords: Vec<PgPoint> = sqlx::query_scalar("
                SELECT coords FROM station WHERE id = ANY($1) ORDER BY id = $2 DESC;
            ")
                .bind(&[s1, s2])
                .bind(s1)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

            if coords.len() != 2 {
                return Err(ErrorResponse::new("missing station coordinates"));
            }

            // Create route via map service
            let route = client.create_route(map_service::CreateRouteRequest {
                stops: vec![[coords[0].x, coords[0].y], [coords[1].x, coords[1].y]],
            })
                .await
                .map_err(|e| ErrorResponse::new(format!("map service returned error: {e}")))?;

            // Insert segment
            sqlx::query("
                INSERT INTO segment (s1, s2, points, distance, time)
                VALUES ($1, $2, $3, $4, $5);
            ")
                .bind(s1)
                .bind(s2)
                .bind(route.way.into_iter().map(|[x, y]| PgPoint { x, y }).collect::<Vec<_>>())
                .bind(route.distance as i32)
                .bind(route.duration as i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;
        }
    }

    // Unlink removed requests from the trip
    for req_id in &r.delete_stations {
        sqlx::query("
            UPDATE request SET trip_id = NULL WHERE id = $1;
        ")
            .bind(req_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;
    }

    tx.commit().await.map_err(|e| ErrorResponse::new(format!("error committing transaction: {e}")))?;

    Ok(())
}

pub async fn get_station(
    State(pool): State<sqlx::PgPool>,
    Json(r): Json<GetStationRequest>
) -> Result<Json<GetStationResponse>> {
    let (address, coords): (String, PgPoint) = sqlx::query_as("
        SELECT address, coords
        FROM station 
        WHERE id = $1
    ")
        .bind(&r.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ErrorResponse::new(format!("db returned error: {e}")))?;

    Ok(Json(GetStationResponse {station: 
        Station {
            id: r.id,
            address, 
            coords: Coords {
                lat: coords.x, 
                lon: coords.y
                }
            }
        }
    ))
}