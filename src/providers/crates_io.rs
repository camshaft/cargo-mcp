// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crates_index::{Crate, IndexConfig, Names, SparseIndex, Version};
use std::sync::Arc;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct CratesIoProvider(Arc<State>);

struct State {
    index: SparseIndex,
    config: IndexConfig,
}

impl CratesIoProvider {
    pub fn new() -> Result<Self, Error> {
        let index = SparseIndex::new_cargo_default()?;
        let config = index.index_config()?;
        let state = State { index, config };
        Ok(Self(Arc::new(state)))
    }

    pub async fn fetch(&self, name: &str) -> Result<Crate, Error> {
        // Try local cache first
        if let Some(krate) = self.find_in_cache(name)? {
            return Ok(krate);
        }

        // Not found locally, try fetching
        if let Some(krate) = self.fetch_crate(name).await? {
            return Ok(krate);
        }

        Err(format!("Crate {name} not found").into())
    }

    pub async fn fetch_latest_version(&self, name: &str) -> Result<String, Error> {
        let krate = self.fetch(name).await?;

        // Filter out pre-release and yanked versions
        let latest = krate.highest_normal_version().ok_or("No versions found")?;

        Ok(latest.version().to_string())
    }

    pub async fn fetch_versions(&self, name: &str) -> Result<Vec<Version>, Error> {
        let krate = self.fetch(name).await?;

        Ok(krate.versions().iter().cloned().collect())
    }

    pub async fn fetch_features(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, Error> {
        let krate = self.fetch(name).await?;

        let version_info = if let Some(version) = version {
            krate
                .versions()
                .iter()
                .find(|v| v.version() == version)
                .ok_or_else(|| format!("Version {} not found", version))?
        } else {
            krate.highest_normal_version().ok_or("No versions found")?
        };

        Ok(version_info.features().keys().cloned().collect())
    }

    pub async fn get_download_url(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<String, Error> {
        let krate = self.fetch(name).await?;

        let version_info = if let Some(version) = version {
            krate
                .versions()
                .iter()
                .find(|v| v.version() == version)
                .ok_or_else(|| format!("Version {} not found", version))?
        } else {
            krate.highest_normal_version().ok_or("No versions found")?
        };

        Ok(version_info
            .download_url(&self.0.config)
            .ok_or("Could not create download url")?)
    }

    fn find_in_cache(&self, name: &str) -> Result<Option<Crate>, Error> {
        for name in self.get_name_variants(name)? {
            if let Ok(krate) = self.0.index.crate_from_cache(&name) {
                return Ok(Some(krate));
            }
        }
        Ok(None)
    }

    async fn fetch_crate(&self, name: &str) -> Result<Option<Crate>, Error> {
        for name in self.get_name_variants(name)? {
            if let Some(krate) = self.update_cache(&name).await? {
                return Ok(Some(krate));
            }
        }
        Ok(None)
    }

    fn get_name_variants(&self, name: &str) -> Result<impl Iterator<Item = String>, Error> {
        Ok(Names::new(name)
            .ok_or_else(|| "Invalid crate name")?
            .take(3))
    }

    async fn update_cache(&self, name: &str) -> Result<Option<Crate>, Error> {
        let request = self.0.index.make_cache_request(name)?;
        let url = request.uri_ref().unwrap().to_string();
        let response = reqwest::Client::new().get(url).send().await?;
        let bytes = response.bytes().await?;
        let response = http::Response::new(bytes.to_vec());
        Ok(self.0.index.parse_cache_response(name, response, true)?)
    }
}
