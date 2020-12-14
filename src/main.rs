use crate::config::Config;
use crate::devices::DeviceState;
use crate::mqtt::mqtt_stream;
use crate::topic::Topic;
use color_eyre::{eyre::WrapErr, Report, Result};
use dashmap::DashMap;
use pin_utils::pin_mut;
use rumqttc::{MqttOptions, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::stream::StreamExt;
use warp::hyper::http::uri::Authority;
use warp::Filter;
use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response};

mod config;
mod devices;
mod mqtt;
mod topic;

type DeviceStates = Arc<DashMap<String, DeviceState>>;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let host_port = config.host_port;

    let device_states = DeviceStates::default();

    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let states = device_states.clone();
    let mqtt_host = config.mqtt_host;
    let mqtt_port = config.mqtt_port;
    tokio::task::spawn(async move {
        loop {
            if let Err(e) = mqtt_client(&mqtt_host, mqtt_port, states.clone()).await {
                eprintln!("lost mqtt collection: {:#}", e);
            }
            eprintln!("reconnecting after 1s");
            tokio::time::delay_for(Duration::from_secs(1)).await;
        }
    });

    let state = warp::any().map(move || device_states.clone());

    let proxy = warp::any()
        .and(warp::filters::host::optional())
        .and(state)
        .and_then(
            move |host: Option<Authority>, states: DeviceStates| async move {
                let host = match host {
                    Some(host) => host,
                    None => return Err(warp::reject::not_found()),
                };
                let requested_device = host.as_str().split('.').next().unwrap();
                if let Some(state) = states.get(requested_device) {
                    if let Some(ip) = state.ip {
                        Ok((format!("http://{}", ip), String::new()))
                    } else {
                        Err(warp::reject::not_found())
                    }
                } else {
                    Err(warp::reject::not_found())
                }
            },
        )
        .untuple_one()
        .and(extract_request_data_filter())
        .and_then(proxy_to_and_forward_response);

    warp::serve(proxy).run(([0, 0, 0, 0], host_port)).await;
    Ok(())
}

async fn mqtt_client(host: &str, port: u16, device_states: DeviceStates) -> Result<()> {
    let hostname = hostname::get()?
        .into_string()
        .map_err(|_| Report::msg("invalid hostname"))?;
    let mut mqtt_options = MqttOptions::new(format!("taspromto-{}", hostname), host, port);
    mqtt_options.set_keep_alive(5);

    let (client, stream) = mqtt_stream(mqtt_options)
        .await
        .wrap_err("Failed to setup mqtt listener")?;

    pin_mut!(stream);

    while let Some(message) = stream.next().await {
        let message = message?;
        println!(
            "{} {}",
            message.topic,
            std::str::from_utf8(message.payload.as_ref()).unwrap_or_default()
        );
        let topic = Topic::from(message.topic.as_str());

        match topic {
            Topic::LWT(device) => {
                // on discovery, ask the device for it's ip and name
                let send_client = client.clone();
                tokio::task::spawn(async move {
                    if let Err(e) = send_client
                        .publish(
                            device.get_topic("cmnd", "IPADDRESS"),
                            QoS::AtMostOnce,
                            false,
                            "",
                        )
                        .await
                    {
                        eprintln!("Failed to ask for power state: {:#}", e);
                    }
                    if let Err(e) = send_client
                        .publish(
                            device.get_topic("cmnd", "DeviceName"),
                            QoS::AtMostOnce,
                            false,
                            "",
                        )
                        .await
                    {
                        eprintln!("Failed to ask for device name: {:#}", e);
                    }
                });
            }
            Topic::Result(device) => {
                let payload = std::str::from_utf8(message.payload.as_ref()).unwrap_or_default();
                if let Ok(json) = json::parse(payload) {
                    let mut device_state = device_states.entry(device.hostname).or_default();
                    device_state.update(json);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
