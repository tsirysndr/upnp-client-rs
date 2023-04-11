use std::time::Duration;

use crate::types::{Action, Argument, Container, Device, Item, Metadata, Service, TransportInfo};
use anyhow::{anyhow, Result};
use elementtree::Element;
use surf::{http::Method, Client, Config, Url};
use xml::reader::XmlEvent;
use xml::EventReader;

pub async fn parse_location(location: &str) -> Result<Device> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;
    let req = surf::Request::new(Method::Get, location.parse()?);
    let xml_root = client
        .recv_string(req)
        .await
        .map_err(|e| anyhow!("Failed to retrieve xml from device endpoint: {}", e))?;

    let mut device = Device {
        location: location.to_string(),
        ..Default::default()
    };

    device.device_type = parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}deviceType",
    )?;

    device.device_type = parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}deviceType",
    )?;
    device.friendly_name = parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}friendlyName",
    )?;
    device.manufacturer = parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}manufacturer",
    )?;
    device.manufacturer_url = match parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}manufacturerURL",
    )? {
        url if url.is_empty() => None,
        url => Some(url),
    };
    device.model_description = match parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}modelDescription",
    )? {
        description if description.is_empty() => None,
        description => Some(description),
    };
    device.model_name = parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}modelName",
    )?;
    device.model_number = match parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}modelNumber",
    )? {
        number if number.is_empty() => None,
        number => Some(number),
    };
    device.udn = parse_attribute(
        &xml_root,
        "{urn:schemas-upnp-org:device-1-0}device/{urn:schemas-upnp-org:device-1-0}UDN",
    )?;

    let base_url = location.split('/').take(3).collect::<Vec<&str>>().join("/");
    device.services = parse_services(&base_url, &xml_root).await?;

    Ok(device)
}

fn parse_attribute(xml_root: &str, xml_name: &str) -> Result<String> {
    let root = Element::from_reader(xml_root.as_bytes())?;
    let mut xml_name = xml_name.split('/');
    match root.find(
        xml_name
            .next()
            .ok_or_else(|| anyhow!("xml_name ended unexpectedly"))?,
    ) {
        Some(element) => {
            let element = element.find(
                xml_name
                    .next()
                    .ok_or_else(|| anyhow!("xml_name ended unexpectedly"))?,
            );
            match element {
                Some(element) => {
                    return Ok(element.text().to_string());
                }
                None => Ok("".to_string()),
            }
        }
        None => Ok("".to_string()),
    }
}

pub async fn parse_services(base_url: &str, xml_root: &str) -> Result<Vec<Service>> {
    let root = Element::from_reader(xml_root.as_bytes())?;
    let device = root
        .find("{urn:schemas-upnp-org:device-1-0}device")
        .ok_or_else(|| anyhow!("Invalid response from device"))?;

    let mut services_with_actions: Vec<Service> = vec![];
    if let Some(service_list) = device.find("{urn:schemas-upnp-org:device-1-0}serviceList") {
        let xml_services = service_list.children();

        let mut services = Vec::new();
        for xml_service in xml_services {
            let mut service = Service {
                service_type: xml_service
                    .find("{urn:schemas-upnp-org:device-1-0}serviceType")
                    .ok_or_else(|| anyhow!("Service missing serviceType"))?
                    .text()
                    .to_string(),
                service_id: xml_service
                    .find("{urn:schemas-upnp-org:device-1-0}serviceId")
                    .ok_or_else(|| anyhow!("Service missing serviceId"))?
                    .text()
                    .to_string(),
                control_url: xml_service
                    .find("{urn:schemas-upnp-org:device-1-0}controlURL")
                    .ok_or_else(|| anyhow!("Service missing controlURL"))?
                    .text()
                    .to_string(),
                event_sub_url: xml_service
                    .find("{urn:schemas-upnp-org:device-1-0}eventSubURL")
                    .ok_or_else(|| anyhow!("Service missing eventSubURL"))?
                    .text()
                    .to_string(),
                scpd_url: xml_service
                    .find("{urn:schemas-upnp-org:device-1-0}SCPDURL")
                    .ok_or_else(|| anyhow!("Service missing SCPDURL"))?
                    .text()
                    .to_string(),
                actions: vec![],
            };

            service.control_url = build_absolute_url(base_url, &service.control_url)?;
            service.event_sub_url = build_absolute_url(base_url, &service.event_sub_url)?;
            service.scpd_url = build_absolute_url(base_url, &service.scpd_url)?;

            services.push(service);
        }

        for service in &services {
            let mut service = service.clone();
            service.actions = parse_service_description(&service.scpd_url).await?;
            services_with_actions.push(service);
        }
    }

    Ok(services_with_actions)
}

