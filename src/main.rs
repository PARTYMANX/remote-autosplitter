mod autosplitter;
mod client;
mod message;

use std::{env, sync::mpsc, thread};

use crate::{autosplitter::Autosplitter, client::LiveSplitClient};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: remote-autosplitter <autosplitter wasm path> <address:port>");
        return;
    }

    let filepath = args.get(1).unwrap().to_owned();
    let address = args.get(2).unwrap().to_owned();

    let (main_sender, main_receiver) = mpsc::channel();

    let (client_sender, client_reciever) = mpsc::channel();
    let client = LiveSplitClient::new(address, client_reciever, main_sender.clone());

    let client_thread = thread::spawn(move || client.run());

    let (autosplitter_sender, autosplitter_reciever) = mpsc::channel();
    let autosplitter = Autosplitter::new(filepath, autosplitter_reciever, main_sender.clone());

    let autosplitter_thread = thread::spawn(move || autosplitter.run());

    loop {
        let dst = main_receiver.recv().unwrap();
        match dst {
            message::RoutedMessage::Client(msg) => client_sender.send(msg).unwrap(),
            message::RoutedMessage::Autosplitter(msg) => autosplitter_sender.send(msg).unwrap(),
        }
    }

    autosplitter_thread.join().unwrap();
    client_thread.join().unwrap();
}
