use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Device {
    pub location: String,
    pub device_type: String,
    pub friendly_name: String,
    pub manufacturer: String,
    pub manufacturer_url: Option<String>,
    pub model_description: Option<String>,
    pub model_name: String,
    pub model_number: Option<String>,
    pub services: Vec<Service>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Service {
    pub service_type: String,
    pub service_id: String,
    pub control_url: String,
    pub event_sub_url: String,
    pub scpd_url: String,
}
