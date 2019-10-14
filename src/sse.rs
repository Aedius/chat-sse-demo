use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::multiplexer::MMess;

#[derive(Clone)]
pub struct Message {
    pub name: String,
    pub content: String,
}

pub fn listen(tcp_address: &str, sender: Sender<MMess<Message>>) -> () {
    let listener = TcpListener::bind(tcp_address).unwrap();

    println!("listen on {}", tcp_address);
    for stream in listener.incoming() {
        let sender = sender.clone();
        let (sender_stream, receiver_stream) = channel();

        match sender.send(MMess::Sender(sender_stream)) {
            Ok(()) => {}
            Err(err) => {
                println!("{}", err);
            }
        }

        match stream {
            Ok(str) => {
                thread::spawn(
                    || {
                        handle_connection(str, receiver_stream);
                    }
                );
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
}

pub fn process_sse(stream: &mut TcpStream, receiver: Receiver<Message>) -> Result<(), std::io::Error> {
    let headers = [
        "HTTP/1.1 200 OK",
        "Content-Type: text/event-stream",
        "Connection: keep-alive",
        "Access-Control-Allow-Origin: *",
        "\r\n"
    ];
    let response = headers.join("\r\n")
        .to_string()
        .into_bytes();
    stream.write(&response)?;

    loop {
        match receiver.recv() {
            Ok(message) => {
                let response = format!("{}{}{}", "event: ", message.name, "\r\n");
                stream.write(response.as_bytes())?;
                let response = format!("{}{}{}", "data:", message.content, "\r\n\r\n");
                stream.write(response.as_bytes())?;
                match stream.flush() {
                    Ok(()) => {}
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                println!("error receiving : {}", e);
                return Ok(());
            }
        }
    }
}


fn handle_connection(mut stream: TcpStream, receiver: Receiver<Message>) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    let sse = b"GET /sse HTTP/1.1\r\n";

    if buffer.starts_with(sse) {
        let res = process_sse(&mut stream, receiver);
        match res {
            Err(e) => {
                println!("cannot write stream : {}", e);
                return;
            }
            Ok(_) => (),
        }
    }
}
