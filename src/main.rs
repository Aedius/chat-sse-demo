use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use chrono;
use serde::{Deserialize, Serialize};
use serde_json::Result as SerdeResult;

#[derive(Clone)]
pub struct Message {
    name: String,
    content: String,
}

pub enum MMess {
    Message(Message),
    Sender(Sender<Message>),
}

pub struct Multiplex {
    receiver: Receiver<MMess>,
    sender_list: Vec<Sender<Message>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMess {
    name: String,
    mess: String,
}

impl Multiplex {
    pub fn new(receiver: Receiver<MMess>) -> Multiplex {
        Multiplex {
            receiver,
            sender_list: Vec::new(),
        }
    }

    pub fn add_sender(mut self, sender: Sender<Message>) {
        self.sender_list.push(sender)
    }

    pub fn listen(mut self) {
        loop {
            match self.receiver.recv() {
                Ok(multi_message) => {
                    match multi_message {
                        MMess::Message(mess) => {
                            let mut dead_channel = Vec::new();
                            println!("new message ! Sending to {} chanel", &self.sender_list.len());
                            for (pos, sender) in self.sender_list.iter().enumerate() {
                                match sender.send(mess.clone()) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        dead_channel.push(pos);
                                        println!("multiplex cannot send {}, removing", e);
                                    }
                                }
                            }

                            // sort to remove valid index
                            dead_channel.sort_unstable_by(|a, b| b.cmp(a));
                            for pos in dead_channel {
                                self.sender_list.remove(pos);
                            }
                        }
                        MMess::Sender(sender) => {
                            println!("new Listener !");
                            self.sender_list.push(sender)
                        }
                    }
                }
                Err(e) => {
                    println!("multiplex error receiving : {}", e);
                }
            }
        }
    }
}


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
            loop {
                let date = chrono::offset::Utc::now();

                let mess = Message {
                    name: "ping".to_string(),
                    content: format!("{}", date),
                };

                match sender_clock.send(MMess::Message(mess)) {
                    Ok(()) => {}
                    Err(e) => {
                        println!("clock cannot send {}", e);
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
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


fn handle_connection(mut stream: TcpStream, receiver: Receiver<Message>, sender: Sender<MMess>) {
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

pub fn process_sse(stream: &mut TcpStream, receiver: Receiver<Message>) -> Result<(), std::io::Error> {
    let headers = [
        "HTTP/1.1 200 OK",
        "Content-Type: text/event-stream",
        "Connection: keep-alive",
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

pub fn decode_chat_mess(row: &str) -> SerdeResult<ChatMess> {
    let json = row.trim_end_matches("\u{0}");

    let m: ChatMess = serde_json::from_str(json)?;
    Ok(m)
}