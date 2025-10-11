use std::sync::OnceLock;
use nadoeda_models::settings::Settings;

use config::{Config, ConfigError, Environment, File};

fn load_settings() -> Result<Settings, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("appsettings").required(true))
            .add_source(File::with_name("appsettings.local").required(false))
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        settings.try_deserialize()
}

pub fn get() -> &'static Settings {
    static APPSETTINGS: OnceLock<Settings> = OnceLock::new();
    APPSETTINGS.get_or_init(|| load_settings().unwrap())
}
