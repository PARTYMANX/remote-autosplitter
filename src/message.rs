pub enum RoutedMessage {
    Client(LiveSplitServerMessage),
    Autosplitter(AutosplitterMessage),
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