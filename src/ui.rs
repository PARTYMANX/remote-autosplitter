use std::{collections::HashMap, ops::Index, sync::mpsc};

use iced::{
    Element, Font, Length, Settings, Subscription, Task, color, futures::Stream, widget::{Container, button, checkbox, column, container, pick_list, row, scrollable, text, text_input, tooltip}, window
};

use crate::remote::{AutosplitterSetting, AutosplitterStatus, ConnectionStatus, LiveSplitServerMessage, Remote, RoutedMessage, UIMessage};

pub fn run_remote_app(autosplitter_filepath: String, server_url: String) {
    let boot = move || {
        let remote = Remote::new(autosplitter_filepath.clone(), server_url.clone());

        let app = RemoteApp::new(remote.get_message_sender());

        let handler = MessageHandler::new(remote);

        (app, iced::Task::run(handler, RemoteApp::map_ui_message))
    };

    let settings = Settings {
        default_text_size: iced::Pixels(14.0),
        ..Default::default()
    };

    let window_settings = window::Settings {
        min_size: Some(iced::Size {
            width: 640.0,
            height: 480.0,
        }),
        exit_on_close_request: false,
        ..Default::default()
    };

    let app = iced::application(boot, RemoteApp::update, RemoteApp::view)
        .subscription(RemoteApp::subscription)
        .settings(settings)
        .window(window_settings)
        .title("Remote Autosplitter");

    app.run().unwrap();

    println!("Done!");
}

#[derive(Clone)]
enum AutosplitterSettingValue {
    Checkbox(bool),
    Combobox(String),
    FilePicker(String),
}

pub struct RemoteApp {
    server_address: String,
    connection_status: ConnectionStatus,
    connect_button_awaiting_state: bool,

    autosplitter_path: String,
    autosplitter_status: AutosplitterStatus,
    autosplitter_settings: Vec<AutosplitterSetting>,
    autosplitter_current_settings: HashMap<String, AutosplitterSettingValue>,

    log_lines: RingBuffer<String>,

    sender: mpsc::Sender<RoutedMessage>,
}

#[derive(Debug, Clone)]
pub enum Message {
    WindowEvent((window::Id, window::Event)),

    AutosplitterPathEdit(String),
    AutosplitterPathSelect,
    AutosplitterRun,
    AutosplitterStop,
    AutosplitterSettingCheckbox(String, bool),
    AutosplitterSettingCombobox(String, String),
    AutosplitterSettingFilepathEdit(String, String),
    AutosplitterSettingFilepathSelect(String),

    AddressEdit(String),
    AddressSubmit,
    ConnectionDisconnect,

    Log(String),
    ConnectionStatus(ConnectionStatus),
    AutosplitterStatus(AutosplitterStatus),
    AutosplitterSettings(Vec<AutosplitterSetting>),

    NoOp,
}

impl RemoteApp {
    pub fn new(sender: mpsc::Sender<RoutedMessage>) -> Self {
        let mut log_lines = RingBuffer::new(1000);
        for i in 0..100 {
            log_lines.push(format!("Log Test Line really really really really really really really really really long {}", i));
        }

        Self {
            server_address: "".to_string(),
            connection_status: ConnectionStatus::Disconnected,
            connect_button_awaiting_state: false,
            autosplitter_path: "".to_string(),
            autosplitter_status: AutosplitterStatus::NotRunning,
            autosplitter_settings: Vec::new(),
            autosplitter_current_settings: HashMap::new(),
            log_lines,
            sender,
        }
    }

