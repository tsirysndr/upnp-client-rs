use anyhow::Error;
use async_stream::stream;
use elementtree::Element;
use futures_util::Stream;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str;
use std::thread::sleep;
use std::time::Duration;
use surf::http::Method;
use surf::{Client, Config};

use crate::types::{Device, Service};

const DISCOVERY_REQUEST: &str = "M-SEARCH * HTTP/1.1\r\n\
                                 HOST: 239.255.255.250:1900\r\n\
                                 MAN: \"ssdp:discover\"\r\n\
                                 MX: 2\r\n\
                                 ST: ssdp:all\r\n\
                                 \r\n";

pub fn discover_pnp_locations() -> impl Stream<Item = Device> {
    // Create a UDP socket
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();

    // Set the socket address to the multicast IP and port for UPnP device discovery
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 1900).into();

    // Join the UPnP multicast group
    socket
        .join_multicast_v4(
            &Ipv4Addr::new(239, 255, 255, 250),
            &Ipv4Addr::new(0, 0, 0, 0),
        )
        .unwrap();

    // Send the discovery request
    socket
        .send_to(DISCOVERY_REQUEST.as_bytes(), &socket_addr)
        .unwrap();

    stream! {
        loop {
          // Receive the discovery response
          let mut buf = [MaybeUninit::uninit(); 2048];
          let (size, _) = socket.recv_from(&mut buf).unwrap();
          // Convert the response to a string
          let response =
              str::from_utf8(unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, size) })
                  .unwrap();
          let headers = parse_raw_http_response(response).unwrap();
          let location = *headers.get("location").unwrap();
          yield parse_location(location).await.unwrap();
          sleep(Duration::from_millis(500));
      }
    }
}

fn parse_raw_http_response(response_str: &str) -> Result<HashMap<String, &str>, Error> {
    let mut headers = HashMap::new();

    match response_str.split("\r\n\r\n").next() {
        Some(header_str) => {
            for header_line in header_str.split("\r\n") {
                if let Some(colon_index) = header_line.find(':') {
                    let header_name = header_line[0..colon_index].to_ascii_lowercase();
                    let header_value = header_line[colon_index + 1..].trim();
                    headers.insert(header_name, header_value);
                }
            }
            Ok(headers)
        }
        None => Err(Error::msg("Invalid HTTP response")),
    }
}

async fn parse_location(location: &str) -> Result<Device, Error> {
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

    device.services = parse_services(&xml_root);

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

fn parse_services(xml_root: &str) -> Vec<Service> {
    let root = Element::from_reader(xml_root.as_bytes()).unwrap();
    let device = root
        .find("{urn:schemas-upnp-org:device-1-0}device")
        .unwrap();
    let service_list = device.find("{urn:schemas-upnp-org:device-1-0}serviceList");
    let services = service_list.unwrap().children();

    services
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
        })
        .collect()
}
