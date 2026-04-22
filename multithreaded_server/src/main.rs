use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

// Bring our ThreadPool into scope from lib.rs
use multithreaded_server::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    // Create a pool of exactly 4 worker threads — no more will EVER be spawned.
    // Python equivalent: ThreadPoolExecutor(max_workers=4)
    let pool = ThreadPool::new(4);

    println!("🚀 Thread Pool Server running on http://127.0.0.1:7878");
    println!("   Workers: 4 (fixed — no unlimited spawning)");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        // Send the job to the pool — a free worker will pick it up.
        // Python equivalent: pool.submit(handle_connection, stream)
        pool.execute(move || {
            handle_connection(stream);
        });
    }
    // pool goes out of scope here → Drop runs → graceful shutdown
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    // Read the first line of the HTTP request (e.g. "GET / HTTP/1.1")
    let request_line = buf_reader
        .lines()
        .next()           // only grab the first line
        .unwrap()         // unwrap the Option (there is a line)
        .unwrap();        // unwrap the Result (the line is valid UTF-8)

    println!(
        "[Thread {:?}] Got request: {}",
        thread::current().id(),
        request_line
    );

    // Simulate a slow request (2 seconds) so we can see threads overlapping
    thread::sleep(Duration::from_secs(2));

    let status_line = "HTTP/1.1 200 OK";
    let contents = "<html><body><h1>Hello from Naive Threaded Rust Server!</h1></body></html>";
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();

    println!("[Thread {:?}] Done.", thread::current().id());
}
