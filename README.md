<h1>UPnP Client</h1>
<p>
  <a href="LICENSE" target="_blank">
    <img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-blue.svg" />
  </a>
  <a href="https://crates.io/crates/upnp-client" target="_blank">
    <img src="https://img.shields.io/crates/v/upnp-client.svg" />
  </a>
  
  <a href="https://crates.io/crates/upnp-client" target="_blank">
    <img src="https://img.shields.io/crates/dr/upnp-client" />
  </a>
  
  <a href="https://docs.rs/upnp-client" target="_blank">
    <img src="https://docs.rs/upnp-client/badge.svg" />
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
    let devices = discover_pnp_locations().await?;
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

## Streaming

```rust
use futures_util::StreamExt;
use upnp_client::{
    device_client::DeviceClient,
    discovery::discover_pnp_locations,
    media_renderer::MediaRendererClient,
    types::{Device, LoadOptions, Metadata, ObjectClass},
};

const KODI_MEDIA_RENDERER: &str = "Kodi - Media Renderer";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_pnp_locations().await?;
    tokio::pin!(devices);

    let mut kodi_device: Option<Device> = None;
    while let Some(device) = devices.next().await {
        // Select the first Kodi device found
        if device.model_description == Some(KODI_MEDIA_RENDERER.to_string()) {
            kodi_device = Some(device);
            break;
        }
    }

    let kodi_device = kodi_device.unwrap();
    let device_client = DeviceClient::new(&kodi_device.location)?.connect().await?;
    let media_renderer = MediaRendererClient::new(device_client);

    let options = LoadOptions {
        dlna_features: Some(
            "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000"
                .to_string(),
        ),
        content_type: Some("video/mp4".to_string()),
        metadata: Some(Metadata {
            title: "Big Buck Bunny".to_string(),
            ..Default::default()
        }),
        autoplay: true,
        object_class: Some(ObjectClass::Video),
        ..Default::default()
    };

    let media_url =
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4";

    media_renderer.load(media_url, options).await?;

    Ok(())
}


```

See the [examples](./examples) directory for more examples.

### Features

- [x] Discover devices
- [x] Control Media Renderer device (Load, Play, Pause, Stop, Seek, etc.)
- [x] Browse Media Server device


### References
- [UPnP Device Architecture 1.1](http://upnp.org/specs/arch/UPnP-arch-DeviceArchitecture-v1.1.pdf)
- [UPnP AVTransport v3 Service](http://www.upnp.org/specs/av/UPnP-av-AVTransport-v3-Service-20101231.pdf)
- [UPnP AV ContentDirectory v3 Service](http://upnp.org/specs/av/UPnP-av-ContentDirectory-v3-Service.pdf)

### License
MIT