fn build_absolute_url(base_url: &str, relative_url: &str) -> Result<String> {
    let base_url = Url::parse(base_url)?;
    Ok(base_url.join(relative_url)?.to_string())
}

pub async fn parse_service_description(scpd_url: &str) -> Result<Vec<Action>> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;
    let req = surf::Request::new(Method::Get, scpd_url.parse()?);

    let xml_root = client
        .recv_string(req)
        .await
        .map_err(|e| anyhow!("Failed to retrieve xml response from device: {}", e))?;
    let root = Element::from_reader(xml_root.as_bytes())?;

    let action_list = match root.find("{urn:schemas-upnp-org:service-1-0}actionList") {
        Some(action_list) => action_list,
        None => return Ok(vec![]),
    };

    let mut actions = Vec::new();
    for xml_action in action_list.children() {
        let mut action = Action {
            name: xml_action
                .find("{urn:schemas-upnp-org:service-1-0}name")
                .ok_or_else(|| anyhow!("Service::Action missing name"))?
                .text()
                .to_string(),
            arguments: vec![],
        };

        if let Some(arguments) = xml_action.find("{urn:schemas-upnp-org:service-1-0}argumentList") {
            for xml_argument in arguments.children() {
                let argument = Argument {
                    name: xml_argument
                        .find("{urn:schemas-upnp-org:service-1-0}name")
                        .ok_or_else(|| anyhow!("Service::Action::Argument missing name"))?
                        .text()
                        .to_string(),
                    direction: xml_argument
                        .find("{urn:schemas-upnp-org:service-1-0}direction")
                        .ok_or_else(|| anyhow!("Service::Action::Argument missing direction"))?
                        .text()
                        .to_string(),
                    related_state_variable: xml_argument
                        .find("{urn:schemas-upnp-org:service-1-0}relatedStateVariable")
                        .ok_or_else(|| {
                            anyhow!("Service::Action::Argument missing relatedStateVariable")
                        })?
                        .text()
                        .to_string(),
                };
                action.arguments.push(argument);
            }
        }
        actions.push(action);
    }
    Ok(actions)
}

pub fn parse_volume(xml_root: &str) -> Result<u8> {
    let parser = EventReader::from_str(xml_root);
    let mut in_current_volume = false;
    let mut current_volume: Option<u8> = None;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "CurrentVolume" {
                    in_current_volume = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "CurrentVolume" {
                    in_current_volume = false;
                }
            }
            Ok(XmlEvent::Characters(volume)) => {
                if in_current_volume {
                    current_volume = Some(volume.parse()?);
                }
            }
            _ => {}
        }
    }
    current_volume.ok_or_else(|| anyhow!("Invalid response from device"))
}

pub fn parse_duration(xml_root: &str) -> Result<u32> {
    let parser = EventReader::from_str(xml_root);
    let mut in_duration = false;
    let mut duration: Option<String> = None;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "MediaDuration" {
                    in_duration = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "MediaDuration" {
                    in_duration = false;
                }
            }
            Ok(XmlEvent::Characters(duration_str)) => {
                if in_duration {
                    let duration_str = duration_str.replace(':', "");
                    duration = Some(duration_str);
                }
            }
            _ => {}
        }
    }

    let duration = duration.ok_or_else(|| anyhow!("Invalid response from device"))?;
    let hours = duration[0..2].parse::<u32>()?;
    let minutes = duration[2..4].parse::<u32>()?;
    let seconds = duration[4..6].parse::<u32>()?;
    Ok(hours * 3600 + minutes * 60 + seconds)
}

