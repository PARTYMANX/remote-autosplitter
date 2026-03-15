mod autosplitter;
mod client;
mod message;

use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
};

pub use message::{
    AutosplitterMessage, AutosplitterStatus, AutosplitterSetting, SettingType, AutosplitterComboboxChoice, ConnectionStatus, LiveSplitServerMessage,
    RoutedMessage, UIMessage,
};

use crate::remote::{autosplitter::Autosplitter, client::LiveSplitClient, message::MessageRouter};

pub struct Remote {
    router_thread: Option<JoinHandle<()>>,
    client_thread: Option<JoinHandle<()>>,
    autosplitter_thread: Option<JoinHandle<()>>,
    ui_receiver: mpsc::Receiver<UIMessage>,
    router_sender: mpsc::Sender<RoutedMessage>,
}

impl Remote {
    pub fn new(filepath: String, address: String) -> Self {
        let (router_sender, router_receiver) = mpsc::channel();

        let (client_sender, client_receiver) = mpsc::channel();
        let mut client = LiveSplitClient::new(address, client_receiver, router_sender.clone());

        let client_thread = thread::spawn(move || client.run());

        let (autosplitter_sender, autosplitter_receiver) = mpsc::channel();
        let mut autosplitter =
            Autosplitter::new(filepath, autosplitter_receiver, router_sender.clone());

        let autosplitter_thread = thread::spawn(move || autosplitter.run());

        let (ui_sender, ui_receiver) = mpsc::channel();

        let mut router = MessageRouter::new(
            client_sender,
            autosplitter_sender,
            ui_sender,
            router_receiver,
        );

        let router_thread = thread::spawn(move || router.run());

        Self {
            router_thread: Some(router_thread),
            client_thread: Some(client_thread),
            autosplitter_thread: Some(autosplitter_thread),
            ui_receiver,
            router_sender: router_sender,
        }
    }

    pub fn get_message(&mut self) -> Option<UIMessage> {
        match self.ui_receiver.try_recv() {
            Ok(v) => Some(v),
            Err(e) => match e {
                mpsc::TryRecvError::Empty => None,
                mpsc::TryRecvError::Disconnected => todo!(),
            },
        }
    }

    pub fn get_message_sender(&self) -> mpsc::Sender<RoutedMessage> {
        self.router_sender.clone()
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        if let Some(router_thread) = self.router_thread.take() {
            router_thread.join().unwrap();
        }

        if let Some(autosplitter_thread) = self.autosplitter_thread.take() {
            autosplitter_thread.join().unwrap();
        }

        if let Some(client_thread) = self.client_thread.take() {
            client_thread.join().unwrap();
        }
    }
}
