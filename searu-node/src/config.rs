use std::path::Path;

pub use config::{ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub etcd_addr: String,
    pub jwt_secret: String,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut config = config::Config::new();
        let config_paths = &[
            "./default.toml",
            "./config/default.toml",
            "./config.toml",
            "/etc/searu/config.toml",
        ];

        for path in config_paths {
            if Path::new(path).exists() {
                config.merge(File::with_name(path))?;
            }
        }

        config.try_into()
    }
}
