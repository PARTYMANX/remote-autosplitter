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
    Stop,
}

pub enum AutosplitterMessage {
    TimerGetStateResponse(livesplit_auto_splitting::TimerState, u32),
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