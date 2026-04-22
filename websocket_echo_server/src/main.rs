use std::net::TcpListener;
use tungstenite::accept;
use tungstenite::Message;

// Bring our ThreadPool into scope from lib.rs
use websocket_echo_server::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:9001").unwrap();

    // Create a fixed pool of 4 workers.
    // No matter how many clients connect, only 4 threads ever exist.
    // Python equivalent: ThreadPoolExecutor(max_workers=4)
    let pool = ThreadPool::new(4);

    println!("🚀 WebSocket Echo Server listening on ws://127.0.0.1:9001");
    println!("   Workers : 4 (fixed thread pool)");
    println!("   Test with: wscat -c ws://127.0.0.1:9001");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let peer   = stream.peer_addr().unwrap();
        println!("\n[{peer}] TCP connection → sending to thread pool...");

        // Send the entire client session to a free worker.
        // 'move' transfers ownership of both 'stream' and 'peer' into the closure.
        // Python equivalent: pool.submit(handle_client, stream)
        pool.execute(move || {
            handle_client(stream, peer);
        });
    }
    // pool drops here → graceful shutdown (all clients finish before exit)
}

// ---------------------------------------------------------------------------
// handle_client — runs inside a worker thread, owns one WebSocket connection
// ---------------------------------------------------------------------------
fn handle_client(stream: std::net::TcpStream, peer: std::net::SocketAddr) {
    // Upgrade TCP → WebSocket (performs the HTTP handshake)
    let mut websocket = match accept(stream) {
        Ok(ws) => ws,
        Err(e) => {
            println!("[{peer}] Handshake failed: {e}");
            return; // return from the worker closure — worker becomes free again
        }
    };

    println!("[{peer}] ✅ WebSocket handshake complete!");

    // Echo loop — runs for the entire lifetime of this client's connection
    loop {
        let message = match websocket.read() {
            Ok(msg) => msg,
            Err(e) => {
                println!("[{peer}] Disconnected: {e}");
                break;
            }
        };

        match message {
            Message::Text(ref text) => {
                println!("[{peer}] 📨 \"{text}\"");
                websocket.send(message.clone()).unwrap();
                println!("[{peer}] 📤 Echoed.");
            }
            Message::Binary(ref bytes) => {
                println!("[{peer}] 📦 {} bytes", bytes.len());
                websocket.send(message.clone()).unwrap();
                println!("[{peer}] 📤 Echoed.");
            }
            Message::Ping(_) => {
                println!("[{peer}] 🏓 Ping (Pong auto-sent).");
            }
            Message::Close(_) => {
                println!("[{peer}] 🔌 Close frame received. Goodbye!");
                let _ = websocket.send(Message::Close(None));
                break;
            }
            _ => {}
        }
    }

    println!("[{peer}] Connection closed. Worker is now free.\n");
    // worker thread loops back to recv() — ready for the next client
}