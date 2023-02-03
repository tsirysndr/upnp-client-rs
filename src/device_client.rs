use std::{collections::HashMap, time::Duration};

use anyhow::Error;
use surf::{Client, Config, Url};
use xml_builder::{XMLBuilder, XMLElement, XMLVersion};

use crate::{
    parser::parse_location,
    types::{Device, Service},
};

pub struct DeviceClient {
    base_url: Url,
    http_client: Client,
    device: Option<Device>,
}

impl DeviceClient {
    pub fn new(url: &str) -> Self {
        Self {
            base_url: Url::parse(url).unwrap(),
            http_client: Config::new()
                .set_timeout(Some(Duration::from_secs(5)))
                .try_into()
                .unwrap(),
            device: None,
        }
    }

    pub async fn connect(&mut self) -> Result<Self, Error> {
        self.device = Some(parse_location(self.base_url.as_str()).await?);
        Ok(Self {
            base_url: self.base_url.clone(),
            http_client: self.http_client.clone(),
            device: self.device.clone(),
        })
    }

    pub async fn call_action(
        &self,
        service_id: &str,
        action_name: &str,
        params: HashMap<String, String>,
    ) -> Result<String, Error> {
        if self.device.is_none() {
            return Err(Error::msg("Device not connected"));
        }
        let service_id = resolve_service(service_id);
        let service = self.get_service_description(&service_id).await?;

        // check if action is available
        let action = service.actions.iter().find(|a| a.name == action_name);
        match action {
            Some(_) => {
                self.call_action_internal(&service, action_name, params)
                    .await
            }
            None => Err(Error::msg("Action not found")),
        }
    }

    async fn call_action_internal(
        &self,
        service: &Service,
        action_name: &str,
        params: HashMap<String, String>,
    ) -> Result<String, Error> {
        let control_url = Url::parse(&service.control_url).unwrap();

        let mut xml = XMLBuilder::new()
            .version(XMLVersion::XML1_1)
            .encoding("UTF-8".into())
            .build();

        let mut envelope = XMLElement::new("s:Envelope");
        envelope.add_attribute("xmlns:s", "http://schemas.xmlsoap.org/soap/envelope/");
        envelope.add_attribute(
            "s:encodingStyle",
            "http://schemas.xmlsoap.org/soap/encoding/",
        );

        let mut body = XMLElement::new("s:Body");
        let action = format!("u:{}", action_name);
        let mut action = XMLElement::new(action.as_str());
        action.add_attribute("xmlns:u", service.service_type.as_str());

        for (name, value) in params {
            let mut param = XMLElement::new(name.as_str());
            param.add_text(value).unwrap();
            action.add_child(param).unwrap();
        }

        body.add_child(action).unwrap();
        envelope.add_child(body).unwrap();

        xml.set_root_element(envelope);

        let mut writer: Vec<u8> = Vec::new();
        xml.generate(&mut writer).unwrap();
        let xml = String::from_utf8(writer).unwrap();

        let soap_action = format!("\"{}#{}\"", service.service_type, action_name);

        let mut res = self
            .http_client
            .post(control_url)
            .header("Content-Type", "text/xml; charset=\"utf-8\"")
            .header("Content-Length", xml.len().to_string())
            .header("SOAPACTION", soap_action)
            .header("Connection", "close")
            .body_string(xml.clone())
            .send()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        Ok(res
            .body_string()
            .await
            .map_err(|e| Error::msg(e.to_string()))?)
    }

    async fn get_service_description(&self, service_id: &str) -> Result<Service, Error> {
        if let Some(device) = &self.device {
            let service = device
                .services
                .iter()
                .find(|s| s.service_id == service_id)
                .unwrap();
            return Ok(service.clone());
        }
        Err(Error::msg("Device not connected"))
    }
}

fn resolve_service(service_id: &str) -> String {
    match service_id.contains(":") {
        true => service_id.to_string(),
        false => format!("urn:upnp-org:serviceId:{}", service_id),
    }
}
