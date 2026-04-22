use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::StreamExt; // gives us .next() await
use futures_util::SinkExt; // gives us .send() await
use std::net::SocketAddr;
use std::time::Duration;
use tracing:: {error, info, warn};

#[tokio::main]
async fn main() {
    // set up logging - shows INFO and above in the terminal
    // Python equivalent: logging.basicConfig(level=logging.INFO)
    tracing_subscriber::fmt::init();


    // bind to address
    let listener = TcpListener::bind("0.0.0.0:9001")
    .await
    .expect("Failed to bind - is port 9001 already in use?");

    info!(" Async WebSocket Echo Server Listening on ws://0.0.0.0:9001");

    // spawn a task that just waits for CTRL+C
    // when it fires, the whole async runtime shuts down
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    // pin is need because ctrl_c() returns a future that must not
    // move in memory once we start polling it inside select

    // accept connection
    loop {
        tokio::select! {
            // branch 1: new client arrived
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                        info!(peer = %peer, "New TCP connection");
                        tokio::spawn(handle_client(stream, peer));
                    }
                    Err(e) => {
                        error!(error = %e, "Accept Error");
                    }
                }
            }
            // branch 2: CTRL+C pressed
            _ = &mut ctrl_c => {
                info!("CTRL+C receivved - shutting down gracefully...");
                break; // exit the main loop server stops accepting
            }
        }
    }
    info!("Server stopped");
}

async fn handle_client(stream: TcpStream, peer: SocketAddr){
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            println!("[{peer}] Handshake failed: {e}");
            return; // just drop this client, server keeps running
        }
    };
    info!(peer = %peer, "WebSocket connected");

    // split into separate sender and receiver
    let (mut sender, mut receiver) = ws_stream.split();
    // we split because we need two mutable references:
    // one for reading, one for writing - Rust won't allow both on same variable

    // heartbeat - send a ping every 20 seconds
    // if the client is dead, the next send recv will fail
    let mut ping_interval = tokio::time::interval(Duration::from_secs(20));
    ping_interval.tick().await; // consume the first instant tick

    loop {
        tokio::select! {
            // branch 1: message received from client
            result = receiver.next() => {
                match result {
                    Some(Ok(msg)) => {
                        if !handle_message(&mut sender, peer, msg).await {
                            break; // handle_message returns false -> close connection
                        }
                    }
                    Some(Err(e)) => {
                        warn!(peer = %peer, error= %e, "Receive error");
                        break;
                    }
                    None => {
                        info!(peer = %peer, "client stream ended");
                        break;
                    }
                }
            }
            // branch 2: ping time fired
            _ = ping_interval.tick() => {
                info!(peer = %peer, "sending heartbeat Ping");
                if sender.send(Message::Ping(vec![].into())).await.is_err(){
                    warn!(peer = %peer, "client gone - Ping failed");
                    break;
                }
            }
        }
    }

    info!(peer = %peer, "🔌 Disconnected. Task cleaned up.");
}

// Returns true  → keep the connection
// Returns false → close the connection
async fn handle_message(
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    peer: SocketAddr,
    msg: Message,
) -> bool {
    match msg {
        Message::Text(text) => {
            info!(peer = %peer, msg = %text, "📨 Text");
            sender.send(Message::Text(text)).await.is_ok()
        }
        Message::Binary(data) => {
            info!(peer = %peer, bytes = data.len(), "📦 Binary");
            sender.send(Message::Binary(data)).await.is_ok()
        }
        Message::Ping(payload) => {
            // respond with Pong manually (tokio-tungstenite doesn't auto-pong)
            sender.send(Message::Pong(payload)).await.is_ok()
        }
        Message::Close(_) => {
            info!(peer = %peer, "👋 Close frame received");
            false // signal to break the loop
        }
        _ => true, // Pong, Frame — ignore, keep connection
    }
}