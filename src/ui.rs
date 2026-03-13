use std::sync::mpsc;

use iced::{
    Font, Length, Program, Subscription, application::{BootFn, IntoBoot}, futures::Stream, widget::{Column, button, column, container, scrollable, text, text_editor, text_input}
};

use crate::remote::{Remote, RoutedMessage, UIMessage};

pub fn run_remote_app(autosplitter_filepath: String, server_url: String) {
    let boot = move || {
        let remote = Remote::new(autosplitter_filepath.clone(), server_url.clone());

        let app = RemoteApp::new(remote.get_message_sender());

        let handler = MessageHandler::new(remote);

        (app, iced::Task::run(handler, RemoteApp::map_ui_message))
    };
    let app = iced::application(boot, RemoteApp::update, RemoteApp::view)
        .title("Remote Autosplitter");

    app.run().unwrap();
}

pub struct RemoteApp {
    value: i32,
    log_text: Vec<String>,
    sender: mpsc::Sender<RoutedMessage>,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
    HandleMessages,
    NoOp,
}

impl RemoteApp {
    pub fn view(&self) -> Column<Message> {
        let log_font = Font {
            family: iced::font::Family::Monospace,
            ..Default::default()
        };

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
            
            container(
                scrollable(
                    column(self.log_text.iter().map(|s| text(s).font(log_font).into()))
                ).width(Length::Fill)
                .height(Length::Fill)
            ).padding(16),
            
        ]
    }
}

impl RemoteApp {
    pub fn new(sender: mpsc::Sender<RoutedMessage>) -> Self {
        let mut log_text = Vec::new();
        for i in 0..100 {
            log_text.push(format!("Log Test Line {}", i));
        }

        Self {
            value: 0,
            log_text,
            sender,
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
                println!("Test!\nMore test");
            }
            Message::NoOp => {}
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let interval = iced::time::Duration::from_secs_f64(1.0 / 60.0); // TODO: make this only go off when there is a new message
        iced::time::every(interval).map(|_| Message::HandleMessages)
    }

    fn map_ui_message(msg: UIMessage) -> Message {
        match msg {
            UIMessage::Log(s) => {
                println!("Log! {}", s);
                Message::Increment
            },
            UIMessage::AutosplitterStatus(autosplitter_status) => Message::Increment,
            UIMessage::ConnectionStatus(connection_status) => Message::Increment,
            UIMessage::Stop => Message::NoOp,
        }
    }
}

struct MessageHandler {
    remote: Remote,
}

impl MessageHandler {
    fn new(remote: Remote) -> Self {
        Self {
            remote,
        }
    }
}

impl Stream for MessageHandler {
    type Item = UIMessage;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match self.remote.get_message() {
            Some(v) => std::task::Poll::Ready(Some(v)),
            None => std::task::Poll::Pending,
        }
    }
}