use crate::devices::Device;

#[derive(Debug, Eq, PartialEq)]
pub enum Topic {
    LWT(Device),
    State(Device),
    Sensor(Device),
    Result(Device),
    Other(String),
}

impl From<&str> for Topic {
    fn from(raw: &str) -> Self {
        let mut parts = raw.split('/');
        if let (Some(prefix), Some(hostname), Some(cmd)) =
            (parts.next(), parts.next(), parts.next())
        {
            let device = Device {
                hostname: hostname.to_string(),
            };
            match (prefix, cmd) {
                ("tele", "LWT") => Topic::LWT(device),
                ("tele", "STATE") => Topic::State(device),
                ("tele", "SENSOR") => Topic::Sensor(device),
                ("stat", "RESULT") => Topic::Result(device),
                _ => Topic::Other(raw.to_string()),
            }
        } else {
            Topic::Other(raw.to_string())
        }
    }
}

#[test]
fn parse_topic() {
    let device = Device {
        hostname: "hostname".to_string(),
    };
    assert_eq!(Topic::LWT(device.clone()), Topic::from("tele/hostname/LWT"));
    assert_eq!(
        Topic::State(device.clone()),
        Topic::from("tele/hostname/STATE")
    );
    assert_eq!(
        Topic::Sensor(device.clone()),
        Topic::from("tele/hostname/SENSOR")
    );
    assert_eq!(
        Topic::Result(device.clone()),
        Topic::from("stat/hostname/RESULT")
    );
}
