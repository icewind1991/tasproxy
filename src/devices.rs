use json::JsonValue;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Device {
    pub hostname: String,
}

impl Device {
    pub fn get_topic(&self, prefix: &str, command: &str) -> String {
        format!("{}/{}/{}", prefix, self.hostname, command)
    }
}

#[derive(Debug, Default)]
pub struct DeviceState {
    name: String,
    pub ip: Option<IpAddr>,
}

impl DeviceState {
    pub fn update(&mut self, json: JsonValue) {
        if json["DeviceName"].is_string() && !json["DeviceName"].is_empty() {
            self.name = json["DeviceName"].to_string();
        }
        if !json["IPAddress1"].is_empty() {
            let result = json["IPAddress1"].to_string();
            if let Some(Ok(ip)) = result
                .split(' ')
                .map(|part| part.trim_start_matches('(').trim_end_matches(')'))
                .rev()
                .map(IpAddr::from_str)
                .next()
            {
                self.ip = Some(ip);
            } else {
                eprintln!("malformed ipaddress result: {}", result);
            }
        }
    }
}
