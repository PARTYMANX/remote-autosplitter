mod autosplitter;
mod client;
mod message;
mod ui;

use std::{env, sync::mpsc, thread};

use crate::{autosplitter::Autosplitter, client::LiveSplitClient, message::{AutosplitterMessage, LiveSplitServerMessage, RoutedMessage}, ui::UI};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: remote-autosplitter <autosplitter wasm path> <address:port>");
        return;
    }

    let filepath = args.get(1).unwrap().to_owned();
    let address = args.get(2).unwrap().to_owned();

    let (main_sender, main_receiver) = mpsc::channel();

    let (client_sender, client_receiver) = mpsc::channel();
    let client = LiveSplitClient::new(address, client_receiver, main_sender.clone());

    let client_thread = thread::spawn(move || client.run());

    let (autosplitter_sender, autosplitter_receiver) = mpsc::channel();
    let autosplitter = Autosplitter::new(filepath, autosplitter_receiver, main_sender.clone());

    let autosplitter_thread = thread::spawn(move || autosplitter.run());

    let (ui_sender, ui_receiver) = mpsc::channel();
    let mut ui = UI::new(ui_receiver, main_sender.clone());

    let ui_thread = thread::spawn(move || ui.run());

    loop {
        let dst = main_receiver.recv().unwrap();
        match dst {
            RoutedMessage::Client(msg) => client_sender.send(msg).unwrap(),
            RoutedMessage::Autosplitter(msg) => autosplitter_sender.send(msg).unwrap(),
            RoutedMessage::UI(msg) => ui_sender.send(msg).unwrap(),
            RoutedMessage::Quit => {
                client_sender.send(LiveSplitServerMessage::Stop).unwrap();
                autosplitter_sender.send(AutosplitterMessage::Stop).unwrap();
                break
            },
        }
    }

    ui_thread.join().unwrap();
    autosplitter_thread.join().unwrap();
    client_thread.join().unwrap();
}
