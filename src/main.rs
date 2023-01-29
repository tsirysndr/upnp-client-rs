use futures_util::StreamExt;

use crate::discovery::discover_pnp_locations;

mod discovery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let locations = discover_pnp_locations();
    tokio::pin!(locations);

    while let Some(location) = locations.next().await {
        println!("discovered location: {}", location);
    }

    Ok(())
}
