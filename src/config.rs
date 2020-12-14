use color_eyre::{eyre::WrapErr, Result};
use std::str::FromStr;

#[derive(Default)]
pub struct Config {
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub host_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mqtt_host = dotenv::var("MQTT_HOSTNAME").wrap_err("MQTT_HOSTNAME not set")?;
        let mqtt_port = dotenv::var("MQTT_PORT")
            .ok()
            .and_then(|port| u16::from_str(&port).ok())
            .unwrap_or(1883);
        let host_port = dotenv::var("PORT")
            .ok()
            .and_then(|port| u16::from_str(&port).ok())
            .unwrap_or(80);

        Ok(Config {
            mqtt_host,
            mqtt_port,
            host_port,
        })
    }
}
