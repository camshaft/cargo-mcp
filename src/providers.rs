use crate::Config;

pub mod crates_io;
mod link;
pub mod metadata;
pub mod rustdoc;

pub struct Providers {
    pub metadata: metadata::Metadata,
    pub crates_io: crates_io::CratesIoProvider,
    pub rustdoc: rustdoc::RustdocProvider,
}

impl Providers {
    pub fn new(_config: &Config) -> Self {
        let metadata = metadata::Metadata::new();
        let crates_io =
            crates_io::CratesIoProvider::new().expect("Failed to initialize crates.io provider");
        let rustdoc =
            rustdoc::RustdocProvider::new().expect("Failed to initialize rustdoc provider");
        Self {
            metadata,
            crates_io,
            rustdoc,
        }
    }
}
