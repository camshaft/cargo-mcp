use crate::Config;

pub mod crates_io;
mod link;
pub mod metadata;

pub struct Providers {
    pub metadata: metadata::Metadata,
    pub crates_io: crates_io::CratesIoProvider,
}

impl Providers {
    pub fn new(_config: &Config) -> Self {
        let metadata = metadata::Metadata::new();
        let crates_io =
            crates_io::CratesIoProvider::new().expect("Failed to initialize crates.io provider");
        Self {
            metadata,
            crates_io,
        }
    }
}