pub fn parse_position(xml_root: &str) -> Result<u32> {
    let parser = EventReader::from_str(xml_root);
    let mut in_position = false;
    let mut position: Option<String> = None;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "RelTime" {
                    in_position = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "RelTime" {
                    in_position = false;
                }
            }
            Ok(XmlEvent::Characters(position_str)) => {
                if in_position {
                    let position_str = position_str.replace(':', "");
                    position = Some(position_str);
                }
            }
            _ => {}
        }
    }

    let position = position.ok_or_else(|| anyhow!("Invalid response from device"))?;
    let hours = position[0..2].parse::<u32>()?;
    let minutes = position[2..4].parse::<u32>()?;
    let seconds = position[4..6].parse::<u32>()?;
    Ok(hours * 3600 + minutes * 60 + seconds)
}

pub fn parse_supported_protocols(xml_root: &str) -> Result<Vec<String>> {
    let parser = EventReader::from_str(xml_root);
    let mut in_protocol = false;
    let mut protocols: String = "".to_string();
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "Sink" {
                    in_protocol = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "Sink" {
                    in_protocol = false;
                }
            }
            Ok(XmlEvent::Characters(protocol)) => {
                if in_protocol {
                    protocols = protocol;
                }
            }
            _ => {}
        }
    }
    Ok(protocols.split(',').map(|s| s.to_string()).collect())
}

pub fn parse_last_change(xml_root: &str) -> Result<Option<String>> {
    let parser = EventReader::from_str(xml_root);
    let mut result = None;
    let mut in_last_change = false;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "LastChange" {
                    in_last_change = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "LastChange" {
                    in_last_change = false;
                }
            }
            Ok(XmlEvent::Characters(last_change)) => {
                if in_last_change {
                    result = Some(last_change);
                }
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn parse_current_play_mode(xml_root: &str) -> Result<Option<String>> {
    let parser = EventReader::from_str(xml_root);
    let mut current_play_mode: Option<String> = None;
    for e in parser.into_iter().flatten() {
        if let XmlEvent::StartElement {
            name, attributes, ..
        } = e
        {
            if name.local_name == "CurrentPlayMode" {
                for attr in attributes {
                    if attr.name.local_name == "val" {
                        current_play_mode = Some(attr.value);
                    }
                }
            }
        }
    }
    Ok(current_play_mode)
}

pub fn parse_transport_state(xml_root: &str) -> Result<Option<String>> {
    let parser = EventReader::from_str(xml_root);
    let mut transport_state: Option<String> = None;
    for e in parser.into_iter().flatten() {
        if let XmlEvent::StartElement {
            name, attributes, ..
        } = e
        {
            if name.local_name == "TransportState" {
                for attr in attributes {
                    if attr.name.local_name == "val" {
                        transport_state = Some(attr.value);
                    }
                }
            }
        }
    }
    Ok(transport_state)
}

pub fn parse_av_transport_uri_metadata(xml_root: &str) -> Result<Option<String>> {
    let parser = EventReader::from_str(xml_root);
    let mut av_transport_uri_metadata: Option<String> = None;
    for e in parser.into_iter().flatten() {
        if let XmlEvent::StartElement {
            name, attributes, ..
        } = e
        {
            if name.local_name == "AVTransportURIMetaData" {
                for attr in attributes {
                    if attr.name.local_name == "val" {
                        av_transport_uri_metadata = Some(attr.value);
                    }
                }
            }
        }
    }
    Ok(av_transport_uri_metadata)
}

pub fn parse_current_track_metadata(xml_root: &str) -> Result<Option<String>> {
    let parser = EventReader::from_str(xml_root);
    let mut current_track_metadata: Option<String> = None;
    for e in parser.into_iter().flatten() {
        if let XmlEvent::StartElement {
            name, attributes, ..
        } = e
        {
            if name.local_name == "CurrentTrackMetaData" {
                for attr in attributes {
                    if attr.name.local_name == "val" {
                        current_track_metadata = Some(attr.value);
                    }
                }
            }
        }
    }
    Ok(current_track_metadata)
}

pub fn deserialize_metadata(xml: &str) -> Result<Metadata> {
    let parser = EventReader::from_str(xml);
    let mut in_title = false;
    let mut in_artist = false;
    let mut in_album = false;
    let mut in_album_art = false;
    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut album: Option<String> = None;
    let mut album_art: Option<String> = None;
    let mut url: String = String::from("");

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name == "item" {
                    for attr in attributes {
                        if attr.name.local_name == "id" {
                            url = attr.value;
                        }
                    }
                }
                if name.local_name == "title" {
                    in_title = true;
                }
                if name.local_name == "artist" {
                    in_artist = true;
                }
                if name.local_name == "album" {
                    in_album = true;
                }
                if name.local_name == "albumArtURI" {
                    in_album_art = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "title" {
                    in_title = false;
                }
                if name.local_name == "artist" {
                    in_artist = false;
                }
                if name.local_name == "album" {
                    in_album = false;
                }
                if name.local_name == "albumArtURI" {
                    in_album_art = false;
                }
            }
            Ok(XmlEvent::Characters(value)) => {
                if in_title {
                    title = Some(value.clone());
                }
                if in_artist {
                    artist = Some(value.clone());
                }
                if in_album {
                    album = Some(value.clone());
                }
                if in_album_art {
                    album_art = Some(value.clone());
                }
            }
            _ => {}
        }
    }
    Ok(Metadata {
        title: title.unwrap_or_default(),
        artist,
        album,
        album_art_uri: album_art,
        url,
        ..Default::default()
    })
}

