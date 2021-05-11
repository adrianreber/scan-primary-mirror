use serde_xml_rs::from_reader;
use std::cmp;

#[derive(Debug, Deserialize)]
struct Timestamp {
    #[serde(rename = "$value")]
    pub value: String,
}
#[derive(Debug, Deserialize)]
struct Data {
    pub timestamp: Timestamp,
}

#[derive(Debug, Deserialize)]
struct Project {
    pub data: Vec<Data>,
}

pub fn get_timestamp(xml: String) -> i64 {
    let project: Project = match from_reader(xml.as_bytes()) {
        Ok(p) => p,
        Err(_) => return 0,
    };

    let mut timestamp: i64 = -1;

    for d in project.data {
        let value = match d.timestamp.value.parse::<i64>() {
            Ok(v1) => v1,
            Err(_) => match d.timestamp.value.parse::<f64>() {
                Ok(v2) => v2 as i64,
                Err(_) => continue,
            },
        };

        timestamp = cmp::max(timestamp, value);
    }

    timestamp
}
