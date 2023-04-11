use std::{collections::HashMap, sync::mpsc};

use anyhow::{Error, Ok};
use async_stream::stream;
use futures_util::Stream;
use xml_builder::{XMLBuilder, XMLElement};

use crate::{
    device_client::DeviceClient,
    parser::{
        parse_duration, parse_position, parse_supported_protocols, parse_transport_info,
        parse_volume,
    },
    types::{Event, LoadOptions, Metadata, ObjectClass, TransportInfo},
    BROADCAST_EVENT,
};

pub enum MediaEvents {
    Status,
    Loading,
    Playing,
    Paused,
    Stopped,
    SpeedChanged,
}

#[derive(Clone)]
pub struct MediaRendererClient {
    device_client: DeviceClient,
}

impl MediaRendererClient {
    pub fn new(device_client: DeviceClient) -> Self {
        Self { device_client }
    }
    pub async fn load(&self, url: &str, options: LoadOptions) -> Result<(), Error> {
        let dlna_features = options.dlna_features.unwrap_or("*".to_string());
        let content_type = options.content_type.unwrap_or("video/mpeg".to_string());
        let protocol_info = format!("http-get:*:{}:{}", content_type, dlna_features);
        let title = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .title;
        let artist = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .artist;
        let album = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .album;
        let album_art_uri = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .album_art_uri;
        let genre = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .genre;

        let m = Metadata {
            url: url.to_string(),
            title,
            artist,
            album,
            album_art_uri,
            genre,
            protocol_info,
        };

        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        params.insert("CurrentURI".to_string(), url.to_string());
        params.insert(
            "CurrentURIMetaData".to_string(),
            build_metadata(m, options.object_class.unwrap_or(ObjectClass::Video)),
        );
        self.device_client
            .call_action("AVTransport", "SetAVTransportURI", params)
            .await?;

        if options.autoplay {
            self.play().await?;
        }

        Ok(())
    }

    pub async fn play(&self) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        params.insert("Speed".to_string(), "1".to_string());
        self.device_client
            .call_action("AVTransport", "Play", params)
            .await?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        self.device_client
            .call_action("AVTransport", "Pause", params)
            .await?;
        Ok(())
    }

    pub async fn seek(&self, seconds: u64) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        params.insert("Unit".to_string(), "REL_TIME".to_string());
        params.insert("Target".to_string(), format_time(seconds));
        self.device_client
            .call_action("AVTransport", "Seek", params)
            .await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        self.device_client
            .call_action("AVTransport", "Stop", params)
            .await?;
        Ok(())
    }

    pub async fn next(&self) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        self.device_client
            .call_action("AVTransport", "Next", params)
            .await?;
        Ok(())
    }

    pub async fn previous(&self) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        self.device_client
            .call_action("AVTransport", "Previous", params)
            .await?;
        Ok(())
    }

    pub async fn set_next(&self, url: &str, options: LoadOptions) -> Result<(), Error> {
        let dlna_features = options.dlna_features.unwrap_or("*".to_string());
        let content_type = options.content_type.unwrap_or("video/mpeg".to_string());
        let protocol_info = format!("http-get:*:{}:{}", content_type, dlna_features);
        let title = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .title;
        let artist = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .artist;
        let album = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .album;
        let album_art_uri = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .album_art_uri;
        let genre = options
            .metadata
            .clone()
            .unwrap_or(Metadata::default())
            .genre;

        let m = Metadata {
            url: url.to_string(),
            title,
            artist,
            protocol_info,
            album,
            album_art_uri,
            genre,
        };

        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        params.insert("NextURI".to_string(), url.to_string());
        params.insert(
            "NextURIMetaData".to_string(),
            build_metadata(m, options.object_class.unwrap_or(ObjectClass::Video)),
        );
        self.device_client
            .call_action("AVTransport", "SetNextAVTransportURI", params)
            .await?;
        Ok(())
    }

    pub async fn get_volume(&self) -> Result<u8, Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        params.insert("Channel".to_string(), "Master".to_string());

        let response = self
            .device_client
            .call_action("RenderingControl", "GetVolume", params)
            .await?;

        Ok(parse_volume(response.as_str())?)
    }

    pub async fn set_volume(&self, volume: u32) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        params.insert("Channel".to_string(), "Master".to_string());
        params.insert("DesiredVolume".to_string(), volume.to_string());
        self.device_client
            .call_action("RenderingControl", "SetVolume", params)
            .await?;
        Ok(())
    }

    pub async fn get_supported_protocols(&self) -> Result<Vec<String>, Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        let response = self
            .device_client
            .call_action("ConnectionManager", "GetProtocolInfo", params)
            .await?;
        Ok(parse_supported_protocols(response.as_str())?)
    }

    pub async fn get_position(&self) -> Result<u32, Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        let response = self
            .device_client
            .call_action("AVTransport", "GetPositionInfo", params)
            .await?;
        Ok(parse_position(response.as_str())?)
    }

    pub async fn get_duration(&self) -> Result<u32, Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        let response = self
            .device_client
            .call_action("AVTransport", "GetMediaInfo", params)
            .await?;
        Ok(parse_duration(response.as_str())?)
    }

    pub async fn subscribe(&mut self) -> impl Stream<Item = Event> {
        let (tx, rx) = mpsc::channel();
        *BROADCAST_EVENT.lock().unwrap() = Some(tx);

        self.device_client.subscribe("AVTransport").await.unwrap();
        stream! {
            while let Some(event) = rx.recv().into_iter().next() {
                yield event;
            }
        }
    }

    pub async fn get_transport_info(&self) -> Result<TransportInfo, Error> {
        let mut params = HashMap::new();
        params.insert("InstanceID".to_string(), "0".to_string());
        let response = self
            .device_client
            .call_action("AVTransport", "GetTransportInfo", params)
            .await?;
        Ok(parse_transport_info(response.as_str())?)
    }
}