pub fn parse_browse_response(xml: &str, ip: &str) -> Result<(Vec<Container>, Vec<Item>)> {
    let parser = EventReader::from_str(xml);
    let mut in_result = false;
    let mut result: (Vec<Container>, Vec<Item>) = (Vec::new(), Vec::new());

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "Result" {
                    in_result = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "Result" {
                    in_result = false;
                }
            }
            Ok(XmlEvent::Characters(value)) => {
                if in_result {
                    result = deserialize_content_directory(&value, ip)?;
                }
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn deserialize_content_directory(xml: &str, ip: &str) -> Result<(Vec<Container>, Vec<Item>)> {
    let parser = EventReader::from_str(xml);
    let mut in_container = false;
    let mut in_item = false;
    let mut in_title = false;
    let mut in_artist = false;
    let mut in_album = false;
    let mut in_album_art = false;
    let mut in_genre = false;
    let mut in_class = false;
    let mut in_res = false;
    let mut containers: Vec<Container> = Vec::new();
    let mut items: Vec<Item> = Vec::new();

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name == "container" {
                    in_container = true;
                    let mut container = Container::default();
                    for attr in attributes.clone() {
                        if attr.name.local_name == "id" {
                            container.id = attr.value.clone();
                        }
                        if attr.name.local_name == "parentID" {
                            container.parent_id = attr.value.clone();
                        }
                    }
                    containers.push(container);
                }
                if name.local_name == "item" {
                    in_item = true;
                    let mut item = Item::default();
                    for attr in attributes.clone() {
                        if attr.name.local_name == "id" {
                            item.id = attr.value.clone();
                        }
                        if attr.name.local_name == "parentID" {
                            item.parent_id = attr.value.clone();
                        }
                    }
                    items.push(item);
                }
                if name.local_name == "title" {
                    in_title = true;
                }
                if name.local_name == "artist" {
                    in_artist = true;
                }
                if name.local_name == "album" {
                    in_album = true;
                }
                if name.local_name == "albumArtURI" {
                    in_album_art = true;
                }
                if name.local_name == "genre" {
                    in_genre = true;
                }
                if name.local_name == "class" {
                    in_class = true;
                }
                if name.local_name == "res" {
                    for attr in attributes {
                        if attr.name.local_name == "protocolInfo"
                            && (attr.value.clone().contains("audio")
                                || attr.value.clone().contains("video"))
                        {
                            items.last_mut().unwrap().protocol_info = attr.value.clone();
                        }
                        if attr.name.local_name == "size" {
                            items.last_mut().unwrap().size = Some(attr.value.parse::<u64>()?);
                        }
                        if attr.name.local_name == "duration" {
                            items.last_mut().unwrap().duration = Some(attr.value.clone());
                        }
                    }
                    in_res = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "container" {
                    in_container = false;
                }
                if name.local_name == "item" {
                    in_item = false;
                }
                if name.local_name == "title" {
                    in_title = false;
                }
                if name.local_name == "artist" {
                    in_artist = false;
                }
                if name.local_name == "album" {
                    in_album = false;
                }
                if name.local_name == "albumArtURI" {
                    in_album_art = false;
                }
                if name.local_name == "genre" {
                    in_genre = false;
                }
                if name.local_name == "class" {
                    in_class = false;
                }
                if name.local_name == "res" {
                    in_res = false;
                }
            }
            Ok(XmlEvent::Characters(value)) => {
                if in_container {
                    if let Some(container) = containers.last_mut() {
                        if in_title {
                            container.title = value.clone();
                        }
                        if in_class {
                            container.object_class = Some(value.as_str().into());
                        }
                    }
                }
                if in_item {
                    if let Some(item) = items.last_mut() {
                        if in_title {
                            item.title = value.clone();
                        }
                        if in_artist {
                            item.artist = Some(value.clone());
                        }
                        if in_album {
                            item.album = Some(value.clone());
                        }
                        if in_album_art {
                            item.album_art_uri = Some(value.clone());
                        }
                        if in_genre {
                            item.genre = Some(value.clone());
                        }
                        if in_class {
                            item.object_class = Some(value.clone().as_str().into());
                        }
                        if in_res
                            && item.url.is_empty()
                            && value.contains(ip)
                            && (item.protocol_info.contains("audio")
                                || item.protocol_info.contains("video"))
                        {
                            item.url = value.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok((containers, items))
}

pub fn parse_transport_info(xml: &str) -> Result<TransportInfo> {
    let parser = EventReader::from_str(xml);
    let mut in_transport_state = false;
    let mut in_transport_status = false;
    let mut in_transport_play_speed = false;
    let mut transport_info = TransportInfo::default();

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "CurrentTransportState" {
                    in_transport_state = true;
                }
                if name.local_name == "CurrentTransportStatus" {
                    in_transport_status = true;
                }
                if name.local_name == "CurrentSpeed" {
                    in_transport_play_speed = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "CurrentTransportState" {
                    in_transport_state = false;
                }
                if name.local_name == "CurrentTransportStatus" {
                    in_transport_status = false;
                }
                if name.local_name == "CurrentSpeed" {
                    in_transport_play_speed = false;
                }
            }
            Ok(XmlEvent::Characters(value)) => {
                if in_transport_state {
                    transport_info.current_transport_state = value.clone();
                }
                if in_transport_status {
                    transport_info.current_transport_status = value.clone();
                }
                if in_transport_play_speed {
                    transport_info.current_speed = value.clone();
                }
            }
            _ => {}
        }
    }
    Ok(transport_info)
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_services;

    #[tokio::test]
    async fn test_parsing_device_without_service_list() {
        const XML_ROOT: &'static str = r#"<?xml version="1.0" encoding="UTF-8"?>
        <root xmlns="urn:schemas-upnp-org:device-1-0">
            <specVersion>
                <major>1</major>
                <minor>0</minor>
            </specVersion>
            <device>
                <deviceType>urn:schemas-upnp-org:device:WLANAccessPointDevice:1</deviceType>
                <friendlyName>NETGEAR47B64C</friendlyName>
                <manufacturer>NETGEAR</manufacturer>
                <manufacturerURL>https://www.netgear.com</manufacturerURL>
                <modelDescription>NETGEAR Dual Band Access Point</modelDescription>
                <modelName>WAX214</modelName>
                <modelNumber>WAX214</modelNumber>
                <modelURL>https://www.netgear.com</modelURL>
                <firmwareVersion>2.1.1.3</firmwareVersion>
                <insightMode>0</insightMode>
                <serialNumber>XXXXXXXXX</serialNumber>
                <UDN>uuid:919ba4ec-ec93-490f-b0e3-80CC9C47B64C</UDN>
                <presentationURL>http://xxxxxx:1337/</presentationURL>
            </device>
        </root>"#;

        let result = parse_services("http://xxxxxx:1337/", XML_ROOT)
            .await
            .unwrap();
        assert_eq!(result.len(), 0);
    }
}
