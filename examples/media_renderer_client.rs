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
    let mut media_renderer = MediaRendererClient::new(device_client);

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
    let events = media_renderer.subscribe().await;
    tokio::pin!(events);

    while let Some(event) = events.next().await {
        println!("\n{}\n", event);
    }

    // media_renderer.stop().await?;
    // media_renderer.play().await?;
    // media_renderer.pause().await?;

    Ok(())
}
