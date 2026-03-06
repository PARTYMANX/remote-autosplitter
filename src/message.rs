pub enum RoutedMessage {
    Client(LiveSplitServerMessage),
    Autosplitter(AutosplitterMessage),
}

pub enum LiveSplitServerMessage {
    TimerStart,
    TimerSplit,
    TimerReset,
    TimerSetGameTime(livesplit_auto_splitting::time::Duration),
    TimerPauseGameTime,
    TimerResumeGameTime,
    Stop,
}

pub enum AutosplitterMessage {
    Stop,
}