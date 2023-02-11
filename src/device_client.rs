use std::{
    collections::HashMap,
    env,
    net::TcpListener,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{
    parser::{
        deserialize_metadata, parse_av_transport_uri_metadata, parse_current_play_mode,
        parse_current_track_metadata, parse_last_change, parse_location, parse_transport_state,
    },
    types::{AVTransportEvent, Device, Event, Service},
    BROADCAST_EVENT,
};
use anyhow::Error;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
};
use hyper::{Body, Request, Response, Server};
use surf::{Client, Config, Url};
use xml_builder::{XMLBuilder, XMLElement, XMLVersion};

#[derive(Clone)]
pub struct DeviceClient {
    base_url: Url,
    http_client: Client,
    device: Option<Device>,
    stop: Arc<Mutex<bool>>,
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
            stop: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn connect(&mut self) -> Result<Self, Error> {
        self.device = Some(parse_location(self.base_url.as_str()).await?);
        Ok(Self {
            base_url: self.base_url.clone(),
            http_client: self.http_client.clone(),
            device: self.device.clone(),
            stop: self.stop.clone(),
        })
    }

    pub fn ip(&self) -> String {
        self.base_url.host_str().unwrap().to_string()
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

    pub async fn subscribe(&mut self, service_id: &str) -> Result<(), Error> {
        if self.device.is_none() {
            return Err(Error::msg("Device not connected"));
        }
        let service_id = resolve_service(service_id);
        let service = self.get_service_description(&service_id).await?;

        let user_agent = format!(
            "upnp-client/{} ({})",
            env!("CARGO_PKG_VERSION"),
            env::consts::OS
        );

        let (address, port) = self.ensure_eventing_server().await?;
        let callback = format!("<http://{}:{}>", address, port);

        let client = hyper::Client::new();
        let req = hyper::Request::builder()
            .method("SUBSCRIBE")
            .uri(service.event_sub_url.clone())
            .header("CALLBACK", callback)
            .header("NT", "upnp:event")
            .header("TIMEOUT", "Second-1800")
            .header("USER-AGENT", user_agent)
            .body(hyper::Body::empty())
            .unwrap();
        client.request(req).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self, service_id: &str, sid: &str) -> Result<(), Error> {
        if self.device.is_none() {
            return Err(Error::msg("Device not connected"));
        }
        let service_id = resolve_service(service_id);
        let service = self.get_service_description(&service_id).await.unwrap();
        let client = hyper::Client::new();
        let req = hyper::Request::builder()
            .method("UNSUBSCRIBE")
            .uri(service.event_sub_url.clone())
            .header("SID", sid)
            .body(hyper::Body::empty())
            .unwrap();

        client.request(req).await?;

        self.release_eventing_server().await?;
        Ok(())
    }

    async fn ensure_eventing_server(&mut self) -> Result<(String, u16), Error> {
        let addr: &str = "0.0.0.0:0";
        let listener = TcpListener::bind(&addr).unwrap();

        let service = make_service_fn(|_: &AddrStream| async {
            Ok::<_, hyper::Error>(service_fn(|req: Request<Body>| async move {
                let sid = req
                    .headers()
                    .get("sid")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                let body = hyper::body::to_bytes(req.into_body()).await?;
                let xml = String::from_utf8(body.to_vec()).unwrap();

                let last_change = parse_last_change(xml.as_str()).unwrap();
                let last_change = last_change.unwrap_or_default();

                let transport_state = parse_transport_state(last_change.as_str()).unwrap();
                let play_mode = parse_current_play_mode(last_change.as_str()).unwrap();
                let av_transport_uri_metadata =
                    parse_av_transport_uri_metadata(last_change.as_str()).unwrap();
                let current_track_metadata =
                    parse_current_track_metadata(last_change.as_str()).unwrap();

                match transport_state {
                    Some(state) => {
                        let tx = BROADCAST_EVENT.lock().unwrap();
                        let tx = tx.as_ref().clone();
                        let ev = AVTransportEvent::TransportState {
                            sid: sid.clone(),
                            transport_state: state,
                        };
                        tx.unwrap().send(Event::AVTransport(ev)).unwrap();
                    }
                    None => {}
                }

                match play_mode {
                    Some(mode) => {
                        let tx = BROADCAST_EVENT.lock().unwrap();
                        let tx = tx.as_ref().clone();
                        let ev = AVTransportEvent::CurrentPlayMode {
                            sid: sid.clone(),
                            play_mode: mode,
                        };
                        tx.unwrap().send(Event::AVTransport(ev)).unwrap();
                    }
                    None => {}
                }

                match av_transport_uri_metadata {
                    Some(metadata) => {
                        let tx = BROADCAST_EVENT.lock().unwrap();
                        let tx = tx.as_ref().clone();
                        let m = deserialize_metadata(metadata.as_str()).unwrap();
                        let ev = AVTransportEvent::AVTransportURIMetaData {
                            sid: sid.clone(),
                            url: m.url,
                            title: m.title,
                            artist: m.artist,
                            album: m.album,
                            album_art_uri: m.album_art_uri,
                            genre: m.genre,
                        };
                        tx.unwrap().send(Event::AVTransport(ev)).unwrap();
                    }
                    None => {}
                }

                match current_track_metadata {
                    Some(metadata) => {
                        let m = deserialize_metadata(metadata.as_str()).unwrap();
                        let tx = BROADCAST_EVENT.lock().unwrap();
                        let tx = tx.as_ref().clone();
                        let ev = AVTransportEvent::CurrentTrackMetadata {
                            sid: sid.clone(),
                            url: m.url,
                            title: m.title,
                            artist: m.artist,
                            album: m.album,
                            album_art_uri: m.album_art_uri,
                            genre: m.genre,
                        };
                        tx.unwrap().send(Event::AVTransport(ev)).unwrap();
                    }
                    None => {}
                }

                Ok::<_, hyper::Error>(Response::new(Body::empty()))
            }))
        });

        let server = Server::from_tcp(listener).unwrap().serve(service);

        let address = server.local_addr().ip().to_string();
        let port = server.local_addr().port();

        let stop = self.stop.clone();

        tokio::spawn(async move {
            server.await.unwrap();
        });

        tokio::spawn(async move {
            while !*stop.lock().unwrap() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        Ok((address, port))
    }

    async fn release_eventing_server(&mut self) -> Result<(), Error> {
        let mut stop = self.stop.lock().unwrap();
        *stop = true;
        Ok(())
    }
}

fn resolve_service(service_id: &str) -> String {
    match service_id.contains(":") {
        true => service_id.to_string(),
        false => format!("urn:upnp-org:serviceId:{}", service_id),
    }
}
