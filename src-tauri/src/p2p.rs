use std::net::{SocketAddr, Ipv4Addr};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::{UdpSocket, TcpListener, TcpStream};
use quinn::{Endpoint, ServerConfig, ClientConfig, Connection};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rcgen::generate_simple_self_signed;
use tauri::Emitter;

static ACTIVE_CONNECTIONS: std::sync::OnceLock<Arc<tokio::sync::Mutex<HashMap<String, Connection>>>> = std::sync::OnceLock::new();

pub fn get_active_connections() -> Arc<tokio::sync::Mutex<HashMap<String, Connection>>> {
    ACTIVE_CONNECTIONS.get_or_init(|| Arc::new(tokio::sync::Mutex::new(HashMap::new()))).clone()
}

/// Helper to generate self-signed cert for Quinn
fn generate_cert() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error>> {
    let cert = generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_der = cert.cert.der().to_vec();
    let priv_key = cert.key_pair.serialize_der();
    Ok((
        vec![CertificateDer::from(cert_der)],
        PrivateKeyDer::try_from(priv_key).unwrap(),
    ))
}

#[derive(Debug)]
pub struct DummyCertificateVerifier;
impl rustls::client::danger::ServerCertVerifier for DummyCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

use rand::Rng;

async fn get_public_addr(socket: &UdpSocket) -> Option<SocketAddr> {
    let stun_servers = [
        "stun.l.google.com:19302",
        "stun1.l.google.com:19302",
        "stun.miwifi.com:3478",
    ];

    let mut req = [0u8; 20];
    req[0] = 0x00; req[1] = 0x01; // Binding Request
    req[2] = 0x00; req[3] = 0x00; // Message Length
    req[4] = 0x21; req[5] = 0x12; req[6] = 0xA4; req[7] = 0x42; // Magic Cookie
    rand::thread_rng().fill(&mut req[8..20]);

    for server in stun_servers {
        if let Ok(_) = socket.send_to(&req, server).await {
            let mut buf = [0u8; 1024];
            let timeout_duration = std::time::Duration::from_millis(5000);
            
            if let Ok(Ok((size, _))) = tokio::time::timeout(timeout_duration, socket.recv_from(&mut buf)).await {
                if size >= 20 && buf[0] == 0x01 && buf[1] == 0x01 { // Binding Success Response
                    let mut i = 20;
                    while i + 4 <= size {
                        let attr_type = u16::from_be_bytes([buf[i], buf[i+1]]);
                        let attr_len = u16::from_be_bytes([buf[i+2], buf[i+3]]) as usize;
                        if i + 4 + attr_len > size { break; }
                        
                        if attr_type == 0x0020 && attr_len == 8 { // XOR-MAPPED-ADDRESS
                            let family = buf[i+5];
                            if family == 0x01 { // IPv4
                                let port = u16::from_be_bytes([buf[i+6], buf[i+7]]) ^ 0x2112;
                                let ip = std::net::Ipv4Addr::new(
                                    buf[i+8] ^ 0x21,
                                    buf[i+9] ^ 0x12,
                                    buf[i+10] ^ 0xA4,
                                    buf[i+11] ^ 0x42
                                );
                                return Some(SocketAddr::new(std::net::IpAddr::V4(ip), port));
                            }
                        } else if attr_type == 0x0001 && attr_len == 8 { // MAPPED-ADDRESS
                            let family = buf[i+5];
                            if family == 0x01 { // IPv4
                                let port = u16::from_be_bytes([buf[i+6], buf[i+7]]);
                                let ip = std::net::Ipv4Addr::new(
                                    buf[i+8],
                                    buf[i+9],
                                    buf[i+10],
                                    buf[i+11]
                                );
                                return Some(SocketAddr::new(std::net::IpAddr::V4(ip), port));
                            }
                        }
                        i += 4 + attr_len;
                    }
                }
            }
        }
    }
    None
}

