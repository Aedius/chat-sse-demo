use std::sync::mpsc::{Receiver, Sender};

pub enum MMess<A> {
    Message(A),
    Sender(Sender<A>),
}

pub struct Multiplex<A> {
    receiver: Receiver<MMess<A>>,
    sender_list: Vec<Sender<A>>,
}


impl<A: Clone> Multiplex<A> {
    pub fn new(receiver: Receiver<MMess<A>>) -> Multiplex<A> {
        Multiplex {
            receiver,
            sender_list: Vec::new(),
        }
    }

    pub fn listen(mut self) -> ! {
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