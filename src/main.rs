use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use http::decode_chat_mess;
use multiplexer::{MMess, Multiplex};
use sse::{Message, process_sse};

mod multiplexer;

mod heart;

mod sse;

mod http;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("listen on 8080");

    let (sender, receiver) = channel();

    let multiplex = Multiplex::new(receiver);

    thread::spawn(
        || multiplex.listen()
    );

    let sender_clock = sender.clone();
    thread::spawn(
        move || {
            heart::start(sender_clock)
        }
    );

    for stream in listener.incoming() {
        let sender = sender.clone();
        let (sender_stream, receiver_stream) = channel();

        match sender.send(MMess::Sender(sender_stream)) {
            Ok(()) => {}
            Err(err) => {
                println!("{}", err)
            }
        }

        match stream {
            Ok(str) => {
                let sender = sender.clone();
                thread::spawn(
                    || {
                        handle_connection(str, receiver_stream, sender);
                    }
                );
            }
            Err(err) => {
                println!("{}", err)
            }
        }
    }
}


fn handle_connection(mut stream: TcpStream, receiver: Receiver<Message>, sender: Sender<MMess<Message>>) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sse = b"GET /sse HTTP/1.1\r\n";
    let mess = b"PUT /mess HTTP/1.1\r\n";

    if buffer.starts_with(sse) {
        let res = process_sse(&mut stream, receiver);
        match res {
            Err(e) => {
                println!("cannot write stream : {}", e);
                return;
            }
            Ok(_) => (),
        }
    } else if buffer.starts_with(mess) {
        match str::from_utf8(&buffer) {
            Ok(text) => {
                let mut lines = text.lines();
                let mut sep_found = false;
                while !sep_found {
                    match lines.next() {
                        Some(row) => {
                            if row == "" {
                                sep_found = true
                            }
                        }
                        None => {
                            sep_found = true
                        }
                    }
                }
                match lines.next() {
                    Some(row) => {
                        match decode_chat_mess(row) {
                            Ok(m) => {
                                println!("chat message : {:?}", m);

                                let new_mess = MMess::Message(Message {
                                    name: "chat".to_string(),
                                    content: serde_json::to_string(&m).unwrap(),
                                });

                                match sender.send(new_mess) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("cannot send mess {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("cannot json decode {:?} ; {}", row, e);
                            }
                        }
                    }
                    None => {
                        println!("No content");
                    }
                }
            }
            Err(e) => {
                println!("error decoding {:?}, {}", str::from_utf8(&buffer[..512]), e);
            }
        }
    } else {
        let (status_line, filename) = if buffer.starts_with(get) {
            ("HTTP/1.1 200 OK\r\n\r\n", "static/index.html")
        } else {
            ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "static/404.html")
        };

        let contents = fs::read_to_string(filename).unwrap();

        let response = format!("{}{}", status_line, contents);
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

