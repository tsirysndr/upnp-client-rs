use anyhow::{anyhow, Result};
use async_stream::stream;
use futures_util::Stream;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::str;
use tokio::net::UdpSocket;

use crate::parser::parse_location;
use crate::types::Device;

const DISCOVERY_REQUEST: &str = "M-SEARCH * HTTP/1.1\r\n\
                                 HOST: 239.255.255.250:1900\r\n\
                                 MAN: \"ssdp:discover\"\r\n\
                                 MX: 2\r\n\
                                 ST: ssdp:all\r\n\
                                 \r\n";

pub async fn discover_pnp_locations() -> Result<impl Stream<Item = Device>> {
    let any: SocketAddr = ([0, 0, 0, 0], 0).into();
    let socket = UdpSocket::bind(any).await?;
    socket.join_multicast_v4(Ipv4Addr::new(239, 255, 255, 250), Ipv4Addr::new(0, 0, 0, 0))?;

    // Set the socket address to the multicast IP and port for UPnP device discovery
    let socket_addr: SocketAddr = ([239, 255, 255, 250], 1900).into();

    // Send the discovery request
    socket
        .send_to(DISCOVERY_REQUEST.as_bytes(), &socket_addr)
        .await?;

    Ok(stream! {
        loop {
            async fn get_next(socket: &UdpSocket) -> Result<String> {
                // Receive the discovery response
                let mut buf = [0; 2048];
                let (size, _) = socket.recv_from(&mut buf).await?;
                // Convert the response to a string
                let response =
                    str::from_utf8(unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, size) })?;
                let headers = parse_raw_http_response(response)?;
                let location = headers.get("location")
                    .ok_or_else(|| anyhow!("Response header missing location"))?
                    .to_string();
                Ok(location)
            }

            if let Ok(location) = get_next(&socket).await {
                if let Ok(device) = parse_location(&location).await {
                    yield device;
                }
            }
        }
    })
}

fn parse_raw_http_response(response_str: &str) -> Result<HashMap<String, &str>> {
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
        None => Err(anyhow!("Invalid HTTP response")),
    }
}