    pub fn view(&self) -> Container<'_, Message> {
        container(
            column![
                row![
                    self.view_autosplitter_panel(),

                    column![
                        self.view_connection_panel(),
                        
                        self.view_logs(),
                    ],
                ],
                self.view_status_bar(),
            ]
        )
    }

    fn view_connection_panel(&self) -> Container<'_, Message> {
        let (address_bar, button) = match self.connection_status {
            ConnectionStatus::Disconnected => {
                let address_bar = text_input("Server Address", &self.server_address)
                    .on_input(Message::AddressEdit)
                    .on_paste(Message::AddressEdit)
                    .on_submit(Message::AddressSubmit);

                let button = if self.server_address.trim().is_empty() {
                    button("Connect")
                } else {
                    button("Connect")
                        .on_press(Message::AddressSubmit)
                };

                (address_bar, button)
            },
            ConnectionStatus::Connecting => {
                let address_bar = text_input("Server Address", &self.server_address);
                let button = if self.connect_button_awaiting_state {
                    button("Cancel")
                } else {
                    button("Cancel")
                        .on_press(Message::ConnectionDisconnect)
                };

                (address_bar, button)
            },
            ConnectionStatus::Connected => {
                let address_bar = text_input("Server Address", &self.server_address);
                let button = if self.connect_button_awaiting_state {
                    button("Disconnect")
                } else {
                    button("Disconnect")
                        .on_press(Message::ConnectionDisconnect)
                }.style(button::danger);

                (address_bar, button)
            },
        };

        container(
            row![
                address_bar,
                button,
            ],
        ).padding(8)
    }

    fn view_autosplitter_panel(&self) -> Container<'_, Message> {
        container(
            column![
                self.view_autosplitter_file_select(),
                self.view_autosplitter_run(),
                self.view_autosplitter_settings(),
            ].spacing(8)
            
        ).padding(8).width(250)
    }

    fn view_autosplitter_file_select(&self) -> Container<'_, Message> {
        let (file_path_input, button) = match self.autosplitter_status {
            AutosplitterStatus::NotRunning => {
                let file_path_input = text_input("File Path", &self.autosplitter_path)
                    .on_input(Message::AutosplitterPathEdit)
                    .on_paste(Message::AutosplitterPathEdit);

                let button = button("Select...")
                    .on_press(Message::AutosplitterPathSelect);

                (file_path_input, button)
            },
            AutosplitterStatus::Running |
            AutosplitterStatus::Attached => {
                let file_path_input = text_input("File Path", &self.autosplitter_path);

                let button = button("Select...");

                (file_path_input, button)
            }
        };

        container(
            column![
                text("ASR Script File:"),
                row![
                    file_path_input,
                    button
                ]
            ].spacing(8)
        ).width(Length::Fill)
    }

    fn view_autosplitter_run(&self) -> Container<'_, Message> {
        let button = match self.autosplitter_status {
            AutosplitterStatus::NotRunning => {
                if self.autosplitter_path.trim().is_empty() {
                    button("Run Autosplitter")
                        .width(Length::Fill)
                } else {
                    button("Run Autosplitter")
                        .on_press(Message::AutosplitterRun)
                        .width(Length::Fill)
                }
            },
            AutosplitterStatus::Running |
            AutosplitterStatus::Attached => {
                if self.autosplitter_path.trim().is_empty() {
                    button("Stop Autosplitter")
                        .style(button::danger)
                        .width(Length::Fill)
                } else {
                    button("Stop Autosplitter")
                        .on_press(Message::AutosplitterStop)
                        .style(button::danger)
                        .width(Length::Fill)
                }
            },
        };

        container(
            button
        ).width(Length::Fill)
    }

    // A total mess. Trying to support dynamically defined settings is difficult!
    fn view_autosplitter_settings(&self) -> Container<'_, Message> {
        let mut settings = Vec::new();
        for setting in &self.autosplitter_settings {
            let value = self.autosplitter_current_settings.get(&setting.key);
            // I hate cloning the value here but we don't get too many options...
            let element = Self::view_autosplitter_setting(setting, value.cloned());

            settings.push(element);
        }

        container(
            scrollable(
                column(settings),
            ).width(Length::Fill).height(Length::Fill)
        )
    }

    fn view_autosplitter_setting(setting: &AutosplitterSetting, current_value: Option<AutosplitterSettingValue>) -> Element<'_, Message> {
        let element: Element<'_, Message> = match &setting.ty {
            crate::remote::SettingType::Heading(level) => {
                // this is probably wrong but sizes for levels
                let size = match level {
                    1 => 20.0,
                    2 => 18.0,
                    3 => 17.0,
                    4 => 16.0,
                    5 => 15.0,
                    6 => 14.0,
                    _ => 14.0,
                };
                
                container(
                    text(setting.description.clone())
                        .size(size)
                ).into()
            },
            crate::remote::SettingType::Checkbox(default) => {
                let value = if let Some(current_value) = current_value {
                    match current_value {
                        AutosplitterSettingValue::Checkbox(v) => v,
                        AutosplitterSettingValue::Combobox(_) |
                        AutosplitterSettingValue::FilePicker(_) => *default,
                    }
                } else {
                    *default
                };

                container(
                    checkbox(value)
                        .label(setting.description.clone())
                        .on_toggle(|v| Message::AutosplitterSettingCheckbox(setting.key.clone(), v))
                ).into()
            },
            crate::remote::SettingType::Combobox(default, autosplitter_combobox_choices) => {
                let current_key = if let Some(current_value) = current_value {
                    match current_value {
                        AutosplitterSettingValue::Combobox(v) => v,
                        AutosplitterSettingValue::Checkbox(_) |
                        AutosplitterSettingValue::FilePicker(_) => default.clone(),
                    }
                } else {
                    default.clone()
                };

                let value = match autosplitter_combobox_choices.iter().find(|v| v.key == current_key) {
                    Some(v) => Some(v.clone()),
                    None => None,
                };

                let combobox = pick_list(autosplitter_combobox_choices.clone(), value, |v| Message::AutosplitterSettingCombobox(setting.key.clone(), v.key));

                container(
                    combobox
                ).into()
            },
            crate::remote::SettingType::FilePicker => {
                let value = if let Some(current_value) = current_value {
                    match current_value {
                        AutosplitterSettingValue::Checkbox(_) |
                        AutosplitterSettingValue::Combobox(_) => "".to_string(),
                        AutosplitterSettingValue::FilePicker(v) => v,
                    }
                } else {
                    "".to_string()
                };

                let file_path_input = text_input("File Path", &value)
                    .on_input(|v| Message::AutosplitterSettingFilepathEdit(setting.key.clone(), v))
                    .on_paste(|v| Message::AutosplitterSettingFilepathEdit(setting.key.clone(), v));

                let button = button("Select...")
                    .on_press(Message::AutosplitterSettingFilepathSelect(setting.key.clone()));

                container(
            column![
                        text(setting.description.clone()),
                        row![
                            file_path_input,
                            button
                        ]
                    ].spacing(8)
                ).width(Length::Fill).into()
            },
        };

        match &setting.tooltip {
            Some(v) => {
                tooltip(
                    element, 
                    container(text(v.clone())).padding(10).style(container::rounded_box), 
                    tooltip::Position::Bottom).into()
            },
            None => element,
        }
    }

    fn view_logs(&self) -> Container<'_, Message> {
        let log_font = Font {
            family: iced::font::Family::Monospace,
            ..Default::default()
        };

        container(
            column![
                //container(text("Log")).padding(8),
                scrollable(
                    column(self.log_lines.iter().map(|s| text(s).font(log_font).into()))
                ).width(Length::Fill)
                .height(Length::Fill)
                .anchor_bottom(),
            ]
        ).padding(8)
    }

    fn view_status_bar(&self) -> Container<'_, Message> {
        let autosplitter_status_color = match self.autosplitter_status {
            AutosplitterStatus::NotRunning => color!(0xff0000),
            AutosplitterStatus::Running => color!(0x00ff00),
            AutosplitterStatus::Attached => color!(0x0000ff),
        };

        let autosplitter_status_text = match self.autosplitter_status {
            AutosplitterStatus::NotRunning => "Not running",
            AutosplitterStatus::Running => "Running",
            AutosplitterStatus::Attached => "Attached",
        };

        let connection_status_color = match self.connection_status {
            ConnectionStatus::Disconnected => color!(0xff0000),
            ConnectionStatus::Connecting => color!(0xffff00),
            ConnectionStatus::Connected => color!(0x00ff00),
        };

        let connection_status_text = match self.connection_status {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting",
            ConnectionStatus::Connected => "Connected",
        };

        container(
            row![
                tooltip(
                    row![
                        text("Connection:"),
                        text("●").color(connection_status_color),
                    ].spacing(8).padding([0, 8]),
                    container(text(connection_status_text)).padding(10).style(container::rounded_box),
                    tooltip::Position::Top,
                ),
                tooltip(
                    row![
                        text("Autosplitter:"),
                        text("●").color(autosplitter_status_color),
                    ].spacing(8).padding([0, 8]),
                    container(text(autosplitter_status_text)).padding(10).style(container::rounded_box),
                    tooltip::Position::Top,
                )
            ]
        ).align_right(Length::Fill).padding(4).style(container::dark)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AutosplitterPathSelect => {
                let filepath = rfd::FileDialog::new()
                    .set_title("Select ASR Script")
                    .add_filter("ASR Script", &["wasm"])
                    .add_filter("All Files", &["*"])
                    .set_directory("/")
                    .pick_file();

                if let Some(path) = filepath {
                    let str = path.to_str().unwrap().to_owned();

                    self.autosplitter_path = str;
                }

                Task::none()
            }
            Message::AutosplitterPathEdit(text) => {
                self.autosplitter_path = text;

                Task::none()
            }
            Message::AutosplitterRun => {
                self.sender.send(RoutedMessage::Autosplitter(crate::remote::AutosplitterMessage::ChangeFile(self.autosplitter_path.clone()))).unwrap();

                Task::none()
            }
            Message::AutosplitterStop => {
                self.sender.send(RoutedMessage::Autosplitter(crate::remote::AutosplitterMessage::ChangeFile("".to_string()))).unwrap();

                Task::none()
            }
            // TODO: next 4: sync settings with autosplitter
            Message::AutosplitterSettingCheckbox(key, value) => {
                self.autosplitter_current_settings.insert(key, AutosplitterSettingValue::Checkbox(value));

                Task::none()
            }
            Message::AutosplitterSettingCombobox(key, value_key) => {
                self.autosplitter_current_settings.insert(key, AutosplitterSettingValue::Combobox(value_key));

                Task::none()
            }
            Message::AutosplitterSettingFilepathEdit(key, text) => {
                self.autosplitter_current_settings.insert(key, AutosplitterSettingValue::FilePicker(text));

                Task::none()
            }
            Message::AutosplitterSettingFilepathSelect(key) => {
                let filepath = rfd::FileDialog::new()
                    .set_title("Select File")
                    .add_filter("All Files", &["*"])
                    .set_directory("/")
                    .pick_file();

                if let Some(path) = filepath {
                    let str = path.to_str().unwrap().to_owned();

                    self.autosplitter_current_settings.insert(key, AutosplitterSettingValue::FilePicker(str));
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
            Message::ConnectionDisconnect => {
                self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::ChangeAddress("".to_string()))).unwrap();

                self.connect_button_awaiting_state = true;

                Task::none()
            }
            Message::Log(msg) => {
                self.log_lines.push(msg);

                Task::none()
            }
            Message::AutosplitterStatus(status) => {
                self.autosplitter_status = status;

                Task::none()
            }
            Message::AutosplitterSettings(settings) => {
                self.autosplitter_settings = settings;

                Task::none()
            }
            Message::ConnectionStatus(status) => {
                if status != self.connection_status {
                    self.connection_status = status;

                    self.connect_button_awaiting_state = false;
                }

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
            UIMessage::AutosplitterStatus(status) => {
                Message::AutosplitterStatus(status)
            },
            UIMessage::ConnectionStatus(status) => {
                Message::ConnectionStatus(status)
            },
            UIMessage::AutosplitterSettings(settings) => {
                Message::AutosplitterSettings(settings)
            }
            UIMessage::Stop => unreachable!(""),
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

// Ring buffer implementation used for logs
struct RingBuffer<T> {
    buffer: Vec<T>,
    offset: usize,
    max_len: usize,
}

impl<T> RingBuffer<T> {
    fn new(max_len: usize) -> Self {
        Self {
            buffer: Vec::new(),
            offset: 0,
            max_len,
        }
    }

    fn push(&mut self, value: T) {
        if self.buffer.len() == self.max_len {
            self.buffer[self.offset] = value;
            self.offset = (self.offset + 1) % self.max_len;
        } else {
            self.buffer.push(value);
        }
    }

    fn iter(&self) -> RingBufferIterator<'_, T> {
        RingBufferIterator::new(self)
    }
}

impl<T> Index<usize> for RingBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let idx = (index + self.offset) % self.max_len;
        &self.buffer[idx]
    }
}

struct RingBufferIterator<'a, T> {
    parent: &'a RingBuffer<T>,
    idx: usize,
}

impl<'a, T> RingBufferIterator<'a, T> {
    fn new(parent: &'a RingBuffer<T>) -> Self {
        Self {
            parent,
            idx: 0,
        }
    }
}

impl<'a, T> Iterator for RingBufferIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.parent.buffer.len() && self.idx < self.parent.max_len {
            let result = Some(&self.parent[self.idx]);
            self.idx += 1;

            result
        } else {
            None
        }
    }
}