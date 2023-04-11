use std::collections::HashMap;

use crate::{
    device_client::DeviceClient,
    parser::parse_browse_response,
    types::{Container, Item},
};
use anyhow::Error;

#[derive(Clone)]
pub struct MediaServerClient {
    device_client: DeviceClient,
}

impl MediaServerClient {
    pub fn new(device_client: DeviceClient) -> Self {
        Self { device_client }
    }

    pub async fn browse(
        &self,
        object_id: &str,
        browse_flag: &str,
    ) -> Result<(Vec<Container>, Vec<Item>), Error> {
        let mut params = HashMap::new();
        params.insert("ObjectID".to_string(), object_id.to_string());
        params.insert("BrowseFlag".to_string(), browse_flag.to_string());
        params.insert("Filter".to_string(), "*".to_string());
        params.insert("StartingIndex".to_string(), "0".to_string());
        params.insert("RequestedCount".to_string(), "0".to_string());
        params.insert("SortCriteria".to_string(), "".to_string());

        let response = self
            .device_client
            .call_action("ContentDirectory", "Browse", params)
            .await?;

        let ip = self.device_client.ip();

        parse_browse_response(&response, &ip)
    }

    pub async fn get_sort_capabilities(&self) -> Result<(), Error> {
        let params = HashMap::new();
        self.device_client
            .call_action("ContentDirectory", "GetSortCapabilities", params)
            .await?;

        todo!()
    }

    pub async fn get_system_update_id(&self) -> Result<(), Error> {
        let params = HashMap::new();
        self.device_client
            .call_action("ContentDirectory", "GetSystemUpdateID", params)
            .await?;

        todo!()
    }

    pub async fn get_search_capabilities(&self) -> Result<(), Error> {
        let params = HashMap::new();
        self.device_client
            .call_action("ContentDirectory", "GetSearchCapabilities", params)
            .await?;

        todo!()
    }

    pub async fn search(&self) -> Result<(), Error> {
        let params = HashMap::new();
        self.device_client
            .call_action("ContentDirectory", "Search", params)
            .await?;

        todo!()
    }

    pub async fn update_object(&self) -> Result<(), Error> {
        let params = HashMap::new();
        self.device_client
            .call_action("ContentDirectory", "UpdateObject", params)
            .await?;

        todo!()
    }
}
