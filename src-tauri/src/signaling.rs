use reqwest::Client;
use serde_json::{json, Value};
use reqwest_eventsource::{EventSource, Event};
use futures_util::stream::StreamExt;
use tauri::Emitter;

const LOBBY_BASE: &str = "http://2.26.87.126:7700";

pub struct SignalingClient {
    client_id: String,
    http: Client,
}

impl SignalingClient {
    pub fn new(client_id: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client_id,
            http: client,
        }
    }
    pub async fn refresh_lobby(&self) -> Result<Vec<Value>, String> {
        let url = format!("{}/channels/minecraft-lobby/messages", LOBBY_BASE);
        let res = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        
        if !res.status().is_success() {
            return Err(format!("Server returned: {}", res.status()));
        }

        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        if let Some(arr) = body.as_array() {
            Ok(arr.clone())
        } else {
            Ok(vec![])
        }
    }

    pub async fn publish_event(&self, channel: &str, event_name: &str, payload: Value) -> Result<(), String> {
        let url = format!("{}/channels/{}/messages", LOBBY_BASE, channel);
        
        let req_body = json!({
            "name": event_name,
            "clientId": self.client_id,
            "data": payload
        });

        let res = self.http.post(&url)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Failed to publish: {}", res.status()));
        }
        
        Ok(())
    }

    pub fn start_sse(client_id: String, app: tauri::AppHandle) {
        tauri::async_runtime::spawn(async move {
            let channels = format!("minecraft-lobby,lobby:{}", client_id);
            let url = format!("{}/sse?channels={}", LOBBY_BASE, channels);
            
            let mut es = EventSource::get(url);
            
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => {}
                    Ok(Event::Message(message)) => {
                        // message.data holds the JSON string, and message.event holds the event name?
                        // Actually, the server sends `data: {"channel":"...","data":{"name":"...", ...}}`
                        if let Ok(json_data) = serde_json::from_str::<Value>(&message.data) {
                            let _ = app.emit("lobby-event", json_data);
                        }
                    }
                    Err(_e) => {
                        // connection error, eventsource handles reconnects automatically
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }
        });
    }
}
