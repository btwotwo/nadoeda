use std::sync::OnceLock;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TelegramSettings {
    pub token: String
}

#[derive(Deserialize, Debug)]
pub struct AppSettings {
    pub telegram: TelegramSettings
}

impl AppSettings {
    fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("appsettings").required(true))
            .add_source(File::with_name("appsettings.local").required(false))
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        settings.try_deserialize()
    }

}

pub fn get() -> &'static AppSettings {
    static APPSETTINGS: OnceLock<AppSettings> = OnceLock::new();
    APPSETTINGS.get_or_init(|| {
        AppSettings::new().unwrap()
    })
}

