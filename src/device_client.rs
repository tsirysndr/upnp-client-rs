use std::time::Duration;

use surf::{Client, Config, Error, Url};

pub struct DeviceClient {
    http_client: Client,
}

impl DeviceClient {
    pub fn new() -> Self {
        Self {
            http_client: Config::new()
                .set_timeout(Some(Duration::from_secs(5)))
                .try_into()
                .unwrap(),
        }
    }

    pub async fn call_action(&self, service_id: &str, action_name: &str) -> Result<(), Error> {
        let service_id = resolve_service(service_id);
        self.get_service_description(&service_id).await;
        let service_url = Url::parse("http://").unwrap();
        self.http_client.post(service_url).send().await?;
        Ok(())
    }

    async fn get_service_description(&self, service_id: &str) {
        todo!()
    }
}

fn resolve_service(service_id: &str) -> String {
    match service_id.contains(":") {
        true => service_id.to_string(),
        false => format!("urn:upnp-org:serviceId:{}", service_id),
    }
}

fn parse_service_description(xml: &str) {
    todo!()
}
