use std::io::prelude::*;
use std::net::TcpStream;
use std::net::TcpListener;
use std::fs;
use std::thread;
use std::time::Duration;
use chrono;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

pub struct Message {
    name: String,
    content: String,
}


fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let (sender, receiver) = channel();

    thread::spawn(
        move || {
            loop {
                let date = chrono::offset::Utc::now();

                let mess = Message {
                    name: "ping".to_string(),
                    content: format!("{}{}", "This is a message at time ", date),
                };

                match sender.send(mess){
                    Ok(()) => {},
                    Err(e) => {
                        println!("cannot send {}", e);
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    );

    let receiver= Arc::new(Mutex::new(receiver)) ;


    for stream in listener.incoming() {
        let receiver_move = receiver.clone();
        match stream {
            Ok(str) => {
                thread::spawn(
                    || {
                        handle_connection(str, receiver_move);
                    }
                );
            }
            Err(err) => {
                println!("{}", err)
            }
        }
    }
}



fn handle_connection(mut stream: TcpStream, receiver: Arc<Mutex<Receiver<Message>>>) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
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

pub fn process_sse(stream: &mut TcpStream, mutex: Arc<Mutex<Receiver<Message>>>) -> Result<(), std::io::Error> {
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

        match mutex.try_lock(){
            Ok(receiver) => {
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
            },
            Err(e) => {
                println!("error receiving : {}", e);
            }
        }
    }
}