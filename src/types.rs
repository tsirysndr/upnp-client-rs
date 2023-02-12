use std::fmt::Display;

use owo_colors::OwoColorize;
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
    Container,
}

impl From<&str> for ObjectClass {
    fn from(value: &str) -> Self {
        match value {
            "object.item.audioItem.musicTrack" => ObjectClass::Audio,
            "object.item.videoItem.movie" => ObjectClass::Video,
            "object.item.imageItem.photo" => ObjectClass::Image,
            "object.container" => ObjectClass::Container,
            _ => ObjectClass::Container,
        }
    }
}

impl ObjectClass {
    pub fn value(&self) -> &'static str {
        match self {
            ObjectClass::Audio => "object.item.audioItem.musicTrack",
            ObjectClass::Video => "object.item.videoItem.movie",
            ObjectClass::Image => "object.item.imageItem.photo",
            ObjectClass::Container => "object.container",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub url: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_art_uri: Option<String>,
    pub genre: Option<String>,
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

#[derive(Debug)]
pub enum AVTransportEvent {
    AVTransportURIMetaData {
        sid: String,
        url: String,
        title: String,
        artist: Option<String>,
        album: Option<String>,
        album_art_uri: Option<String>,
        genre: Option<String>,
    },
    CurrentPlayMode {
        sid: String,
        play_mode: String,
    },
    CurrentTrackMetadata {
        sid: String,
        url: String,
        title: String,
        artist: Option<String>,
        album: Option<String>,
        album_art_uri: Option<String>,
        genre: Option<String>,
    },
    TransportState {
        sid: String,
        transport_state: String,
    },
}

#[derive(Debug)]
pub enum Event {
    AVTransport(AVTransportEvent),
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::AVTransport(event) => match event {
                AVTransportEvent::AVTransportURIMetaData {
                    sid,
                    url,
                    title,
                    artist,
                    album,
                    album_art_uri,
                    genre,
                } => write!(
                    f,
                    "AVTransportEvent::AVTransportURIMetaData {{\n sid: {},\n url: {},\n title: {},\n artist: {:?},\n album: {:?},\n album_art_uri: {:?},\n genre: {:?}\n }}",
                    sid.bright_green(), url.bright_green(), title.bright_green(), artist.bright_green(), album.bright_green(), album_art_uri.bright_green(), genre.bright_green()
                ),
                AVTransportEvent::CurrentPlayMode { sid, play_mode } => {
                    write!(f, "AVTransportEvent::CurrentPlayMode {{\n sid: {}, play_mode: {}\n }}", sid.bright_green(), play_mode.bright_green())
                }
                AVTransportEvent::CurrentTrackMetadata {
                    sid,
                    url,
                    title,
                    artist,
                    album,
                    album_art_uri,
                    genre,
                } => write!(
                    f,
                    "AVTransportEvent::CurrentTrackMetadata {{\n sid: {},\n url: {},\n title: {},\n artist: {:?},\n album: {:?},\n album_art_uri: {:?},\n genre: {:?}\n }}",
                    sid.bright_green(), url.bright_green(), title.bright_green(), artist.bright_green(), album.bright_green(), album_art_uri.bright_green(), genre.bright_green()
                ),
                AVTransportEvent::TransportState {
                    sid,
                    transport_state,
                } => write!(
                    f,
                    "AVTransportEvent::TransportState {{\n sid: {},\n transport_state: {}\n }}",
                    sid.bright_green(), transport_state.bright_green()
                ),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Container {
    pub id: String,
    pub parent_id: String,
    pub title: String,
    pub creator: Option<String>,
    pub restricted: bool,
    pub searchable: bool,
    pub child_count: Option<u32>,
    pub album_art_uri: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub original_track_number: Option<u32>,
    pub protocol_info: Option<String>,
    pub url: Option<String>,
    pub object_class: Option<ObjectClass>,
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub id: String,
    pub parent_id: String,
    pub title: String,
    pub creator: Option<String>,
    pub restricted: bool,
    pub searchable: bool,
    pub album_art_uri: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub original_track_number: Option<u32>,
    pub protocol_info: String,
    pub url: String,
    pub size: Option<u64>,
    pub duration: Option<String>,
    pub object_class: Option<ObjectClass>,
}

#[derive(Debug, Clone, Default)]
pub struct TransportInfo {
    pub current_transport_state: String,
    pub current_transport_status: String,
    pub current_speed: String,
}
