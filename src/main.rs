use crate::config::{Config, Listen};
use crate::devices::{Device, DeviceState};
use crate::mqtt::mqtt_stream;
use crate::topic::Topic;
use color_eyre::{eyre::WrapErr, Result};
use dashmap::DashMap;
use futures_util::future::{Either, FutureExt};
use futures_util::stream::StreamExt;
use pin_utils::pin_mut;
use rumqttc::{AsyncClient, QoS};
use std::fs::{remove_file, set_permissions};
use std::os::unix::prelude::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::signal;
use tokio::time::sleep;
use tokio_stream::wrappers::UnixListenerStream;
use warp::http::{HeaderMap, HeaderValue, Method};
use warp::hyper::body::Bytes;
use warp::hyper::http::uri::Authority;
use warp::path::FullPath;
use warp::Filter;
use warp_reverse_proxy::{
    extract_request_data_filter, proxy_to_and_forward_response, QueryParameters,
};

mod config;
mod devices;
mod mqtt;
mod topic;

type DeviceStates = Arc<DashMap<String, DeviceState>>;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let listen = config.listen.clone();
    let tasmota_credentails = config
        .tasmota_credentials
        .as_ref()
        .map(|auth| auth.auth_header());

    let device_states = DeviceStates::default();

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
        .and_then(
            move |proxy_address: String,
                  base_path: String,
                  uri: FullPath,
                  params: QueryParameters,
                  method: Method,
                  mut headers: HeaderMap,
                  body: Bytes| {
                if let Some(credentials) = tasmota_credentails.as_deref() {
                    headers.append("authorization", HeaderValue::from_str(credentials).unwrap());
                }
                proxy_to_and_forward_response(
                    proxy_address,
                    base_path,
                    uri,
                    params,
                    method,
                    headers,
                    body,
                )
            },
        );

    let cancel = async {
        signal::ctrl_c().await.ok();
    };

    let warp_server = warp::serve(proxy);
    let server = match listen {
        Listen::Tcp(host_port) => Either::Left(
            warp_server
                .bind_with_graceful_shutdown(([0, 0, 0, 0], host_port), cancel)
                .1,
        ),
        Listen::Unix(socket) => {
            remove_file(&socket).ok();

            let listener = UnixListener::bind(&socket)?;
            set_permissions(&socket, PermissionsExt::from_mode(0o666))?;
            let stream = UnixListenerStream::new(listener);
            Either::Right(
                warp_server
                    .serve_incoming_with_graceful_shutdown(stream, cancel)
                    .map(move |_| {
                        remove_file(&socket).ok();
                    }),
            )
        }
    };

    server.await;

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
            Topic::Lwt(device) => match payload {
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
