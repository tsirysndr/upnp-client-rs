use futures_util::StreamExt;
use upnp_client::{
    device_client::DeviceClient, discovery::discover_pnp_locations,
    media_server::MediaServerClient, types::Device,
};

const KODI_MEDIA_SERVER: &str = "Kodi - Media Server";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_pnp_locations().await?;
    tokio::pin!(devices);

    let mut kodi_device: Option<Device> = None;
    while let Some(device) = devices.next().await {
        // Select the first Kodi device found
        if device.model_description == Some(KODI_MEDIA_SERVER.to_string()) {
            kodi_device = Some(device);
            break;
        }
    }

    let kodi_device = kodi_device.unwrap();
    let device_client = DeviceClient::new(&kodi_device.location)?.connect().await?;
    let media_server_client = MediaServerClient::new(device_client);
    let results = media_server_client
        .browse("0", "BrowseDirectChildren")
        .await?;
    println!("{:#?}", results);
    Ok(())
}
