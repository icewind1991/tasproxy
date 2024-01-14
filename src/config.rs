use base64::prelude::*;
use color_eyre::{eyre::WrapErr, Report, Result};
use rumqttc::MqttOptions;
use std::str::FromStr;
use std::time::Duration;

pub struct Config {
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub mqtt_credentials: Option<Credentials>,
    pub listen: Listen,
    pub tasmota_credentials: Option<Credentials>,
}

#[derive(Clone)]
pub enum Listen {
    Tcp(u16),
    Unix(String),
}

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn auth_header(&self) -> String {
        let mut header = "Basic ".to_string();
        BASE64_STANDARD.encode_string(format!("{}:{}", self.username, self.password), &mut header);
        header
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mqtt_host = dotenv::var("MQTT_HOSTNAME").wrap_err("MQTT_HOSTNAME not set")?;
        let mqtt_port = dotenv::var("MQTT_PORT")
            .ok()
            .and_then(|port| u16::from_str(&port).ok())
            .unwrap_or(1883);
        let listen = match dotenv::var("SOCKET") {
            Ok(socket) => Listen::Unix(socket),
            _ => {
                let port = dotenv::var("PORT")
                    .ok()
                    .and_then(|port| u16::from_str(&port).ok())
                    .unwrap_or(80);
                Listen::Tcp(port)
            }
        };

        let mqtt_credentials = match dotenv::var("MQTT_USERNAME") {
            Ok(username) => {
                let password = dotenv::var("MQTT_PASSWORD")
                    .wrap_err("MQTT_USERNAME set, but MQTT_PASSWORD not set")?;
                Some(Credentials { username, password })
            }
            Err(_) => None,
        };

        let tasmota_credentials = match dotenv::var("TASMOTA_USERNAME") {
            Ok(username) => {
                let password = dotenv::var("TASMOTA_PASSWORD")
                    .wrap_err("TASMOTA_USERNAME set, but TASMOTA_PASSWORD not set")?;
                Some(Credentials { username, password })
            }
            Err(_) => None,
        };

        Ok(Config {
            mqtt_host,
            mqtt_port,
            mqtt_credentials,
            tasmota_credentials,
            listen,
        })
    }

    pub fn mqtt(&self) -> Result<MqttOptions> {
        let hostname = hostname::get()?
            .into_string()
            .map_err(|_| Report::msg("invalid hostname"))?;
        let mut mqtt_options = MqttOptions::new(
            format!("tasproxy-{}", hostname),
            &self.mqtt_host,
            self.mqtt_port,
        );
        if let Some(credentials) = self.mqtt_credentials.as_ref() {
            mqtt_options.set_credentials(&credentials.username, &credentials.password);
        }
        mqtt_options.set_keep_alive(Duration::from_secs(5));
        Ok(mqtt_options)
    }
}
