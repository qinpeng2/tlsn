use anyhow::Result;
use http_body_util::Empty;
use hyper::{body::Bytes, Request, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use tlsn::{
    config::{
        prove::ProveConfig,
        prover::ProverConfig,
        tls::TlsClientConfig,
        tls_commit::{mpc::MpcTlsConfig, TlsCommitConfig},
    },
    connection::ServerName,
    webpki::RootCertStore,
    Session,
};
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};

// Maximum number of bytes that can be sent from prover to server.
const MAX_SENT_DATA: usize = 1 << 16; // 64KB
// Maximum number of bytes that can be received by prover from server.
const MAX_RECV_DATA: usize = 1 << 20; // 1MB

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let verifier_url = std::env::var("VERIFIER_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8080".to_string());
    
    let target_url = std::env::var("TARGET_URL")
        .unwrap_or_else(|_| "https://www.binance.com/setting/kyc".to_string());

    println!("🚀 Prover starting");
    println!("📡 Connecting to Verifier at: {}", verifier_url);
    println!("🌐 Target URL: {}", target_url);
    println!("🔐 Using real Garbled Circuits and Oblivious Transfer (OT)");
    println!("⚠️  Make sure tlsn_insecure feature is NOT enabled\n");

    // Parse target URL
    let uri = target_url.parse::<Uri>()?;
    assert_eq!(uri.scheme().unwrap().as_str(), "https");
    let server_domain = uri.authority().unwrap().host();
    let server_port = uri.port_u16().unwrap_or(443);
    
    // Resolve server address
    let server_addr = format!("{}:{}", server_domain, server_port);
    let server_socket_addr: SocketAddr = tokio::net::lookup_host(&server_addr)
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve {}", server_addr))?;

    // Connect to verifier via WebSocket
    println!("📡 Connecting to verifier...");
    let (ws_stream, _) = connect_async(&verifier_url).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    // Create a duplex stream for the Session
    let (session_io_read, session_io_write) = tokio::io::duplex(1 << 20);
    
    // Bridge: WebSocket -> Session
    let bridge_ws_to_session = tokio::spawn(async move {
        let mut write_half = session_io_write;
        while let Some(msg) = ws_read.next().await {
            let data = match msg {
                Ok(Message::Binary(data)) => data,
                Ok(Message::Text(text)) => text.into_bytes(),
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => continue,
            };
            use tokio::io::AsyncWriteExt;
            if write_half.write_all(&data).await.is_err() {
                break;
            }
        }
    });
    
    // Bridge: Session -> WebSocket
    let bridge_session_to_ws = tokio::spawn(async move {
        let mut read_half = session_io_read;
        let mut buf = vec![0u8; 8192];
        loop {
            use tokio::io::AsyncReadExt;
            match read_half.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if ws_write.send(Message::Binary(buf[..n].to_vec())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Create session with the verifier
    let session = Session::new(session_io_read.compat());
    let (driver, mut handle) = session.split();
    
    // Spawn the session driver
    let driver_task = tokio::spawn(driver);
    
    // Create prover
    let prover = handle
        .new_prover(ProverConfig::builder().build()?)?
        .commit(
            TlsCommitConfig::builder()
                .protocol(
                    MpcTlsConfig::builder()
                        .max_sent_data(MAX_SENT_DATA)
                        .max_recv_data(MAX_RECV_DATA)
                        .build()?,
                )
                .build()?,
        )
        .await?;
    
    println!("✅ Connected to verifier. Starting TLS connection to server...");
    
    // Open TCP connection to the server
    let client_socket = tokio::net::TcpStream::connect(server_socket_addr).await?;
    
    // Bind the prover to the server connection
    let (tls_connection, prover_fut) = prover
        .connect(
            TlsClientConfig::builder()
                .server_name(ServerName::Dns(server_domain.try_into()?))
                .root_store(RootCertStore::mozilla())
                .build()?,
            client_socket.compat(),
        )
        .await?;
    let tls_connection = TokioIo::new(tls_connection.compat());
    
    // Spawn the Prover to run in the background
    let prover_task = tokio::spawn(prover_fut);
    
    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(tls_connection).await?;
    
    // Spawn the connection to run in the background
    tokio::spawn(connection);
    
    // Send HTTP request
    println!("📤 Sending HTTP request to {}", target_url);
    let request = Request::builder()
        .uri(uri.clone())
        .header("Host", server_domain)
        .header("Connection", "close")
        .header("User-Agent", "TLSNotary-Prover/1.0")
        .method("GET")
        .body(Empty::<Bytes>::new())?;
    let response = request_sender.send_request(request).await?;
    
    println!("📥 Received HTTP response: {}", response.status());
    assert!(response.status() == StatusCode::OK);
    
    // Read response body
    let body = hyper::body::to_bytes(response.into_body()).await?;
    println!("📥 Response body length: {} bytes", body.len());
    
    // Wait for prover to complete
    let mut prover = prover_task.await??;
    
    println!("✅ TLS connection completed. Creating proof...");
    
    // Create proof for the Verifier
    let mut builder = ProveConfig::builder(prover.transcript());
    
    // Reveal the DNS name
    builder.server_identity();
    
    // Reveal all sent data
    builder.reveal_sent(&(0..prover.transcript().sent().len()))?;
    
    // Reveal all received data
    builder.reveal_recv(&(0..prover.transcript().received().len()))?;
    
    let config = builder.build()?;
    
    prover.prove(&config).await?;
    prover.close().await?;
    
    // Close the session
    handle.close();
    driver_task.await??;
    
    // Wait for bridges to complete
    let _ = bridge_ws_to_session.await;
    let _ = bridge_session_to_ws.await;
    
    println!("\n🎉 Proof created and sent to verifier!");
    
    Ok(())
}
