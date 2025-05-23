# Moved to https://codeberg.org/icewind/tasproxy

# Tasproxy

Auto-discovery reverse proxy for [tasmota](https://tasmota.github.io/docs/)

## Why

Remembering what ip addresses all of your tasmota devices is a pain.

## Setup

Ensure your tasmota devices are connected to an MQTT server with the following "Full Topic":

    %prefix%/%topic%/

And the "Topic" set to the desired name for the device.

Run the binary with the following environment variables

- `MQTT_HOSTNAME`: hostname of the MQTT server to connect to
- `MQTT_PORT`: port of the mqtt server to connect to, defaults to 1883
- `MQTT_USERNAME`: username to authenticate against the mqtt server
- `MQTT_PASSWORD`: password to authenticate against the mqtt server
- `PORT`: port this binary MQTT listen on, defaults to 80

You can also configure the proxy to send HTTP Basic authentication to the tasmota devices by setting the `TASMOTA_USERNAME` and `TASMOTA_PASSWORD` environment variables.

Setup dns/hosts/etc to point `*.example.com` to the server running this binary

## Usage

The proxy server will use MQTT to discover and gather the ip addresses of your tasmota devices.

Any request made to `%hostname%.example.com` will be proxied to the tasmota device with the corresponding topic.
