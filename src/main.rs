use crate::config::Config;
use crate::devices::{Device, DeviceState};
use crate::mqtt::mqtt_stream;
use crate::topic::Topic;
use color_eyre::{eyre::WrapErr, Result};
use dashmap::DashMap;
use futures_util::stream::StreamExt;
use pin_utils::pin_mut;
use rumqttc::{AsyncClient, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
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
    tokio::task::spawn(async move {
        loop {
            if let Err(e) = mqtt_client(&config, states.clone()).await {
                eprintln!("lost mqtt collection: {:#}", e);
            }
            eprintln!("reconnecting after 1s");
            tokio::time::sleep(Duration::from_secs(1)).await;
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
                if let Some(state) = states
                    .get(requested_device)
                    .or_else(|| states.get(&requested_device.replace('-', "_")))
                {
                    if let Some(ip) = state.ip {
                        println!("{} => {}", requested_device, ip);
                        Ok((format!("http://{}", ip), String::new()))
                    } else {
                        eprintln!("Error {} has no ip set", requested_device);
                        Err(warp::reject::not_found())
                    }
                } else {
                    eprintln!("Error {} has not been discovered", requested_device);
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

async fn mqtt_client(config: &Config, device_states: DeviceStates) -> Result<()> {
    let mqtt_options = config.mqtt()?;

    let (client, stream) = mqtt_stream(mqtt_options)
        .await
        .wrap_err("Failed to setup mqtt listener")?;

    pin_mut!(stream);

    while let Some(message) = stream.next().await {
        let message = message?;
        let payload = std::str::from_utf8(message.payload.as_ref()).unwrap_or_default();
        println!("{} {}", message.topic, payload);
        let topic = Topic::from(message.topic.as_str());

        match topic {
            Topic::LWT(device) => match payload {
                "Online" => {
                    println!("Discovered {}", device.hostname);
                    query_device(client.clone(), device).await;
                }
                "Offline" => {
                    println!("Removing {}", device.hostname);
                    device_states.remove(&device.hostname);
                }
                _ => {}
            },
            Topic::Result(device) => {
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

async fn query_device(client: AsyncClient, device: Device) {
    tokio::task::spawn(async move {
        // one device boot, the discovery event can happen before the device is ready to respond to our messages
        // thus we wait 5 seconds before asking

        sleep(Duration::from_secs(5)).await;

        if let Err(e) = client
            .publish(
                device.get_topic("cmnd", "IPADDRESS"),
                QoS::AtLeastOnce,
                false,
                "",
            )
            .await
        {
            eprintln!("Failed to ask for device IP: {:#}", e);
        }
        if let Err(e) = client
            .publish(
                device.get_topic("cmnd", "DeviceName"),
                QoS::AtLeastOnce,
                false,
                "",
            )
            .await
        {
            eprintln!("Failed to ask for device name: {:#}", e);
        }
    });
}
