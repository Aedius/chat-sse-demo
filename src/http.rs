use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::str;
use std::sync::mpsc::Sender;
use std::thread;

use serde::{Deserialize, Serialize};
use serde_json::Result as SerdeResult;

use crate::multiplexer::MMess;
use crate::sse::Message;

#[derive(Serialize, Deserialize, Debug)]
struct ChatMess {
    name: String,
    mess: String,
}

pub fn listen(tcp_address: &str, sender: Sender<MMess<Message>>) -> () {
    let listener = TcpListener::bind(tcp_address).unwrap();

    println!("listen on {}", tcp_address);
    for stream in listener.incoming() {
        let sender = sender.clone();

        match stream {
            Ok(str) => {
                let sender = sender.clone();
                thread::spawn(
                    || {
                        handle_connection(str, sender);
                    }
                );
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
}


fn handle_connection(mut stream: TcpStream, sender: Sender<MMess<Message>>) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let mess = b"PUT /mess HTTP/1.1\r\n";

    if buffer.starts_with(mess) {
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

fn decode_chat_mess(row: &str) -> SerdeResult<ChatMess> {
    let json = row.trim_end_matches("\u{0}");

    let m: ChatMess = serde_json::from_str(json)?;
    Ok(m)
}