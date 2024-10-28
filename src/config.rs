use base64::prelude::*;
use color_eyre::{eyre::WrapErr, Report, Result};
use rumqttc::MqttOptions;
use serde::Deserialize;
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub listen: Listen,
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub tasmota: TasmotaConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MqttConfig {
    #[serde(rename = "hostname")]
    pub host: String,
    #[serde(default = "default_mqtt_port")]
    pub port: u16,
    #[serde(flatten)]
    pub credentials: Option<Credentials>,
}

fn default_mqtt_port() -> u16 {
    1883
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct TasmotaConfig {
    #[serde(flatten)]
    pub credentials: Option<Credentials>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Listen {
    Tcp {
        #[serde(default = "default_address")]
        address: IpAddr,
        port: u16,
    },
    Unix {
        socket: String,
    },
}

fn default_address() -> IpAddr {
    Ipv4Addr::UNSPECIFIED.into()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Credentials {
    Raw {
        username: String,
        password: String,
    },
    File {
        username: String,
        password_file: String,
    },
}

impl Credentials {
    pub fn username(&self) -> String {
        match self {
            Credentials::Raw { username, .. } => username.clone(),
            Credentials::File { username, .. } => username.clone(),
        }
    }
    pub fn password(&self) -> String {
        match self {
            Credentials::Raw { password, .. } => password.clone(),
            Credentials::File { password_file, .. } => secretfile::load(password_file).unwrap(),
        }
    }

    pub fn auth_header(&self) -> String {
        let mut header = "Basic ".to_string();
        BASE64_STANDARD.encode_string(
            format!("{}:{}", self.username(), self.password()),
            &mut header,
        );
        header
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
        let raw = read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn from_env() -> Result<Self> {
        let mqtt_host = dotenv::var("MQTT_HOSTNAME").wrap_err("MQTT_HOSTNAME not set")?;
        let mqtt_port = dotenv::var("MQTT_PORT")
            .ok()
            .and_then(|port| u16::from_str(&port).ok())
            .unwrap_or(1883);
        let listen = match dotenv::var("SOCKET") {
            Ok(socket) => Listen::Unix { socket },
            _ => {
                let port = dotenv::var("PORT")
                    .ok()
                    .and_then(|port| u16::from_str(&port).ok())
                    .unwrap_or(80);
                Listen::Tcp {
                    address: default_address(),
                    port,
                }
            }
        };

        let mqtt_credentials = match dotenv::var("MQTT_USERNAME") {
            Ok(username) => {
                let password = dotenv::var("MQTT_PASSWORD")
                    .wrap_err("MQTT_USERNAME set, but MQTT_PASSWORD not set")?;
                Some(Credentials::Raw { username, password })
            }
            Err(_) => None,
        };

        let tasmota_credentials = match dotenv::var("TASMOTA_USERNAME") {
            Ok(username) => {
                let password = dotenv::var("TASMOTA_PASSWORD")
                    .wrap_err("TASMOTA_USERNAME set, but TASMOTA_PASSWORD not set")?;
                Some(Credentials::Raw { username, password })
            }
            Err(_) => None,
        };

        Ok(Config {
            mqtt: MqttConfig {
                host: mqtt_host,
                port: mqtt_port,
                credentials: mqtt_credentials,
            },
            tasmota: TasmotaConfig {
                credentials: tasmota_credentials,
            },
            listen,
        })
    }

    pub fn mqtt(&self) -> Result<MqttOptions> {
        let hostname = hostname::get()?
            .into_string()
            .map_err(|_| Report::msg("invalid hostname"))?;
        let mut mqtt_options = MqttOptions::new(
            format!("tasproxy-{}", hostname),
            &self.mqtt.host,
            self.mqtt.port,
        );
        if let Some(credentials) = self.mqtt.credentials.as_ref() {
            mqtt_options.set_credentials(credentials.username(), credentials.password());
        }
        mqtt_options.set_keep_alive(Duration::from_secs(5));
        Ok(mqtt_options)
    }
}
