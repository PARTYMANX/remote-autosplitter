use std::sync::mpsc;

use iced::{
    Font, Length, Subscription, Task, futures::Stream, widget::{Column, button, column, container, row, scrollable, text, text_editor, text_input}, window
};

use crate::remote::{LiveSplitServerMessage, Remote, RoutedMessage, UIMessage};

pub fn run_remote_app(autosplitter_filepath: String, server_url: String) {
    let boot = move || {
        let remote = Remote::new(autosplitter_filepath.clone(), server_url.clone());

        let app = RemoteApp::new(remote.get_message_sender());

        let handler = MessageHandler::new(remote);

        (app, iced::Task::run(handler, RemoteApp::map_ui_message))
    };
    let app = iced::application(boot, RemoteApp::update, RemoteApp::view)
        .exit_on_close_request(false)
        .subscription(RemoteApp::subscription)
        .title("Remote Autosplitter");

    app.run().unwrap();

    println!("Done!");
}

pub struct RemoteApp {
    value: i32,
    server_address: String,
    log_text: Vec<String>,
    sender: mpsc::Sender<RoutedMessage>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Increment,
    Decrement,
    HandleMessages,
    NoOp,

    LoadSplitter,
    Log(String),
    WindowEvent((window::Id, window::Event)),

    AddressEdit(String),
    AddressSubmit,
}

impl RemoteApp {
    pub fn new(sender: mpsc::Sender<RoutedMessage>) -> Self {
        let mut log_text = Vec::new();
        for i in 0..100 {
            log_text.push(format!("Log Test Line {}", i));
        }

        Self {
            value: 0,
            server_address: "".to_string(),
            log_text,
            sender,
        }
    }

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

            button("Load Splitter").on_press(Message::LoadSplitter),

            row![
                text_input("Server Address", &self.server_address)
                    .on_input(Message::AddressEdit)
                    .on_paste(Message::AddressEdit)
                    .on_submit(Message::AddressSubmit),
                button("Connect")
                    .on_press(Message::AddressSubmit),
            ],
            
            container(
                scrollable(
                    column(self.log_text.iter().map(|s| text(s).font(log_font).into()))
                ).width(Length::Fill)
                .height(Length::Fill)
                .anchor_bottom()
            ).padding(16),
            
        ]
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Increment => {
                self.value += 1;

                Task::none()
            }
            Message::Decrement => {
                self.value -= 1;

                Task::none()
            }
            Message::HandleMessages => {
                println!("Test!\nMore test");

                Task::none()
            }
            Message::LoadSplitter => {
                let filepath = rfd::FileDialog::new()
                    .set_title("Select ASR File")
                    .add_filter("wasm", &["wasm"])
                    .add_filter("All Files", &["*"])
                    .set_directory("/")
                    .pick_file();

                if let Some(path) = filepath {
                    let str = path.to_str().unwrap().to_owned();

                    self.sender.send(RoutedMessage::Autosplitter(crate::remote::AutosplitterMessage::ChangeFile(str))).unwrap();
                }

                Task::none()
            }
            Message::AddressEdit(text) => {
                self.server_address = text;

                Task::none()
            }
            Message::AddressSubmit => {
                self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::ChangeAddress(self.server_address.clone()))).unwrap();

                Task::none()
            }
            Message::Log(msg) => {
                self.log_text.push(msg);

                Task::none()
            }
            Message::WindowEvent((id, event)) => {
                match event {
                    window::Event::CloseRequested => {
                        self.sender.send(RoutedMessage::Quit).unwrap();

                        window::close(id)
                    },
                    window::Event::FileDropped(path) => {
                        let str = path.to_str().unwrap().to_owned();

                        self.sender.send(RoutedMessage::Autosplitter(crate::remote::AutosplitterMessage::ChangeFile(str))).unwrap();

                        Task::none()
                    },
                    _ => {
                        Task::none()
                    }
                }
            }
            Message::NoOp => {
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        window::events().map(Message::WindowEvent)
    }

    fn map_ui_message(msg: UIMessage) -> Message {
        match msg {
            UIMessage::Log(s) => {
                Message::Log(s)
            },
            UIMessage::AutosplitterStatus(autosplitter_status) => {
                println!("Autosplitter Status!");
                Message::Increment
            },
            UIMessage::ConnectionStatus(connection_status) => {
                println!("Connection Status!");
                Message::Increment
            },
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

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match self.remote.get_message() {
            Some(v) => match v {
                UIMessage::Stop => std::task::Poll::Ready(None),
                _ => std::task::Poll::Ready(Some(v)),
            },
            None => {
                self.remote.get_message_sender().send(RoutedMessage::SetWaker(cx.waker().clone())).unwrap();
                std::task::Poll::Pending
            }
        }
    }
}