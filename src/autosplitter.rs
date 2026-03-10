use std::{
    fmt, fs,
    io::Read,
    sync::{Arc, Mutex, mpsc},
    time::Instant,
};

use livesplit_auto_splitting::{AutoSplitter, Runtime, Timer, TimerState};

use crate::message::{
    AutosplitterMessage, AutosplitterStatus, LiveSplitServerMessage, RoutedMessage, UIMessage,
};

enum ExitReason {
    SplitterPanic,
    ChangeFile(String),
    RequestedStop,
}

pub struct Autosplitter {
    filepath: String,
    receiver: mpsc::Receiver<AutosplitterMessage>,
    sender: mpsc::Sender<RoutedMessage>,
}

impl Autosplitter {
    pub fn new(
        filepath: String,
        receiver: mpsc::Receiver<AutosplitterMessage>,
        sender: mpsc::Sender<RoutedMessage>,
    ) -> Self {
        Self {
            filepath,
            receiver,
            sender,
        }
    }

    pub fn run(&mut self) {
        loop {
            self.sender
                .send(RoutedMessage::UI(UIMessage::AutosplitterStatus(
                    AutosplitterStatus::NotRunning,
                )))
                .unwrap();

            if !self.filepath.is_empty() {
                match self.inner_run() {
                    ExitReason::SplitterPanic => {}
                    ExitReason::ChangeFile(new_file) => self.filepath = new_file,
                    ExitReason::RequestedStop => break,
                }
            } else {
                match self.handle_offline_message() {
                    Ok(_) => {}
                    Err(e) => match e {
                        ExitReason::SplitterPanic => {}
                        ExitReason::ChangeFile(new_file) => self.filepath = new_file,
                        ExitReason::RequestedStop => break,
                    },
                }
            }
        }
    }

    fn inner_run(&self) -> ExitReason {
        self.log(format!("Opening autosplitter {}...", self.filepath));

        let mut filebuf = vec![];
        match fs::File::open(&self.filepath) {
            Ok(mut file) => {
                file.read_to_end(&mut filebuf).unwrap();
            }
            Err(err) => {
                println!("Failed to open {}: {}", self.filepath, err);
                return ExitReason::SplitterPanic;
            }
        }

        self.log(format!("Read {} bytes", filebuf.len()));

        let timer_state = Arc::new(Mutex::new((TimerState::NotRunning, 0)));
        let timer = RemoteTimer::new(self.sender.clone(), timer_state.clone());
        let settings_map = livesplit_auto_splitting::settings::Map::new(); // TODO: splitter config

        let config = livesplit_auto_splitting::Config::default();

        let runtime = match Runtime::new(config) {
            Ok(v) => v,
            Err(err) => {
                self.log(format!("Failed to start autosplitter runtime: {}", err));
                return ExitReason::SplitterPanic;
            }
        };

        let compiled_splitter = match runtime.compile(&filebuf) {
            Ok(v) => v,
            Err(err) => {
                self.log(format!("Failed to compile autosplitter: {}", err));
                return ExitReason::SplitterPanic;
            }
        };

        match compiled_splitter.instantiate(timer, Some(settings_map), None) {
            Ok(splitter) => {
                let result = self.run_splitter(&splitter, &timer_state);
                self.sender
                    .send(RoutedMessage::UI(UIMessage::AutosplitterStatus(
                        AutosplitterStatus::NotRunning,
                    )))
                    .unwrap();

                return result;
            }
            Err(err) => {
                self.log(format!("Failed to start autosplitter runtime: {}", err));
                return ExitReason::SplitterPanic;
            }
        }
    }

    fn run_splitter(
        &self,
        splitter: &AutoSplitter<RemoteTimer>,
        timer_state: &Arc<Mutex<(TimerState, u32)>>,
    ) -> ExitReason {
        let mut next_run_time = Instant::now();
        loop {
            let mut status = AutosplitterStatus::Running;

            {
                let mut splitter_lock = splitter.lock();
                match splitter_lock.update() {
                    Ok(()) => {}
                    Err(e) => {
                        self.log(format!("Failed to run autosplitter: {}", e));
                        return ExitReason::SplitterPanic;
                    }
                };

                for _ in splitter_lock.attached_processes() {
                    status = AutosplitterStatus::Attached;
                    break;
                }
            }

            next_run_time += splitter.tick_rate();

            // request a state update from the client
            // we'll get the response when polling the receiver
            self.sender
                .send(RoutedMessage::Client(LiveSplitServerMessage::TimerGetState))
                .unwrap();
            self.sender
                .send(RoutedMessage::UI(UIMessage::AutosplitterStatus(status)))
                .unwrap();

            match self.wait_poll_messages(next_run_time, &timer_state) {
                Ok(_) => {}
                Err(e) => return e,
            }
        }
    }

    fn handle_offline_message(&self) -> Result<(), ExitReason> {
        match self
            .receiver
            .recv_timeout(std::time::Duration::ZERO)
            .unwrap()
        {
            AutosplitterMessage::ChangeFile(new_file) => Err(ExitReason::ChangeFile(new_file)),
            AutosplitterMessage::Stop => Err(ExitReason::RequestedStop),
            _ => Ok(()),
        }
    }

