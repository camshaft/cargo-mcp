use cargo_metadata::{Error, Metadata as Metadata_, MetadataCommand};

pub struct Metadata {
    // TODO implement a cache with change monitoring
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Metadata {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_metadata(&self, workspace: &str) -> Result<Metadata_, Error> {
        MetadataCommand::new().current_dir(workspace).exec()
    }
}
