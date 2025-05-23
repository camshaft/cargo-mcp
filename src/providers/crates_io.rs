// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crates_index::{Crate, Names, SparseIndex, Version};
use std::error::Error;

pub struct CratesIoProvider {
    index: SparseIndex,
}

impl CratesIoProvider {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            index: SparseIndex::new_cargo_default()?,
        })
    }

    pub async fn fetch(&self, name: &str) -> Result<Crate, Box<dyn Error>> {
        // Try local cache first
        if let Some(krate) = self.find_in_cache(name)? {
            return Ok(krate);
        }

        // Not found locally, try fetching
        if let Some(krate) = self.fetch_crate(name)? {
            return Ok(krate);
        }

        Err(format!("Crate {name} not found").into())
    }

    pub async fn fetch_latest_version(&self, name: &str) -> Result<String, Box<dyn Error>> {
        let krate = self.fetch(name).await?;

        // Filter out pre-release and yanked versions
        let latest = krate.highest_normal_version().ok_or("No versions found")?;

        Ok(latest.version().to_string())
    }

    pub async fn fetch_versions(&self, name: &str) -> Result<Vec<Version>, Box<dyn Error>> {
        let krate = self.fetch(name).await?;

        Ok(krate.versions().iter().cloned().collect())
    }

    pub async fn fetch_features(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let krate = self.fetch(name).await?;

        let version_info = if let Some(version) = version {
            krate
                .versions()
                .iter()
                .find(|v| v.version() == version)
                .ok_or_else(|| format!("Version {} not found", version))?
        } else {
            krate
                .versions()
                .iter()
                .max_by_key(|v| v.version())
                .ok_or("No versions found")?
        };

        Ok(version_info.features().keys().cloned().collect())
    }

    fn find_in_cache(&self, name: &str) -> Result<Option<Crate>, Box<dyn Error>> {
        for name in self.get_name_variants(name)? {
            if let Ok(krate) = self.index.crate_from_cache(&name) {
                return Ok(Some(krate));
            }
        }
        Ok(None)
    }

    fn fetch_crate(&self, name: &str) -> Result<Option<Crate>, Box<dyn Error>> {
        for name in self.get_name_variants(name)? {
            if let Some(krate) = self.update_cache(&name)? {
                return Ok(Some(krate));
            }
        }
        Ok(None)
    }

    fn get_name_variants(
        &self,
        name: &str,
    ) -> Result<impl Iterator<Item = String>, Box<dyn Error>> {
        Ok(Names::new(name)
            .ok_or_else(|| "Invalid crate name")?
            .take(3))
    }

    fn update_cache(&self, name: &str) -> Result<Option<Crate>, Box<dyn Error>> {
        let request = self
            .index
            .make_cache_request(name)?
            .version(ureq::http::Version::HTTP_11)
            .body(())?;

        let response = ureq::run(request)?;
        let (parts, mut body) = response.into_parts();
        let response = http::Response::from_parts(parts, body.read_to_vec()?);
        Ok(self.index.parse_cache_response(name, response, true)?)
    }
}
