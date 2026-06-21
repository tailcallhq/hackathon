// Just a reader.. nothing special here

use crate::config::Config;
use std::path::Path;

pub struct ConfigReader {}

impl ConfigReader {
    pub fn init() -> Self {
        Self {}
    }
    pub fn read<T: AsRef<Path>>(&self, path: T) -> anyhow::Result<Config> {
        let sdl = std::fs::read_to_string(path)?;
        let doc = async_graphql::parser::parse_schema(sdl)?;
        crate::from_doc::from_doc(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read() {
        let root = env!("CARGO_MANIFEST_DIR");
        let reader = ConfigReader::init();
        let config = reader
            .read(format!("{}/schema/schema.graphql", root))
            .unwrap();
        assert_eq!(config.types.len(), 5);
    }
}