pub async fn run_host(local_tcp_port: u16, app: tauri::AppHandle) -> Result<(u16, Option<String>), String> {
    let _ = app.emit("app-log", "run_host: Генерация сертификата...");
    let (certs, key) = generate_cert().map_err(|e| e.to_string())?;
    let server_config = ServerConfig::with_single_cert(certs, key).map_err(|e| e.to_string())?;
    
    let _ = app.emit("app-log", "run_host: Биндинг UDP порта 0.0.0.0:0...");
    let tokio_sock = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    
    let _ = app.emit("app-log", "run_host: Отправка STUN запроса...");
    let public_ip_port = get_public_addr(&tokio_sock).await.map(|addr| addr.to_string());
    
    let _ = app.emit("app-log", "run_host: STUN запрос завершен, конвертация в std...");
    let std_sock = tokio_sock.into_std().map_err(|e| e.to_string())?;
    std_sock.set_nonblocking(true).unwrap();
    let listen_port = std_sock.local_addr().unwrap().port();
    
    let _ = app.emit("app-log", "run_host: Создание QUIC Endpoint...");
    let endpoint = Endpoint::new(
        Default::default(),
        Some(server_config),
        std_sock,
        Arc::new(quinn::TokioRuntime),
    ).map_err(|e| e.to_string())?;
    
    let local_addr = format!("127.0.0.1:{}", local_tcp_port);
    
    tokio::spawn(async move {
        while let Some(incoming) = endpoint.accept().await {
            let conn = match incoming.await {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            let remote_addr = conn.remote_address();
            let ip_str = remote_addr.ip().to_string();
            
            // Add to active connections
            get_active_connections().lock().await.insert(ip_str.clone(), conn.clone());
            
            let app_clone = app.clone();
            let ip_clone = ip_str.clone();
            let conn_clone = conn.clone();
            
            // Fetch GeoLocation and emit connected event
            tokio::spawn(async move {
                let loc = match reqwest::get(format!("http://ip-api.com/json/{}?lang=ru", ip_clone)).await {
                    Ok(r) => {
                        if let Ok(v) = r.json::<serde_json::Value>().await {
                            let city = v.get("city").and_then(|c| c.as_str()).unwrap_or("").to_string();
                            let country = v.get("country").and_then(|c| c.as_str()).unwrap_or("").to_string();
                            if city.is_empty() && country.is_empty() {
                                "Неизвестно".to_string()
                            } else {
                                format!("{}, {}", city, country)
                            }
                        } else {
                            "Неизвестно".to_string()
                        }
                    },
                    Err(_) => "Неизвестно".to_string()
                };
                
                let _ = app_clone.emit("peer-connected", serde_json::json!({
                    "ip": ip_clone,
                    "location": loc,
                    "rtt_ms": 0
                }));
                
                // Track ping loop
                let ping_conn = conn_clone.clone();
                let ping_ip = ip_clone.clone();
                let ping_app = app_clone.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        if ping_conn.close_reason().is_some() { break; }
                        let stats = ping_conn.stats();
                        let ping_ms = stats.path.rtt.as_millis();
                        let _ = ping_app.emit("peer-ping-updated", serde_json::json!({
                            "ip": ping_ip,
                            "rtt_ms": ping_ms
                        }));
                    }
                });
                
                // Wait for disconnect
                conn_clone.closed().await;
                get_active_connections().lock().await.remove(&ip_clone);
                let _ = app_clone.emit("peer-disconnected", serde_json::json!({ "ip": ip_clone }));
            });
            
            let local_addr = local_addr.clone();
            tokio::spawn(async move {
                while let Ok((mut quic_send, mut quic_recv)) = conn.accept_bi().await {
                    let local_addr = local_addr.clone();
                    tokio::spawn(async move {
                        // Forward TCP to QUIC
                        if let Ok(tcp_stream) = TcpStream::connect(local_addr).await {
                            let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();
                            let _ = tokio::join!(
                                tokio::io::copy(&mut quic_recv, &mut tcp_write),
                                tokio::io::copy(&mut tcp_read, &mut quic_send)
                            );
                        }
                    });
                }
            });
        }
    });

    Ok((listen_port, public_ip_port))
}

pub async fn run_client(host_addr: SocketAddr) -> Result<u16, String> {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DummyCertificateVerifier))
        .with_no_client_auth();
    let client_config = ClientConfig::new(Arc::new(quinn::crypto::rustls::QuicClientConfig::try_from(crypto).unwrap()));

    let std_sock = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    std_sock.set_nonblocking(true).unwrap();

    let endpoint = Endpoint::new(
        Default::default(),
        None,
        std_sock,
        Arc::new(quinn::TokioRuntime),
    ).map_err(|e| e.to_string())?;
    
    let conn = endpoint.connect_with(client_config, host_addr, "localhost")
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;

    // Open a local TCP port for Minecraft to connect to
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let local_port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        while let Ok((tcp_stream, _)) = listener.accept().await {
            if let Ok((mut quic_send, mut quic_recv)) = conn.open_bi().await {
                tokio::spawn(async move {
                    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();
                    let _ = tokio::join!(
                        tokio::io::copy(&mut tcp_read, &mut quic_send),
                        tokio::io::copy(&mut quic_recv, &mut tcp_write)
                    );
                });
            }
        }
    });

    Ok(local_port)
}
