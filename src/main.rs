use std::io::prelude::*;
use std::net::TcpStream;
use std::net::TcpListener;
use std::fs;
use std::thread;
use std::time::Duration;
use testsse::ThreadPool;
use chrono;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let pool = ThreadPool::new(100);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}


fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";
    let sse = b"GET /sse HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "static/index.html")
    } else if buffer.starts_with(sleep) {
        thread::sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK\r\n\r\n", "static/index.html")
    } else if buffer.starts_with(sse) {
        let headers = [
            "HTTP/1.1 200 OK",
            "Content-Type: text/event-stream",
            "Connection: keep-alive",
            "\r\n"
        ];
        let response = headers.join("\r\n")
            .to_string()
            .into_bytes();


        let res = stream.write(&response);
        match res {
            Err(e) => {
                println!("cannot write stream : {}", e);
                return;
            }
            Ok(len) => println!("write {} to stream", len)
        }
        loop {
            let response = "event: ping\r\n";
            let res = stream.write(response.as_bytes());
            match res {
                Err(e) => {
                    println!("cannot write stream : {}", e);
                    return;
                }
                Ok(len) => println!("write {} to stream", len)
            }
            let date = chrono::offset::Utc::now();
            let response = format!("{}{}{}", "data: This is a message at time ", date, "\r\n\r\n");
            let res = stream.write(response.as_bytes());
            match res {
                Err(e) => {
                    println!("cannot write stream : {}", e);
                    return;
                }
                Ok(len) => println!("write {} to stream", len)
            }
            stream.flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "static/404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();

    let response = format!("{}{}", status_line, contents);
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}