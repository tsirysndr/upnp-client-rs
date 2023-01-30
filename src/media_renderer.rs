use crate::device_client::DeviceClient;

pub struct MediaRendererClient {
    device_client: DeviceClient,
}

impl MediaRendererClient {
    pub fn new() -> Self {
        Self {
            device_client: DeviceClient::new(),
        }
    }
    pub fn load(&self, url: &str) {
        todo!()
    }

    pub fn play(&self) {
        todo!()
    }

    pub fn pause(&self) {
        todo!()
    }

    pub fn seek(&self) {
        todo!()
    }

    pub fn stop(&self) {
        todo!()
    }

    pub fn get_volume(&self) {
        todo!()
    }

    pub fn set_volume(&self) {
        todo!()
    }

    pub fn get_supported_protocols(&self) {
        todo!()
    }

    pub fn get_position(&self) {
        todo!()
    }

    pub fn get_duration(&self) {
        todo!()
    }
}
