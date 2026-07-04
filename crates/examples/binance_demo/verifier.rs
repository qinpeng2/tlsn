use anyhow::Result;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tlsn::{
    config::verifier::VerifierConfig,
    verifier::VerifierOutput,
    webpki::RootCertStore,
    Session,
};
use async_tungstenite::{
    tokio::accept_hdr_async,
    tungstenite::handshake::server::{Request, Response},
    tungstenite::Message,
};
use futures::{SinkExt, StreamExt};

// Maximum number of bytes that can be sent from prover to server.
const MAX_SENT_DATA: usize = 1 << 16; // 64KB
// Maximum number of bytes that can be received by prover from server.
const MAX_RECV_DATA: usize = 1 << 20; // 1MB

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr: SocketAddr = std::env::var("VERIFIER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    println!("🚀 Verifier starting on {}", addr);
    println!("📡 Waiting for Prover to connect via WebSocket...");
    println!("🔐 Using real Garbled Circuits and Oblivious Transfer (OT)");
    println!("⚠️  Make sure tlsn_insecure feature is NOT enabled\n");

    let listener = TcpListener::bind(addr).await?;
    
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        println!("✅ New connection from {}", peer_addr);
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("❌ Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) -> Result<()> {
    // Accept WebSocket connection
    let callback = |req: &Request, mut response: Response| {
        println!("📥 WebSocket handshake request: {}", req.uri());
        Ok(response)
    };
    
    let ws_stream = accept_hdr_async(stream, callback).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    // Create a duplex stream for the Session
    let (session_io_read, session_io_write) = tokio::io::duplex(1 << 20);
    
    // Bridge WebSocket to session IO
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
    
    // Bridge session IO to WebSocket
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
    
    // Create session
    let session = Session::new(session_io_read.compat());
    let (driver, mut handle) = session.split();
    
    // Spawn the session driver
    let driver_task = tokio::spawn(driver);
    
    // Create verifier
    let verifier_config = VerifierConfig::builder()
        .root_store(RootCertStore::mozilla())
        .build()?;
    let verifier = handle.new_verifier(verifier_config)?;
    
    println!("🔍 Verifying TLS commitment protocol configuration...");
    
    // Validate and accept the commitment
    let verifier = verifier.commit().await?;
    
    // Check configuration limits
    let reject = if let tlsn::config::tls_commit::TlsCommitProtocolConfig::Mpc(mpc_tls_config) = verifier.request().protocol() {
        if mpc_tls_config.max_sent_data() > MAX_SENT_DATA {
            Some("max_sent_data is too large")
        } else if mpc_tls_config.max_recv_data() > MAX_RECV_DATA {
            Some("max_recv_data is too large")
        } else {
            None
        }
    } else {
        Some("expecting to use MPC-TLS")
    };
    
    if let Some(reason) = reject {
        println!("❌ Rejecting configuration: {}", reason);
        verifier.reject(Some(reason)).await?;
        return Err(anyhow::anyhow!("protocol configuration rejected: {}", reason));
    }
    
    println!("✅ Configuration accepted. Running TLS commitment protocol...");
    
    // Run the TLS commitment protocol
    let verifier = verifier.accept().await?.run().await?;
    
    println!("✅ TLS commitment protocol completed!");
    println!("🔍 Verifying proof...");
    
    // Verify the proof
    let verifier = verifier.verify().await?;
    
    // Check that server identity was revealed
    if !verifier.request().server_identity() {
        let verifier = verifier
            .reject(Some("expecting to verify the server name"))
            .await?;
        verifier.close().await?;
        return Err(anyhow::anyhow!("prover did not reveal the server name"));
    }
    
    let (
        VerifierOutput {
            server_name,
            transcript,
            ..
        },
        verifier,
    ) = verifier.accept().await?;
    
    verifier.close().await?;
    
    // Close the session
    handle.close();
    driver_task.await??;
    
    // Wait for bridges
    let _ = bridge_ws_to_session.await;
    let _ = bridge_session_to_ws.await;
    
    let server_name = server_name.expect("prover should have revealed server name");
    let transcript = transcript.expect("prover should have revealed transcript data");
    
    println!("\n🎉 Verification successful!");
    println!("📋 Server: {:?}", server_name);
    println!("📤 Sent data length: {} bytes", transcript.sent_unsafe().len());
    println!("📥 Received data length: {} bytes", transcript.received_unsafe().len());
    
    // Print a preview of the data
    let sent_preview = String::from_utf8_lossy(&transcript.sent_unsafe()[..transcript.sent_unsafe().len().min(200)]);
    let recv_preview = String::from_utf8_lossy(&transcript.received_unsafe()[..transcript.received_unsafe().len().min(200)]);
    
    println!("\n📤 Sent data preview:\n{}", sent_preview);
    println!("\n📥 Received data preview:\n{}", recv_preview);
    
    Ok(())
}
