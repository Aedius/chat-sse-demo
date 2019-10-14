use std::sync::mpsc::channel;
use std::thread;

use multiplexer::Multiplex;

mod multiplexer;

mod heart;

mod sse;

mod http;

fn main() {


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

    let sender_http = sender.clone();
    thread::spawn(
        move || {
            http::listen("127.0.0.1:8080", sender_http);
        }
    );
    sse::listen("127.0.0.1:32000", sender);
}




