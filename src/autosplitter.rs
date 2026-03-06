use std::{fmt, fs, io::Read, sync::mpsc, thread};

use livesplit_auto_splitting::{Runtime, Timer, SettingsStore, TimerState, time};

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

        let timer = RemoteTimer::new(&self.reciever, &self.sender);
        let settings = SettingsStore::new();

        match Runtime::new(&filebuf, timer, settings) {
            Ok(mut runtime) => {
                loop {
                    let time_to_wait = runtime.update().unwrap();
                    // would probably be a good idea to update state here
                    // also change wait behavior to wait for time of last run + time_to_wait
                    thread::sleep(time_to_wait);
                }
            },
            Err(err) => {
                println!("Failed to start autosplitter runtime: {}", err);
                return;
            }
        }
    }
}

struct RemoteTimer<'a> {
    state: TimerState,
    reciever: &'a mpsc::Receiver<AutosplitterMessage>,
    sender: &'a mpsc::Sender<RoutedMessage>,
}

impl<'a> RemoteTimer<'a> {
    fn new(reciever: &'a mpsc::Receiver<AutosplitterMessage>, sender: &'a mpsc::Sender<RoutedMessage>) -> Self {
        Self {
            state: TimerState::NotRunning,
            reciever,
            sender,
        }
    }
}

impl Timer for RemoteTimer<'_> {
    fn state(&self) -> TimerState {
        self.state
    }

    fn start(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerStart)).unwrap();
        println!("starting timer!");
        self.state = TimerState::Running;
    }

    fn split(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerSplit)).unwrap();
        println!("splitting timer!");
    }

    fn reset(&mut self) {
        self.sender.send(RoutedMessage::Client(LiveSplitServerMessage::TimerReset)).unwrap();
        println!("resetting timer!");
        self.state = TimerState::NotRunning;
        
    }

    fn set_game_time(&mut self, time: time::Duration) {
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

    fn log(&mut self, message: fmt::Arguments<'_>) {
        println!("autosplitter log: {}", message);
    }
}