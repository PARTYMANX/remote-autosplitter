use std::{fmt, fs, io::Read, sync::{Arc, Mutex, mpsc}, thread, time::Instant};

use livesplit_auto_splitting::{Runtime, Timer, TimerState};

use crate::message::{AutosplitterMessage, LiveSplitServerMessage, RoutedMessage};

pub struct Autosplitter {
    filepath: String,
    reciever: mpsc::Receiver<AutosplitterMessage>,
    sender: mpsc::Sender<RoutedMessage>,
}

impl Autosplitter {
    pub fn new(filepath: String, reciever: mpsc::Receiver<AutosplitterMessage>, sender: mpsc::Sender<RoutedMessage>) -> Self {
        Self { 
            filepath, 
            reciever,
            sender,
        }
    }

    pub fn run(&self) {
        println!("Opening autosplitter {}...", self.filepath);

        let mut filebuf = vec!();
        match fs::File::open(&self.filepath) {
            Ok(mut file) => {
                file.read_to_end(&mut filebuf).unwrap();
            },
            Err(err) => {
                println!("Failed to open {}: {}", self.filepath, err);
                return;
            }
        }

        println!("Read {} bytes", filebuf.len());

        let timer_state = Arc::new(Mutex::new((TimerState::NotRunning, 0)));
        let timer = RemoteTimer::new(self.sender.clone(), timer_state.clone());
        let settings_map = livesplit_auto_splitting::settings::Map::new();

        let config = livesplit_auto_splitting::Config::default();

        let runtime = match Runtime::new(config) {
            Ok(v) => v,
            Err(err) => {
                println!("Failed to start autosplitter runtime: {}", err);
                return;
            }
        };

        let compiled_splitter = match runtime.compile(&filebuf) {
            Ok(v) => v,
            Err(err) => {
                println!("Failed to compile autosplitter: {}", err);
                return;
            }
        };

        let mut next_run_time = Instant::now();
        match compiled_splitter.instantiate(timer, Some(settings_map), None) {
            Ok(splitter) => {
                loop {
                    safe_wait(next_run_time);
                    match splitter.lock().update() {
                        Ok(()) => {}
                        Err(e) => {
                            println!("Failed to run autosplitter: {}", e);
                            break;
                        }
                    };
                    next_run_time += splitter.tick_rate();

                    self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerGetState)).unwrap();

                    let (state, split_index) = match self.reciever.recv().unwrap() {
                        AutosplitterMessage::TimerGetStateResponse(state, split_index) => (state, split_index),
                        AutosplitterMessage::Stop => todo!(),
                    };

                    let mut lock = timer_state.lock().unwrap();
                    lock.0 = state;
                    lock.1 = split_index;

                    // would probably be a good idea to update state here
                    //self.sender.send(RoutedMessage::Client(Li))
                }
            },
            Err(err) => {
                println!("Failed to start autosplitter runtime: {}", err);
                return;
            }
        }
    }
}

struct RemoteTimer {
    sender: mpsc::Sender<RoutedMessage>,
    timer_state: Arc<Mutex<(TimerState, u32)>>,
}

impl RemoteTimer {
    fn new(sender: mpsc::Sender<RoutedMessage>, timer_state: Arc<Mutex<(TimerState, u32)>>) -> Self {
        Self {
            sender,
            timer_state,
        }
    }

    fn set_state(&self, state: TimerState) {
        let mut lock = self.timer_state.lock().unwrap();
        lock.0 = state;
    }
}

impl Timer for RemoteTimer {
    fn state(&self) -> TimerState {
        let state = self.timer_state.lock().unwrap();
        state.0
    }

    fn start(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerStart)).unwrap();
        println!("starting timer!");
        self.set_state(TimerState::Running);
    }

    fn split(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerSplit)).unwrap();
        println!("splitting timer!");
    }

    fn reset(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerReset)).unwrap();
        println!("resetting timer!");
        self.set_state(TimerState::NotRunning);
        
    }

    fn set_game_time(&mut self, time: livesplit_auto_splitting::time::Duration) {
        // send message here
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerSetGameTime(time))).unwrap();
        //println!("setting game time!");
    }

    fn pause_game_time(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerPauseGameTime)).unwrap();
        println!("pausing game time!");
    }

    fn resume_game_time(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerResumeGameTime)).unwrap();
        println!("resuming game time!");
    }

    fn set_variable(&mut self, _key: &str, _value: &str) {

    }
    
    fn current_split_index(&self) -> Option<usize> {
        let state = self.timer_state.lock().unwrap();
        match state.0 {
            TimerState::NotRunning => None,
            TimerState::Running |
            TimerState::Paused |
            TimerState::Ended => Some(state.1 as usize)
        }
    }
    
    fn segment_splitted(&self, idx: usize) -> Option<bool> {
        todo!()
    }
    
    fn skip_split(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerSkipSplit)).unwrap();
        println!("skipping split!");
    }
    
    fn undo_split(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerUndoSplit)).unwrap();
        println!("undoing split!");
    }
    
    fn log_auto_splitter(&mut self, message: fmt::Arguments) {
        println!("autosplitter log: {}", message);
    }
    
    fn log_runtime(&mut self, message: fmt::Arguments, _log_level: livesplit_auto_splitting::LogLevel) {
        println!("runtime log: {}", message);
    }
}

fn safe_wait(target: std::time::Instant) {
    let mut cur_time = std::time::Instant::now();
    while cur_time < target {
        cur_time = std::time::Instant::now();

        if target - cur_time > std::time::Duration::from_millis(3) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
