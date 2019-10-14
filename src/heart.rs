use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::multiplexer::MMess;
use crate::sse::Message;

pub fn start(sender_clock: Sender<MMess<Message>>) -> ! {
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