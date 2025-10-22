use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TelegramSettings {
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub telegram: TelegramSettings,
}
