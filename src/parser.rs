use std::time::Duration;

use crate::types::{Action, Argument, Device, Service};
use anyhow::Error;
use elementtree::Element;
use surf::{http::Method, Client, Config, Url};
use xml::reader::XmlEvent;
use xml::EventReader;

pub async fn parse_location(location: &str) -> Result<Device, Error> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()
        .unwrap();
    let req = surf::Request::new(Method::Get, location.parse().unwrap());
    let xml_root = client.recv_string(req).await.unwrap();

    let mut device: Device = Device::default();

    device.location = location.to_string();

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
    device.services = parse_services(&base_url, &xml_root).await;

    Ok(device)
}

fn parse_attribute(xml_root: &str, xml_name: &str) -> Result<String, Error> {
    let root = Element::from_reader(xml_root.as_bytes())?;
    let mut xml_name = xml_name.split('/');
    match root.find(xml_name.next().unwrap()) {
        Some(element) => {
            let element = element.find(xml_name.next().unwrap());
            match element {
                Some(element) => {
                    return Ok(element.text().to_string());
                }
                None => {
                    return Ok("".to_string());
                }
            }
        }
        None => Ok("".to_string()),
    }
}

pub async fn parse_services(base_url: &str, xml_root: &str) -> Vec<Service> {
    let root = Element::from_reader(xml_root.as_bytes()).unwrap();
    let device = root
        .find("{urn:schemas-upnp-org:device-1-0}device")
        .unwrap();
    let service_list = device.find("{urn:schemas-upnp-org:device-1-0}serviceList");
    let services = service_list.unwrap().children();

    let services: Vec<Service> = services
        .into_iter()
        .map(|item| Service {
            service_type: item
                .find("{urn:schemas-upnp-org:device-1-0}serviceType")
                .unwrap()
                .text()
                .to_string(),
            service_id: item
                .find("{urn:schemas-upnp-org:device-1-0}serviceId")
                .unwrap()
                .text()
                .to_string(),
            control_url: item
                .find("{urn:schemas-upnp-org:device-1-0}controlURL")
                .unwrap()
                .text()
                .to_string(),
            event_sub_url: item
                .find("{urn:schemas-upnp-org:device-1-0}eventSubURL")
                .unwrap()
                .text()
                .to_string(),
            scpd_url: item
                .find("{urn:schemas-upnp-org:device-1-0}SCPDURL")
                .unwrap()
                .text()
                .to_string(),
            actions: vec![],
        })
        .map(|mut service| {
            service.control_url = build_absolute_url(base_url, &service.control_url);
            service.event_sub_url = build_absolute_url(base_url, &service.event_sub_url);
            service.scpd_url = build_absolute_url(base_url, &service.scpd_url);
            service
        })
        .collect();
    let mut services_with_actions: Vec<Service> = vec![];
    for service in &services {
        let mut service = service.clone();
        service.actions = parse_service_description(&service.scpd_url).await;
        services_with_actions.push(service);
    }
    services_with_actions
}

fn build_absolute_url(base_url: &str, relative_url: &str) -> String {
    let base_url = Url::parse(base_url).unwrap();
    base_url.join(relative_url).unwrap().to_string()
}

pub async fn parse_service_description(scpd_url: &str) -> Vec<Action> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()
        .unwrap();
    let req = surf::Request::new(Method::Get, scpd_url.parse().unwrap());
    if let Ok(xml_root) = client.recv_string(req).await {
        if let Ok(root) = Element::from_reader(xml_root.as_bytes()) {
            let action_list = root.find("{urn:schemas-upnp-org:service-1-0}actionList");

            if action_list.is_none() {
                return vec![];
            }

            let action_list = action_list.unwrap().children();
            let actions: Vec<Action> = action_list
                .into_iter()
                .map(|item| {
                    let name = item
                        .find("{urn:schemas-upnp-org:service-1-0}name")
                        .unwrap()
                        .text();
                    let arguments = item.find("{urn:schemas-upnp-org:service-1-0}argumentList");
                    let arguments = arguments.unwrap().children();
                    let arguments = arguments.into_iter().map(|item| {
                        let name = item
                            .find("{urn:schemas-upnp-org:service-1-0}name")
                            .unwrap()
                            .text();
                        let direction = item
                            .find("{urn:schemas-upnp-org:service-1-0}direction")
                            .unwrap()
                            .text();
                        let related_state_variable = item
                            .find("{urn:schemas-upnp-org:service-1-0}relatedStateVariable")
                            .unwrap()
                            .text();
                        Argument {
                            name: name.to_string(),
                            direction: direction.to_string(),
                            related_state_variable: related_state_variable.to_string(),
                        }
                    });
                    Action {
                        name: name.to_string(),
                        arguments: arguments.collect(),
                    }
                })
                .collect();
            return actions;
        }
    }
    vec![]
}

pub fn parse_volume(xml_root: &str) -> Result<u8, Error> {
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
                    current_volume = Some(volume.parse().unwrap());
                }
            }
            _ => {}
        }
    }
    Ok(current_volume.unwrap())
}

pub fn parse_duration(xml_root: &str) -> Result<u32, Error> {
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
                    let duration_str = duration_str.replace(":", "");
                    duration = Some(duration_str);
                }
            }
            _ => {}
        }
    }

    let duration = duration.unwrap();
    let hours = duration[0..2].parse::<u32>().unwrap();
    let minutes = duration[2..4].parse::<u32>().unwrap();
    let seconds = duration[4..6].parse::<u32>().unwrap();
    Ok(hours * 3600 + minutes * 60 + seconds)
}

pub fn parse_position(xml_root: &str) -> Result<u32, Error> {
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
                    let position_str = position_str.replace(":", "");
                    position = Some(position_str);
                }
            }
            _ => {}
        }
    }

    let position = position.unwrap();
    let hours = position[0..2].parse::<u32>().unwrap();
    let minutes = position[2..4].parse::<u32>().unwrap();
    let seconds = position[4..6].parse::<u32>().unwrap();
    Ok(hours * 3600 + minutes * 60 + seconds)
}

pub fn parse_supported_protocols(xml_root: &str) -> Result<Vec<String>, Error> {
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
    Ok(protocols.split(",").map(|s| s.to_string()).collect())
}
