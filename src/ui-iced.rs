use std::{sync::mpsc, time};

use crate::message::{AutosplitterStatus, ConnectionStatus, RoutedMessage, UIMessage};

/*pub struct UI {
    tick_duration: time::Duration,
    receiver: mpsc::Receiver<UIMessage>,
    sender: mpsc::Sender<RoutedMessage>,
    splitter_status: AutosplitterStatus,
    connection_status: ConnectionStatus,
    log_lines: Vec<String>,
    log_front: u32,
}

impl UI {
    pub fn new(receiver: mpsc::Receiver<UIMessage>, sender: mpsc::Sender<RoutedMessage>) -> Self {
        Self {
            tick_duration: time::Duration::from_secs_f64(1.0 / 60.0),
            receiver,
            sender,
            splitter_status: AutosplitterStatus::NotRunning,
            connection_status: ConnectionStatus::Disconnected,
            log_lines: Vec::new(),
            log_front: 0,
        }
    }

    pub fn run(&mut self) {
        let app = iced::application(|| MessageReceiver::new(self.receiver), Counter::update, Counter::view)
            //.subscription(state)
            .title("Remote Autosplitter");

        let _ = app.run();

        self.sender.send(RoutedMessage::Quit).unwrap();
    }
}*/

struct MessageReceiverOther {
    receiver: u32,
}

struct MessageReceiver {
    receiver: MessageReceiverOther
}

impl MessageReceiver {
    fn new(receiver: u32) -> Self {
        Self {
            receiver,
        }
    }
}

impl BootFn<Counter, Message> for MessageReceiver {
    fn boot(&self) -> (Counter, iced::Task<Message>) {
        (Counter::new(), iced::Task::run(self.receiver, handle_message))
    }
}

impl Stream for MessageReceiverOther {
    type Item = UIMessage;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

fn handle_message(msg: UIMessage) -> Message {
    match msg {
        UIMessage::Log(_) => Message::Increment,
        UIMessage::AutosplitterStatus(autosplitter_status) => Message::Increment,
        UIMessage::ConnectionStatus(connection_status) => Message::Increment,
        UIMessage::Stop => todo!(),
    }
}

pub fn run_ui(receiver: mpsc::Receiver<UIMessage>, sender: mpsc::Sender<RoutedMessage>) {
    let task = iced::Task::run(MessageReceiver::new(receiver), handle_message);
    let app = iced::application(move || (Counter::new(), task), Counter::update, Counter::view)
        //.subscription(Counter::subscription)
        .title("Remote Autosplitter");

    let _ = app.run();
}

struct Counter {
    value: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
    HandleMessages,
}

use iced::{
    Program, Subscription, application::{BootFn, IntoBoot}, futures::Stream, widget::{Column, button, column, text}
};

impl Counter {
    pub fn view(&self) -> Column<Message> {
        // We use a column: a simple vertical layout
        column![
            // The increment button. We tell it to produce an
            // `Increment` message when pressed
            button("+").on_press(Message::Increment),
            // We show the value of the counter here
            text(self.value).size(50),
            // The decrement button. We tell it to produce a
            // `Decrement` message when pressed
            button("-").on_press(Message::Decrement),
        ]
    }
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: 0,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
            }
            Message::Decrement => {
                self.value -= 1;
            }
            Message::HandleMessages => {
                println!("Test!");
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let interval = iced::time::Duration::from_secs_f64(1.0 / 60.0); // TODO: make this only go off when there is a new message
        iced::time::every(interval).map(|_| Message::HandleMessages)
    }
}

/*impl Drop for Counter {
    fn drop(&mut self) {
        self.sender.send(RoutedMessage::Quit).unwrap();
    }
}*/
