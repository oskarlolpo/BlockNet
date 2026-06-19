use tauri::{AppHandle, Emitter};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::thread;
use serde::{Deserialize, Serialize};
use regex::Regex;
// use futures_util::stream::StreamExt;

#[derive(Clone, Serialize, Deserialize)]
struct LanServer {
    motd: String,
    port: String,
    ip: String,
    edition: String,
    host: Option<String>,
    online: Option<String>,
}

#[tauri::command]
fn start_lan_scanner(app: AppHandle, edition: String) {
    if edition == "java" {
        let app_java = app.clone();
    thread::spawn(move || {
        let multi_addr = Ipv4Addr::new(224, 0, 2, 60);
        let port = 4445;
        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);

        let socket = match UdpSocket::bind(addr) {
            Ok(s) => s,
            Err(e) => {
                let _ = app_java.emit("app-log", format!("Ошибка привязки UDP сокета (Java): {}", e));
                return;
            }
        };

        if let Err(e) = socket.join_multicast_v4(&multi_addr, &Ipv4Addr::new(0, 0, 0, 0)) {
            let _ = app_java.emit("app-log", format!("Ошибка подключения к multicast (Java): {}", e));
            return;
        }

        let mut buf = [0u8; 1024];
        let re = Regex::new(r"\[MOTD\](.*?)\[/MOTD\]\[AD\](.*?)\[/AD\]").unwrap();

        let _ = app_java.emit("app-log", "Запуск локального сканера (Java Edition)...");
        let _ = socket.set_read_timeout(Some(std::time::Duration::from_millis(500)));

        let global_end_time = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < global_end_time {
            match socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    if let Ok(msg) = std::str::from_utf8(&buf[..amt]) {
                        if let Some(caps) = re.captures(msg) {
                            let motd_raw = caps.get(1).map_or("", |m| m.as_str()).to_string();
                            let motd = motd_raw.split(" - ").last().unwrap_or(&motd_raw).to_string();
                            let port_str = caps.get(2).map_or("", |m| m.as_str()).to_string();
                            let ip = src.ip().to_string();
                            
                            let server = LanServer {
                                motd,
                                port: port_str,
                                ip,
                                edition: "Java".to_string(),
                                host: None,
                                online: None,
                            };
                            
                            let _ = app_java.emit("lan-server-found", server);
                        }
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock && e.kind() != std::io::ErrorKind::TimedOut {
                        let _ = app_java.emit("app-log", format!("Ошибка получения UDP (Java): {}", e));
                    }
                }
            }
        }
        let _ = app_java.emit("app-log", "Остановка локального сканера (Java Edition)");
    });
    }

    if edition == "bedrock" {
    let app_bedrock = app.clone();
    thread::spawn(move || {
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                let _ = app_bedrock.emit("app-log", format!("Ошибка привязки RakNet сокета: {}", e));
                return;
            }
        };
        let _ = socket.set_broadcast(true);
        let _ = socket.set_read_timeout(Some(std::time::Duration::from_millis(500)));

        let magic: [u8; 16] = [0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78];
        
        let _ = app_bedrock.emit("app-log", "Запуск RakNet сканера (Bedrock Edition)...");

        let mut buf = [0u8; 1024];
        let global_end_time = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < global_end_time {
            let mut ping_packet = vec![0x01];
            let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
            ping_packet.extend_from_slice(&time.to_be_bytes());
            ping_packet.extend_from_slice(&magic);
            ping_packet.extend_from_slice(&0u64.to_be_bytes());
            
            let _ = socket.send_to(&ping_packet, "255.255.255.255:19132");
            let _ = socket.send_to(&ping_packet, "127.0.0.1:19132");

            let end_time = std::time::Instant::now() + std::time::Duration::from_secs(1);
            while std::time::Instant::now() < end_time {
                match socket.recv_from(&mut buf) {
                    Ok((amt, src)) => {
                        if amt > 35 && buf[0] == 0x1C {
                            let str_len = u16::from_be_bytes([buf[33], buf[34]]) as usize;
                            if amt >= 35 + str_len {
                                if let Ok(server_str) = std::str::from_utf8(&buf[35..35 + str_len]) {
                                    let parts: Vec<&str> = server_str.split(';').collect();
                                    if parts.len() >= 8 {
                                        let host = parts.get(1).unwrap_or(&"").to_string();
                                        let online = format!("{}/{}", parts.get(4).unwrap_or(&"0"), parts.get(5).unwrap_or(&"0"));
                                        let world_name = parts.get(7).unwrap_or(&"").to_string();
                                        let port = parts.get(10).unwrap_or(&"19132").to_string();
                                        
                                        let server = LanServer {
                                            motd: world_name,
                                            port,
                                            ip: src.ip().to_string(),
                                            edition: "Bedrock".to_string(),
                                            host: Some(host),
                                            online: Some(online),
                                        };
                                        let _ = app_bedrock.emit("lan-server-found", server);
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        let _ = app_bedrock.emit("app-log", "Остановка RakNet сканера (Bedrock Edition)");
    });

    let app_nethernet = app.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            use nethernet::signaling::lan::LanSignaling;
            use std::net::SocketAddr;
            let bind_addr: SocketAddr = "[::]:0".parse().unwrap();
            
            if let Ok(signaling) = LanSignaling::new(0, bind_addr).await {
                let _ = app_nethernet.emit("app-log", "Запуск NetherNet сканера (Bedrock Edition)...");
                let end_time = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
                while tokio::time::Instant::now() < end_time {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let servers = signaling.discover().await;
                    for (id, server_data) in servers {
                        let ip = if let Some(addr) = signaling.get_address(id).await {
                            addr.ip().to_string()
                        } else {
                            "127.0.0.1".to_string()
                        };
                        let server = LanServer {
                            motd: server_data.level_name.clone(),
                            port: "19132".to_string(), 
                            ip,
                            edition: "Bedrock".to_string(),
                            host: Some(server_data.server_name.clone()),
                            online: Some(format!("{}/{}", server_data.player_count, server_data.max_player_count)),
                        };
                        let _ = app_nethernet.emit("lan-server-found", server);
                    }
                }
                signaling.shutdown().await;
                let _ = app_nethernet.emit("app-log", "Остановка NetherNet сканера (Bedrock Edition)");
            } else {
                let _ = app_nethernet.emit("app-log", "Ошибка запуска NetherNet сканера (Bedrock Edition)");
            }
        });
    });
    }
}

pub mod p2p;
pub mod signaling;

use tauri::State;
use std::sync::Mutex;
// Removed duplicate serde

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct AppStatus {
    pub mode: String, // "idle", "host", "client"
    pub state: String,
    pub client_id: String,
    pub logs: Vec<String>,
    pub public_udp_addr: Option<String>,
    pub local_game_port: Option<u16>,
    pub max_players: Option<u32>,
    pub bedrock_port: Option<u16>,
}

pub struct AppState {
    pub status: Mutex<AppStatus>,
    pub signaling: signaling::SignalingClient,
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    Ok(state.status.lock().unwrap().clone())
}

#[tauri::command]
async fn refresh_lobby(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let raw = state.signaling.refresh_lobby().await?;
    let mut flattened = Vec::new();
    for msg in raw {
        if let Some(data) = msg.get("data").and_then(|d| d.as_object()) {
            let mut flat = data.clone();
            if let Some(client_id) = msg.get("clientId") {
                flat.insert("client_id".to_string(), client_id.clone());
            }
            flattened.push(serde_json::Value::Object(flat));
        }
    }
    Ok(flattened)
}

#[tauri::command]
async fn publish_lobby_event(
    state: State<'_, AppState>,
    channel: String,
    event: String,
    payload: serde_json::Value
) -> Result<(), String> {
    state.signaling.publish_event(&channel, &event, payload).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct HostConfig {
    room_name: String,
    local_port: u16,
}

#[allow(unused_variables)]
#[tauri::command]
async fn start_hosting(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    room_name: Option<String>,
    room_theme: Option<String>,
    room_password: Option<String>,
    require_password: Option<bool>,
    game_version: Option<String>,
    local_port: Option<u16>,
    is_external_host: Option<bool>,
    external_host_address: Option<String>,
    force_direct_mode: Option<bool>,
    enable_geyser: Option<bool>,
    geyser_port: Option<u16>,
    enable_e4mc: Option<bool>,
    host_name: Option<String>,
) -> Result<String, String> {
    let _ = std::fs::write("start_hosting_invoked.txt", "INVOKED!");
    let _ = app.emit("app-log", "Начало start_hosting...");
    let (port, public_ip_port) = match p2p::run_host(local_port.unwrap_or(25565), app.clone()).await {
        Ok(res) => res,
        Err(e) => {
            let _ = app.emit("app-log", format!("run_host failed: {}", e));
            return Err(e);
        }
    };
    
    let _ = app.emit("app-log", format!("run_host завершен. Порт: {}", port));
    
    let final_room_name = room_name.unwrap_or_else(|| "My Server".into());
    let final_host_name = host_name.unwrap_or_else(|| "Player".into());
    let final_game_version = game_version.unwrap_or_else(|| "java".into());
    let final_max_players: u32 = 30;
    let has_pw = require_password.unwrap_or(false);
    let public_join_addr = public_ip_port.clone().unwrap_or_default();

    let payload = serde_json::json!({
        "room_name": final_room_name,
        "host_name": final_host_name,
        "nickname": final_host_name,
        "minecraft_version": final_game_version,
        "game_version": final_game_version,
        "version": final_game_version,
        "port": port,
        "public_ip": public_ip_port,
        "public_join_address": public_join_addr,
        "is_quic": true,
        "has_password": has_pw,
        "requirePassword": has_pw,
        "max_players": final_max_players,
        "players_max": final_max_players,
        "maxPlayers": final_max_players,
        "player_count": 0,
        "playerCount": 0,
        "slots": format!("0/{}", final_max_players)
    });
    
    let _ = app.emit("app-log", "Публикация в лобби...");
    match state.signaling.publish_event("minecraft-lobby", "host-presence", payload).await {
        Ok(_) => { let _ = app.emit("app-log", "Опубликовано успешно"); },
        Err(e) => { let _ = app.emit("app-log", format!("Ошибка публикации: {}", e)); }
    }
    
    let mut st = state.status.lock().unwrap();
    st.mode = "host".to_string();
    st.state = "hosting".to_string();
    st.local_game_port = Some(port);
    st.public_udp_addr = public_ip_port.clone();
    st.max_players = Some(30);
    st.logs.push(serde_json::to_string(&serde_json::json!({
        "type": "host_started",
        "local_port": port,
        "public_address": public_ip_port,
        "event": "Ожидаем игроков..."
    })).unwrap_or_default());
    
    Ok(port.to_string())
}

#[tauri::command]
async fn kick_player(ip: String) -> Result<(), String> {
    let active_conns = p2p::get_active_connections();
    let mut conns = active_conns.lock().await;
    if let Some(conn) = conns.remove(&ip) {
        conn.close(0u32.into(), b"kicked by host");
    }
    Ok(())
}

#[tauri::command]
async fn stop_hosting(state: State<'_, AppState>) -> Result<(), String> {
    let mut st = state.status.lock().unwrap();
    st.mode = "idle".to_string();
    st.state = "idle".to_string();
    Ok(())
}

#[tauri::command]
async fn prepare_client_connect(_peer_id: String, _room_name: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn connect_to_peer(
    _app: tauri::AppHandle,
    state: State<'_, AppState>,
    _peer_id: String,
    peer_addrs: Vec<String>,
    _password: Option<String>,
    _relay_session_id: Option<String>,
) -> Result<String, String> {
    if peer_addrs.is_empty() {
        return Err("Нет адресов для подключения".into());
    }
    
    let addr: std::net::SocketAddr = peer_addrs[0].parse().map_err(|e| format!("Invalid address: {}", e))?;
    
    let local_port = p2p::run_client(addr).await?;
    let local_ip = format!("127.0.0.1:{}", local_port);
    
    let mut st = state.status.lock().unwrap();
    st.mode = "client".to_string();
    st.state = "connected".to_string();
    st.logs.push(format!("Подключено! Заходите на {}", local_ip));
    
    Ok(local_ip)
}


#[tauri::command]
async fn subscribe_lobby_events(
    _state: State<'_, AppState>,
    _channel: String,
) -> Result<(), String> {
    // Stub: SSE subscription is handled globally by SignalingClient at startup.
    // This command exists for frontend compatibility.
    Ok(())
}

#[tauri::command]
async fn unsubscribe_lobby_events(
    _state: State<'_, AppState>,
    _channel: String,
) -> Result<(), String> {
    // Stub: SSE unsubscription placeholder.
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client_id = format!("client-{}", rand::random::<u16>());
    let client_id_for_setup = client_id.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            signaling::SignalingClient::start_sse(client_id_for_setup, app.handle().clone());
            Ok(())
        })
        .manage(AppState {
            status: std::sync::Mutex::new(AppStatus {
                mode: "idle".to_string(),
                state: "idle".to_string(),
                client_id: client_id.clone(),
                logs: vec![],
                public_udp_addr: None,
                local_game_port: None,
                max_players: None,
                bedrock_port: None,
            }),
            signaling: signaling::SignalingClient::new(client_id),
        })
        .invoke_handler(tauri::generate_handler![
            start_lan_scanner,
            get_status,
            refresh_lobby,
            publish_lobby_event,
            start_hosting,
            stop_hosting,
            prepare_client_connect,
            connect_to_peer,
            kick_player,
            subscribe_lobby_events,
            unsubscribe_lobby_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
