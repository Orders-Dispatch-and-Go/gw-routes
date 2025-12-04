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
        VALUES ($1, $2, $3), ($4, $5, $6)
        RETURNING id;
    ")
        .bind(&from.id)
        .bind(&from.address)
        .bind(PgPoint { x: from.coords.lat, y: from.coords.lon })
        .bind(&to.id)
        .bind(&to.address)
        .bind(PgPoint { x: to.coords.lat, y: to.coords.lon })
        .fetch_all(&mut *tx)
        .await?;

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
        .await?;

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
        .await?;

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
        .await?;

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
        .await?;

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
        .await?;

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
        .await?;

    let points = pg_points
        .into_iter()
        .map(|p| [p.x, p.y])
        .collect();

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
    
    let a = (dlat / 2.0).sin().powi(2) + 
            lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    
    R * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())
}

pub async fn get_trip_points(
    State(pool): State<sqlx::PgPool>,
    Path(r): Path<GetPointsRequest>
) -> Result<Json<GetPointsResponse>> {
    let points = fetch_trip_points(&pool, &r.id).await?;
    Ok(Json(GetPointsResponse { points }))
}

// pub async fn get_potential_routes(
//     State(pool): State<sqlx::PgPool>,
//     Json(r): Json<GetPotentialRoutesRequest>
// ) -> Result<Json<GetPotentialRoutesResponse>> {
//     let points_to_coords = |points: &[[f64; 2]]| {
//         points
//             .iter()
//             .map(|p| crate::types::Coord { lat: p[0], lon: p[1] })
//             .collect::<Vec<_>>()
//     };
//
//     let trip_points = points_to_coords(&fetch_trip_points(&pool, &r.trip).await?);
//
//     let mut requests = Vec::new();
//
//     for trip in &r.cargo_requests {
//         let points = fetch_trip_points(&pool, trip).await?;
//         requests.push((trip.clone(), points_to_coords(&points)));
//     }
//
//     requests.sort_unstable_by(|(_, points1), (_, points2)| {
//         let d1 = crate::ffi::distance(&points1, &trip_points);
//         let d2 = crate::ffi::distance(&points2, &trip_points);
//
//         d1.total_cmp(&d2)
//     });
//
//     /* TODO: remove trips with distance higher than MAX_DISTANCE */
//
//     let ids = requests
//         .into_iter()
//         .map(|(id, _)| id)
//         .collect::<Vec<_>>();
//
//     Ok(Json(GetPotentialRoutesResponse { requests: ids }))
// }

