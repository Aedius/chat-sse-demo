use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;

#[derive(Clone)]
pub struct Message {
    pub name: String,
    pub content: String,
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