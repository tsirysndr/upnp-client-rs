use anyhow::Error;
use async_stream::stream;
use futures_util::Stream;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str;
use std::thread::sleep;
use std::time::Duration;

use crate::parser::parse_location;
use crate::types::Device;

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
