use std::sync::mpsc;

pub enum RoutedMessage {
    Client(LiveSplitServerMessage),
    Autosplitter(AutosplitterMessage),
    UI(UIMessage),
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
    Stop,
}

pub enum AutosplitterStatus {
    NotRunning,
    Running,
    Attached,
}

pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

pub struct MessageRouter {
    client_sender: mpsc::Sender<LiveSplitServerMessage>,
    autosplitter_sender: mpsc::Sender<AutosplitterMessage>,
    ui_sender: mpsc::Sender<UIMessage>,
    receiver: mpsc::Receiver<RoutedMessage>,
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
        }
    }

    pub fn run(&self) {
        loop {
            let dst = self.receiver.recv().unwrap();
            let should_quit = self.handle_message(dst);

            if should_quit {
                return;
            }
        }
    }

    fn handle_message(&self, dst: RoutedMessage) -> bool {
        match dst {
            RoutedMessage::Client(msg) => self.client_sender.send(msg).unwrap(),
            RoutedMessage::Autosplitter(msg) => self.autosplitter_sender.send(msg).unwrap(),
            RoutedMessage::UI(msg) => self.ui_sender.send(msg).unwrap(),
            RoutedMessage::Quit => {
                self.client_sender
                    .send(LiveSplitServerMessage::Stop)
                    .unwrap();
                self.autosplitter_sender
                    .send(AutosplitterMessage::Stop)
                    .unwrap();
                return true;
            }
        }

        false
    }
}
