use anyhow::anyhow;
use axum::http::response;
use reqwest::Url;

use crate::api::types::Coord;

use super::types::*;

#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
    base: Url,
}

impl Client {
    pub fn new(base: &str) -> anyhow::Result<Self> {
        let client = reqwest::Client::new();
        let base = base
            .parse()
            .map_err(|e| anyhow!("{} is not a valid url: {}", base, e))?;

        Ok(Self {
            inner: client,
            base,
        })
    }

    pub async fn create_route(&self, r: CreateRouteRequest) -> anyhow::Result<CreateRouteResponse> {
        let url = self
            .base
            .join("/api/create_route")
            .map_err(|e| anyhow!("error joining url: {e}"))?;

        let response = self
            .inner
            .post(url)
            .json(&r)
            .send()
            .await?
            .json()
            .await
            .map(|CreateRouteResponse { way, graph }| CreateRouteResponse {
                way: way.into_iter().map(|[x, y]| [y, x]).collect(),
                graph,
            })?;

        Ok(response)
    }

    pub async fn find_station(&self, r: FindStationRequest) -> anyhow::Result<FindStationResponse> {
        let url = self
            .base
            .join("/api/find_stations")
            .map_err(|e| anyhow!("error joining url: {e}"))?;

        let response = self
            .inner
            .get(url)
            .query(&r)
            .send()
            .await?
            .json()
            .await
            .map(
                |FindStationResponse { address, coord }| FindStationResponse {
                    address,
                    coord: Coord {
                        lat: coord.lon,
                        lon: coord.lat,
                    },
                },
            )?;
        Ok(response)
    }
}
