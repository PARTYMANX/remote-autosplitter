use std::{fmt::Display, sync::mpsc};

pub enum RoutedMessage {
    Client(LiveSplitServerMessage),
    Autosplitter(AutosplitterMessage),
    UI(UIMessage),
    SetWaker(std::task::Waker),
    Quit,
}

pub enum LiveSplitServerMessage {
    TimerStart,
    TimerSplit,
    TimerReset,
    TimerSkipSplit,
    TimerUndoSplit,
    TimerSetGameTime(livesplit_auto_splitting::time::Duration),
    TimerPauseGameTime,
    TimerResumeGameTime,
    TimerGetState,
    ChangeAddress(String),
    Stop,
}

pub enum AutosplitterMessage {
    TimerGetStateResponse(livesplit_auto_splitting::TimerState, u32),
    ChangeFile(String),
    Stop,
}

pub enum UIMessage {
    Log(String),
    AutosplitterStatus(AutosplitterStatus),
    ConnectionStatus(ConnectionStatus),
    AutosplitterSettings(Vec<AutosplitterSetting>),
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutosplitterStatus {
    NotRunning,
    Running,
    Attached,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone)]
pub enum SettingType {
    Heading(u32),
    Checkbox(bool),
    Combobox(String, Vec<AutosplitterComboboxChoice>),
    FilePicker, // ignoring filter for now
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutosplitterComboboxChoice {
    pub key: String,
    pub description: String,
}

impl Display for AutosplitterComboboxChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
}

#[derive(Debug, Clone)]
pub struct AutosplitterSetting {
    pub key: String,
    pub description: String,
    pub tooltip: Option<String>,
    pub ty: SettingType,
}

pub struct MessageRouter {
    client_sender: mpsc::Sender<LiveSplitServerMessage>,
    autosplitter_sender: mpsc::Sender<AutosplitterMessage>,
    ui_sender: mpsc::Sender<UIMessage>,
    receiver: mpsc::Receiver<RoutedMessage>,
    waker: Option<std::task::Waker>,    // waker for UI message polling
}

impl MessageRouter {
    pub fn new(
        client_sender: mpsc::Sender<LiveSplitServerMessage>,
        autosplitter_sender: mpsc::Sender<AutosplitterMessage>,
        ui_sender: mpsc::Sender<UIMessage>,
        receiver: mpsc::Receiver<RoutedMessage>,
    ) -> Self {
        Self {
            client_sender,
            autosplitter_sender,
            ui_sender,
            receiver,
            waker: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            let dst = self.receiver.recv().unwrap();
            let should_quit = self.handle_message(dst);

            if should_quit {
                return;
            }
        }
    }

    fn handle_message(&mut self, dst: RoutedMessage) -> bool {
        
        match dst {
            RoutedMessage::Client(msg) => self.client_sender.send(msg).unwrap(),
            RoutedMessage::Autosplitter(msg) => self.autosplitter_sender.send(msg).unwrap(),
            RoutedMessage::UI(msg) => {
                if let Some(waker) = &self.waker {
                    waker.wake_by_ref();
                    self.waker = None;
                }
                self.ui_sender.send(msg).unwrap()
            },
            RoutedMessage::SetWaker(waker) => self.waker = Some(waker),
            RoutedMessage::Quit => {
                if let Some(waker) = &self.waker {
                    waker.wake_by_ref();
                    self.waker = None;
                }
                self.client_sender
                    .send(LiveSplitServerMessage::Stop)
                    .unwrap();
                self.autosplitter_sender
                    .send(AutosplitterMessage::Stop)
                    .unwrap();
                self.ui_sender
                    .send(UIMessage::Stop)
                    .unwrap();
                return true;
            }
        }

        false
    }
}
