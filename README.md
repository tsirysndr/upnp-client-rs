<h1>UPnP Client</h1>
<p>
  <a href="LICENSE" target="_blank">
    <img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-blue.svg" />
  </a>
  <a href="https://crates.io/crates/upnp-client-rs" target="_blank">
    <img src="https://img.shields.io/crates/v/upnp-client-rs.svg" />
  </a>
  
  <a href="https://crates.io/crates/upnp-client-rs" target="_blank">
    <img src="https://img.shields.io/crates/dr/upnp-client-rs" />
  </a>
  
  <a href="https://docs.rs/upnp-client-rs" target="_blank">
    <img src="https://docs.rs/upnp-client-rs/badge.svg" />
  </a>
</p>

This is a UPNP client library for Rust.

### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
upnp-client = "0.1"
```

### Example

This example will print out all the devices found on the network.

```rust
use colored_json::prelude::*;
use futures_util::StreamExt;

use crate::discovery::discover_pnp_locations;

mod discovery;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_pnp_locations();
    tokio::pin!(devices);

    while let Some(device) = devices.next().await {
        let json = serde_json::to_string_pretty(&device)?;
        println!("{}", json.to_colored_json_auto()?);
    }

    Ok(())
}
```

Output:

```json
{
  "device_type": "urn:schemas-upnp-org:device:MediaRenderer:1",
  "friendly_name": "Kodi (MacBook-Pro-de-Tsiry-4.local)",
  "location": "http://192.168.8.101:1825/",
  "manufacturer": "XBMC Foundation",
  "manufacturer_url": "http://kodi.tv/",
  "model_description": "Kodi - Media Renderer",
  "model_name": "Kodi",
  "model_number": "18.4 Git:20190831-3ade758ceb",
  "services": [
    {
      "control_url": "/AVTransport/d599320b-2d3b-e0d7-3224-dc1c4b074dae/control.xml",
      "event_sub_url": "/AVTransport/d599320b-2d3b-e0d7-3224-dc1c4b074dae/event.xml",
      "scpd_url": "/AVTransport/d599320b-2d3b-e0d7-3224-dc1c4b074dae/scpd.xml",
      "service_id": "urn:upnp-org:serviceId:AVTransport",
      "service_type": "urn:schemas-upnp-org:service:AVTransport:1"
    },
    {
      "control_url": "/ConnectionManager/d599320b-2d3b-e0d7-3224-dc1c4b074dae/control.xml",
      "event_sub_url": "/ConnectionManager/d599320b-2d3b-e0d7-3224-dc1c4b074dae/event.xml",
      "scpd_url": "/ConnectionManager/d599320b-2d3b-e0d7-3224-dc1c4b074dae/scpd.xml",
      "service_id": "urn:upnp-org:serviceId:ConnectionManager",
      "service_type": "urn:schemas-upnp-org:service:ConnectionManager:1"
    },
    {
      "control_url": "/RenderingControl/d599320b-2d3b-e0d7-3224-dc1c4b074dae/control.xml",
      "event_sub_url": "/RenderingControl/d599320b-2d3b-e0d7-3224-dc1c4b074dae/event.xml",
      "scpd_url": "/RenderingControl/d599320b-2d3b-e0d7-3224-dc1c4b074dae/scpd.xml",
      "service_id": "urn:upnp-org:serviceId:RenderingControl",
      "service_type": "urn:schemas-upnp-org:service:RenderingControl:1"
    }
  ]
}
```

### License
MIT