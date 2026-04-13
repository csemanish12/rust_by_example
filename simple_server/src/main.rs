use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};


fn main() {
    // 1. Bind the listener to an address
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    println!("Server running on port 7878");
    
    // 2. Iterate over incoming connections
    for stream in listener.incoming() {
        let _stream = stream.unwrap();
        handle_connection(_stream);
        println!("Connection Established");
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<String> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:#?}", http_request);
    let status_line = "HTTP/1.1 200 OK";
    let contents = "<html><body><h1>Hello from Rust!</h1></body></html>";
    let length = contents.len();

    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    );
    stream.write_all(response.as_bytes()).unwrap();

}
