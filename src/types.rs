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
    pub udn: String,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Service {
    pub service_type: String,
    pub service_id: String,
    pub control_url: String,
    pub event_sub_url: String,
    pub scpd_url: String,
    pub actions: Vec<Action>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Action {
    pub name: String,
    pub arguments: Vec<Argument>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Argument {
    pub name: String,
    pub direction: String,
    pub related_state_variable: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ObjectClass {
    Audio,
    Video,
    Image,
}

impl ObjectClass {
    pub fn value(&self) -> &'static str {
        match self {
            ObjectClass::Audio => "object.item.audioItem.musicTrack",
            ObjectClass::Video => "object.item.videoItem.movie",
            ObjectClass::Image => "object.item.imageItem.photo",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub url: String,
    pub title: String,
    pub artist: String,
    pub protocol_info: String,
}

#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    pub dlna_features: Option<String>,
    pub content_type: Option<String>,
    pub object_class: Option<ObjectClass>,
    pub metadata: Option<Metadata>,
    pub autoplay: bool,
}