pub async fn get_potential_routes(
    State(pool): State<sqlx::PgPool>,
    Json(r): Json<GetPotentialRoutesRequest>
) -> Result<Json<GetPotentialRoutesResponse>> {
    let mut trip_stations: Vec<PgPoint> = sqlx::query_scalar("
        SELECT s.coords
        FROM path p
        INNER JOIN station s ON p.station_id = s.id
        WHERE p.trip_id = $1
        ORDER BY p.index;
    ")
        .bind(&r.trip)
        .fetch_all(&pool)
        .await?;

    let mut route_ids = Vec::new();

    for id in &r.cargo_requests {
        let request: (PgPoint, PgPoint) = sqlx::query_as("
            SELECT 
                s_source.coords AS source_coords,
                s_dest.coords AS destination_coords
            FROM request r
            INNER JOIN station s_source ON r.source = s_source.id
            INNER JOIN station s_dest ON r.destination = s_dest.id
            WHERE r.id = $1;
        ")
        .bind(id)
        .fetch_one(&pool)
        .await?;

        let (insert_src_idx, _) = trip_stations
            .iter()
            .enumerate()
            .min_by(|(_, coords1), (_, coords2)| {
                distance(coords1, &request.0).total_cmp(&distance(coords2, &request.0))
            })
            .unwrap();

        trip_stations.insert(insert_src_idx + 1, request.0);

        let (insert_dst_idx, _) = trip_stations[..trip_stations.len() - 1]
            .iter()
            .enumerate()
            .skip(insert_src_idx + 1)
            .min_by(|(_, coords1), (_, coords2)| {
                distance(coords1, &request.1).total_cmp(&distance(coords2, &request.1))
            })
            .unwrap();

        trip_stations.insert(insert_dst_idx + 1, request.1);

        let res_distance = distance(&trip_stations[insert_src_idx], &trip_stations[insert_src_idx + 1]) +
                           distance(&trip_stations[insert_src_idx + 1], &trip_stations[insert_src_idx + 2]) +
                           distance(&trip_stations[insert_dst_idx], &trip_stations[insert_dst_idx + 1]) +
                           distance(&trip_stations[insert_dst_idx + 1], &trip_stations[insert_dst_idx + 2]);

        trip_stations.remove(insert_dst_idx + 1);
        trip_stations.remove(insert_src_idx + 1);

        route_ids.push((id, res_distance));
    }

    route_ids.sort_by(|a, b| a.1.total_cmp(&b.1));
    route_ids.retain(|(_, distance)| *distance < 10000.0);

    Ok(Json(GetPotentialRoutesResponse { requests: route_ids.into_iter().map(|(id, distance)| id.clone()).collect() }))
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
        .await?;

    Ok(stations)
}

pub async fn merge_routes(
    State(pool): State<sqlx::PgPool>,
    State(client): State<map_service::Client>,
    Json(r): Json<MergeRoutesRequest>
) -> Result<Json<MergeRoutesResponse>> {
    let mut tx = pool.begin().await.map_err(|e| ErrorResponse::new(format!("error starting transaction: {e}")))?;

    // 2. Get all stations in the trip in order
    let mut trip_stations: Vec<(i32, PgPoint)> = sqlx::query_as("
        SELECT s.id, s.coords
        FROM path p
        INNER JOIN station s ON p.station_id = s.id
        WHERE p.trip_id = $1
        ORDER BY p.index;
    ")
        .bind(&r.trip)
        .fetch_all(&mut *tx)
        .await?;

    if trip_stations.is_empty() {
        return Err(ErrorResponse::new("trip has no stations"));
    }

    for request in &r.requests {
        // 1. Get request's source and destination stations
        let (req_src_id, req_src_coords, req_dst_id, req_dst_coords) = get_request_stations(&pool, request).await?;

        // 3. Find closest station in trip to request source
        let (insert_src_idx, _) = trip_stations
            .iter()
            .enumerate()
            .min_by(|(_, (_, coords1)), (_, (_, coords2))| {
                distance(coords1, &req_src_coords).total_cmp(&distance(coords2, &req_src_coords))
            })
            .unwrap();
            
        trip_stations.insert(insert_src_idx + 1, (req_src_id, req_src_coords));
    
        // 4. Find closest station after source insertion point to request destination
        let (insert_dst_idx, _) = trip_stations[..trip_stations.len() - 1]
            .iter()
            .enumerate()
            .skip(insert_src_idx + 1)
            .min_by(|(_, (_, coords1)), (_, (_, coords2))| {
                distance(coords1, &req_dst_coords).total_cmp(&distance(coords2, &req_dst_coords))
            })
            .unwrap();

        trip_stations.insert(insert_dst_idx + 1, (req_dst_id, req_dst_coords));
    }

    // 6. Create new trip
    let new_trip_id: Uuid = sqlx::query_scalar("
        INSERT INTO trip (id, source, destination)
        VALUES (gen_random_uuid(), $1, $2)
        RETURNING id;
    ")
        .bind(trip_stations[0].0)
        .bind(trip_stations[trip_stations.len() - 1].0)
        .fetch_one(&mut *tx)
        .await?;
        

    // 7. Insert path entries
    for (index, station) in trip_stations.iter().enumerate() {
        sqlx::query("
            INSERT INTO path (trip_id, station_id, index)
            VALUES ($1, $2, $3);
        ")
            .bind(&new_trip_id)
            .bind(station.0)
            .bind(index as i32)
            .execute(&mut *tx)
            .await?;
    }

    // 8. Create segments between consecutive stations
    for i in 0..trip_stations.len() - 1 {
        let s1 = trip_stations[i].0;
        let s2 = trip_stations[i + 1].0;

        // Check if segment already exists
        let existing: Option<(i32, i32)> = sqlx::query_as("
            SELECT distance, time FROM segment WHERE s1 = $1 AND s2 = $2;
        ")
            .bind(s1)
            .bind(s2)
            .fetch_optional(&mut *tx)
            .await?;
            

        if existing.is_none() {
            // Get coordinates for both stations
            let coords: Vec<PgPoint> = sqlx::query_scalar("
                SELECT coords FROM station WHERE id = ANY($1);
            ")
                .bind(&[s1, s2])
                .fetch_all(&mut *tx)
                .await?;

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
                .await?;
        }
    }

    // 9. Update request to reference the new trip
    for request in &r.requests {
        sqlx::query("
            UPDATE request SET trip_id = $1 WHERE id = $2;
        ")
            .bind(&new_trip_id)
            .bind(request)
            .execute(&mut *tx)
            .await?;
    }

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
            .await?;

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
        .await?;

    // Remove deleted stations from path
    path.retain(|(station_id, _)| !stations_to_delete.contains(station_id));

    if path.len() < 2 {
        return Err(ErrorResponse::new("cannot delete all stations from trip"));
    }

    // Delete old path entries
    sqlx::query("DELETE FROM path WHERE trip_id = $1;")
        .bind(&r.trip)
        .execute(&mut *tx)
        .await?;

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
            .await?;
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
        .await?;

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
            .await?;

        if existing.is_none() {
            // Get coordinates for both stations
            let coords: Vec<PgPoint> = sqlx::query_scalar("
                SELECT coords FROM station WHERE id = ANY($1) ORDER BY id = $2 DESC;
            ")
                .bind(&[s1, s2])
                .bind(s1)
                .fetch_all(&mut *tx)
                .await?;

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
                .await?;
        }
    }

    // Unlink removed requests from the trip
    for req_id in &r.delete_stations {
        sqlx::query("
            UPDATE request SET trip_id = NULL WHERE id = $1;
        ")
            .bind(req_id)
            .execute(&mut *tx)
            .await?;
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
        .await?;

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