    /// Performs a safe sleep but with `recv_timeout`, so we can read messages while
    /// waiting on the next tick.
    /// Returns a bool that is true if the Autosplitter should terminate.
    fn wait_poll_messages(
        &self,
        target: std::time::Instant,
        timer_state: &Arc<Mutex<(TimerState, u32)>>,
    ) -> Result<(), ExitReason> {
        let mut cur_time = std::time::Instant::now();
        while cur_time < target {
            cur_time = std::time::Instant::now();

            if target - cur_time > std::time::Duration::from_millis(3) {
                match self
                    .receiver
                    .recv_timeout(std::time::Duration::from_millis(1))
                {
                    Ok(message) => match self.service_message(message, timer_state) {
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    },
                    Err(e) => match e {
                        mpsc::RecvTimeoutError::Timeout => {}
                        mpsc::RecvTimeoutError::Disconnected => {
                            panic!("Receiver disconnected! Error: {}", e)
                        }
                    },
                }
            }
        }

        Ok(())
    }

    fn service_message(
        &self,
        message: AutosplitterMessage,
        timer_state: &Arc<Mutex<(TimerState, u32)>>,
    ) -> Result<(), ExitReason> {
        match message {
            AutosplitterMessage::TimerGetStateResponse(state, split_index) => {
                let mut lock = timer_state.lock().unwrap();

                lock.0 = state;
                lock.1 = split_index;

                Ok(())
            }
            AutosplitterMessage::ChangeFile(new_file) => Err(ExitReason::ChangeFile(new_file)),
            AutosplitterMessage::Stop => Err(ExitReason::RequestedStop),
        }
    }

    fn log(&self, msg: String) {
        self.sender
            .send(RoutedMessage::UI(UIMessage::Log(msg)))
            .unwrap();
    }
}

struct RemoteTimer {
    sender: mpsc::Sender<RoutedMessage>,
    timer_state: Arc<Mutex<(TimerState, u32)>>,
}

impl RemoteTimer {
    fn new(
        sender: mpsc::Sender<RoutedMessage>,
        timer_state: Arc<Mutex<(TimerState, u32)>>,
    ) -> Self {
        Self {
            sender,
            timer_state,
        }
    }

    fn set_state(&self, state: TimerState) {
        let mut lock = self.timer_state.lock().unwrap();
        lock.0 = state;
    }

    fn log_action(&self, msg: String) {
        self.sender
            .send(RoutedMessage::UI(UIMessage::Log(msg)))
            .unwrap();
    }
}

impl Timer for RemoteTimer {
    fn state(&self) -> TimerState {
        let state = self.timer_state.lock().unwrap();
        state.0
    }

    fn start(&mut self) {
        self.sender
            .send(RoutedMessage::Client(LiveSplitServerMessage::TimerStart))
            .unwrap();
        self.log_action(format!("starting timer!"));
        self.set_state(TimerState::Running);
    }

    fn split(&mut self) {
        self.sender
            .send(RoutedMessage::Client(LiveSplitServerMessage::TimerSplit))
            .unwrap();
        self.log_action(format!("splitting timer!"));
    }

    fn reset(&mut self) {
        self.sender
            .send(RoutedMessage::Client(LiveSplitServerMessage::TimerReset))
            .unwrap();
        self.log_action(format!("resetting timer!"));
        self.set_state(TimerState::NotRunning);
    }

    fn set_game_time(&mut self, time: livesplit_auto_splitting::time::Duration) {
        self.sender
            .send(RoutedMessage::Client(
                LiveSplitServerMessage::TimerSetGameTime(time),
            ))
            .unwrap();
    }

    fn pause_game_time(&mut self) {
        self.sender
            .send(RoutedMessage::Client(
                LiveSplitServerMessage::TimerPauseGameTime,
            ))
            .unwrap();
        self.log_action(format!("pausing game time!"));
    }

    fn resume_game_time(&mut self) {
        self.sender
            .send(RoutedMessage::Client(
                LiveSplitServerMessage::TimerResumeGameTime,
            ))
            .unwrap();
        self.log_action(format!("resuming game time!"));
    }

    fn set_variable(&mut self, _key: &str, _value: &str) {}

    fn current_split_index(&self) -> Option<usize> {
        let state = self.timer_state.lock().unwrap();
        match state.0 {
            TimerState::NotRunning => None,
            TimerState::Running | TimerState::Paused | TimerState::Ended => Some(state.1 as usize),
        }
    }

    fn segment_splitted(&self, _idx: usize) -> Option<bool> {
        todo!()
    }

    fn skip_split(&mut self) {
        self.sender
            .send(RoutedMessage::Client(
                LiveSplitServerMessage::TimerSkipSplit,
            ))
            .unwrap();
        self.log_action(format!("skipping split!"));
    }

    fn undo_split(&mut self) {
        self.sender
            .send(RoutedMessage::Client(
                LiveSplitServerMessage::TimerUndoSplit,
            ))
            .unwrap();
        self.log_action(format!("undoing split!"));
    }

    fn log_auto_splitter(&mut self, message: fmt::Arguments) {
        self.sender
            .send(RoutedMessage::UI(UIMessage::Log(format!(
                "autosplitter: {}",
                message
            ))))
            .unwrap();
    }

    fn log_runtime(
        &mut self,
        message: fmt::Arguments,
        _log_level: livesplit_auto_splitting::LogLevel,
    ) {
        self.sender
            .send(RoutedMessage::UI(UIMessage::Log(format!(
                "runtime: {}",
                message
            ))))
            .unwrap();
    }
}
