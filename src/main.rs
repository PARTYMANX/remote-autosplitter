use std::{fmt, thread, env, fs, io::{Read, Write}, net};

use livesplit_auto_splitting::{Runtime, Timer, SettingsStore, TimerState, time};

pub struct RemoteTimer {
    state: TimerState,
    socket: net::TcpStream,
}

impl RemoteTimer {
    fn new(socket: net::TcpStream) -> Self {
        Self {
            state: TimerState::NotRunning,
            socket: socket,
        }
    }
}

impl Timer for RemoteTimer {
    fn state(&self) -> TimerState {
        self.state
    }

    fn start(&mut self) {
        // send message here
        self.socket.write("starttimer\r\n".as_bytes()).unwrap();
        println!("starting timer!");
        self.state = TimerState::Running;
    }

    fn split(&mut self) {
        // send message here
        self.socket.write("split\r\n".as_bytes()).unwrap();
        println!("splitting timer!");
    }

    fn reset(&mut self) {
        // send message here
        self.socket.write("reset\r\n".as_bytes()).unwrap();
        println!("resetting timer!");
        self.state = TimerState::NotRunning;
    }

    fn set_game_time(&mut self, time: time::Duration) {
        // send message here
        self.socket.write(format!("setgametime {}:{}:{}.{}\r\n", time.whole_hours(), time.whole_minutes(), time.whole_seconds(), time.whole_milliseconds()).as_bytes()).unwrap();
        println!("setting game time!");
    }

    fn pause_game_time(&mut self) {
        // send message here
        self.socket.write("pausegametime\r\n".as_bytes()).unwrap();
        println!("pausing game time!");
    }

    fn resume_game_time(&mut self) {
        // send message here
        self.socket.write("unpausegametime\r\n".as_bytes()).unwrap();
        println!("resuming game time!");
    }

    fn set_variable(&mut self, _key: &str, _value: &str) {

    }

    fn log(&mut self, message: fmt::Arguments<'_>) {
        println!("autosplitter log: {}", message);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: remote-autosplitter <autosplitter wasm path> <address:port>");
        return;
    }

    let address = args.get(2).unwrap();

    println!("Connecting to {}...", address);

    match net::TcpStream::connect(address) {
        Ok(socket) => {
            println!("Connected to server!");

            let filepath = args.get(1).unwrap();
            let mut filebuf = vec!();
            match fs::File::open(filepath) {
                Ok(mut file) => {
                    file.read_to_end(&mut filebuf).unwrap();
                },
                Err(err) => {
                    println!("Failed to open {}: {}", filepath, err);
                    return;
                }
            }

            let timer = RemoteTimer::new(socket);
            let settings = SettingsStore::new();

            match Runtime::new(&filebuf, timer, settings) {
                Ok(mut runtime) => {
                    loop {
                        let time_to_wait = runtime.update().unwrap();
                        thread::sleep(time_to_wait);
                    }
                },
                Err(err) => {
                    println!("Failed to start autosplitter runtime: {}", err);
                    return;
                }
            }
        },
        Err(err) => {
            println!("Failed to connect to {}: {}", address, err);
        }
    }
}
