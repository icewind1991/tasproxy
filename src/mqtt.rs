use async_stream::try_stream;
use color_eyre::Result;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, Publish, QoS};
use tokio::stream::{Stream, StreamExt};

pub async fn mqtt_stream(
    mqtt_options: MqttOptions,
) -> Result<(AsyncClient, impl Stream<Item = Result<Publish>>)> {
    let (client, event_loop) = AsyncClient::new(mqtt_options, 10);
    client.subscribe("tele/+/LWT", QoS::AtMostOnce).await?;
    client.subscribe("stat/+/RESULT", QoS::AtMostOnce).await?;

    let stream = event_loop_to_stream(event_loop).filter_map(|event| match event {
        Ok(Event::Incoming(Packet::Publish(message))) => Some(Ok(message)),
        Ok(_) => None,
        Err(e) => Some(Err(e)),
    });

    Ok((client, stream))
}

fn event_loop_to_stream(mut event_loop: EventLoop) -> impl Stream<Item = Result<Event>> {
    try_stream! {
        loop {
            let event = event_loop.poll().await?;
            yield event;
        }
    }
}
