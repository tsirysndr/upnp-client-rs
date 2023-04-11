use colored_json::prelude::*;
use futures_util::StreamExt;
use upnp_client::discovery::discover_pnp_locations;

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