fn build_metadata(m: Metadata, media_type: ObjectClass) -> String {
    let mut didl = XMLElement::new("DIDL-Lite");
    didl.add_attribute("xmlns", "urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/");
    didl.add_attribute("xmlns:dc", "http://purl.org/dc/elements/1.1/");
    didl.add_attribute("xmlns:upnp", "urn:schemas-upnp-org:metadata-1-0/upnp/");
    didl.add_attribute("xmlns:dlna", "urn:schemas-dlna-org:metadata-1-0/");
    didl.add_attribute("xmlns:xbmc", "urn:schemas-xbmc-org:metadata-1-0/");
    didl.add_attribute("xmlns:sec", "http://www.sec.co.kr/");

    let mut item = XMLElement::new("item");
    item.add_attribute("id", "0");
    item.add_attribute("parentID", "-1");
    item.add_attribute("restricted", "false");

    let mut title = XMLElement::new("dc:title");
    title.add_text(m.title).unwrap();
    item.add_child(title).unwrap();

    let mut class = XMLElement::new("upnp:class");
    class.add_text(media_type.value().to_owned()).unwrap();
    item.add_child(class).unwrap();

    if let Some(value) = m.artist {
        let mut artist = XMLElement::new("upnp:artist");
        artist.add_text(value).unwrap();
        item.add_child(artist).unwrap();
    }

    if let Some(value) = m.album {
        let mut album = XMLElement::new("upnp:album");
        album.add_text(value).unwrap();
        item.add_child(album).unwrap();
    }

    if let Some(value) = m.album_art_uri {
        let mut album_art = XMLElement::new("upnp:albumArtURI");
        album_art.add_attribute("dlna:profileID", "JPEG_TN");
        album_art.add_attribute("xmlns:dlna", "urn:schemas-dlna-org:metadata-1-0/");
        album_art.add_text(value).unwrap();
        item.add_child(album_art).unwrap();
    }

    if let Some(value) = m.genre {
        let mut genre = XMLElement::new("upnp:genre");
        genre.add_text(value).unwrap();
        item.add_child(genre).unwrap();
    }

    let mut res = XMLElement::new("res");
    res.add_attribute("protocolInfo", m.protocol_info.as_str());
    res.add_text(m.url).unwrap();
    item.add_child(res).unwrap();

    didl.add_child(item).unwrap();

    let mut xml = XMLBuilder::new().build();
    xml.set_root_element(didl);

    let mut writer: Vec<u8> = Vec::new();
    xml.generate(&mut writer).unwrap();
    let metadata = String::from_utf8(writer)
        .unwrap()
        .replace(r#"<?xml version="1.0" encoding="UTF-8"?>"#, "");
    xml::escape::escape_str_attribute(&metadata).to_string()
}

fn format_time(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
