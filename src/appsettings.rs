use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppSettings {
    telegram_token: String
}

impl AppSettings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("config").required(true))
            .add_source(File::with_name("config.local").required(false))
            .add_source(Environment::with_prefix("APP"))
            .build()?;


        settings.try_deserialize()
    }
}
