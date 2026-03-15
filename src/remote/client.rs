use std::{
    io::{Read, Write},
    net,
    sync::mpsc,
};

use livesplit_auto_splitting::TimerState;

use super::message::{
    AutosplitterMessage, ConnectionStatus, LiveSplitServerMessage, RoutedMessage, UIMessage,
};

enum ExitReason {
    LostConnection,
    ChangeAddress(String),
    RequestedStop,
}

pub struct LiveSplitClient {
    address: String,
    receiver: mpsc::Receiver<LiveSplitServerMessage>,
    sender: mpsc::Sender<RoutedMessage>,
}

impl LiveSplitClient {
    pub fn new(
        address: String,
        receiver: mpsc::Receiver<LiveSplitServerMessage>,
        sender: mpsc::Sender<RoutedMessage>,
    ) -> Self {
        Self {
            address,
            receiver,
            sender,
        }
    }

    pub fn run(&mut self) {
        loop {
            self.sender
                .send(RoutedMessage::UI(UIMessage::ConnectionStatus(
                    ConnectionStatus::Disconnected,
                )))
                .unwrap();

            if !self.address.is_empty() {
                match self.inner_run() {
                    ExitReason::LostConnection => {}
                    ExitReason::ChangeAddress(new_address) => self.address = new_address,
                    ExitReason::RequestedStop => break,
                }
            } else {
                println!("Running connection offline message");
                match self.handle_offline_messages() {
                    Ok(_) => {}
                    Err(e) => match e {
                        ExitReason::LostConnection => {}
                        ExitReason::ChangeAddress(new_address) => self.address = new_address,
                        ExitReason::RequestedStop => break,
                    },
                }
            }
        }
    }

    fn inner_run(&self) -> ExitReason {
        self.sender
            .send(RoutedMessage::UI(UIMessage::ConnectionStatus(
                ConnectionStatus::Connecting,
            )))
            .unwrap();
        self.log(format!("Connecting to {}...", self.address));

        let failure_continue_time = std::time::Instant::now() + std::time::Duration::from_secs_f64(1.0 / 60.0);

        let result = match net::TcpStream::connect(&self.address) {
            Ok(mut socket) => self.handle_messages(&mut socket),
            Err(e) => {
                self.log(format!("Failed to connect to {}: {}", self.address, e));

                // handle any messages that work offline (stop or change address)
                // also wait a bit if the connection failed instantly to prevent
                // flooding messages
                // is there a way to interrupt a connection attempt for one of these?
                match self.wait_poll_offline_messages(failure_continue_time) {
                    Ok(_) => ExitReason::LostConnection,
                    Err(e) => e,
                }
            }
        };

        result
    }

    fn handle_messages(&self, socket: &mut net::TcpStream) -> ExitReason {
        self.log(format!("Connected to server!"));
        self.sender
            .send(RoutedMessage::UI(UIMessage::ConnectionStatus(
                ConnectionStatus::Connected,
            )))
            .unwrap();
        loop {
            let result = self.handle_message(socket);

            match result {
                Ok(()) => {}
                Err(e) => return e,
            }
        }
    }

    fn handle_offline_messages(&self) -> Result<(), ExitReason> {
        // messages until stop or address changes since we're not attempting a connection
        loop {
            match self.receiver.recv().unwrap() {
                LiveSplitServerMessage::ChangeAddress(new_address) => {
                    return Err(ExitReason::ChangeAddress(new_address))
                }
                LiveSplitServerMessage::Stop => return Err(ExitReason::RequestedStop),
                // ignore all other messages since we're not connected
                _ => {},
            }
        }
    }

    fn wait_poll_offline_messages(
        &self,
        target: std::time::Instant,
    ) -> Result<(), ExitReason> {
        let mut cur_time = std::time::Instant::now();
        while cur_time < target {
            cur_time = std::time::Instant::now();

            if target - cur_time > std::time::Duration::from_millis(3) {
                match self
                    .receiver
                    .recv_timeout(std::time::Duration::from_millis(1))
                {
                    Ok(message) => match Self::handle_offline_message(message) {
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

    fn handle_offline_message(msg: LiveSplitServerMessage) -> Result<(), ExitReason> {
        match msg {
            LiveSplitServerMessage::ChangeAddress(new_address) => Err(ExitReason::ChangeAddress(new_address)),
            LiveSplitServerMessage::Stop => Err(ExitReason::RequestedStop),
            // ignore all other messages since we're not connected
            _ => Ok(()),
        }
    }

    fn handle_message(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match self.receiver.recv().unwrap() {
            LiveSplitServerMessage::TimerStart => self.msg_timer_start(socket),
            LiveSplitServerMessage::TimerSplit => self.msg_timer_split(socket),
            LiveSplitServerMessage::TimerReset => self.msg_timer_reset(socket),
            LiveSplitServerMessage::TimerUndoSplit => self.msg_timer_undo_split(socket),
            LiveSplitServerMessage::TimerSkipSplit => self.msg_timer_skip_split(socket),
            LiveSplitServerMessage::TimerSetGameTime(time) => self.msg_timer_set_time(socket, time),
            LiveSplitServerMessage::TimerPauseGameTime => self.msg_timer_pause_game_time(socket),
            LiveSplitServerMessage::TimerResumeGameTime => self.msg_timer_resume_game_time(socket),
            LiveSplitServerMessage::TimerGetState => self.msg_timer_get_timer_state(socket),
            LiveSplitServerMessage::ChangeAddress(new_address) => {
                Err(ExitReason::ChangeAddress(new_address))
            }
            LiveSplitServerMessage::Stop => Err(ExitReason::RequestedStop),
        }
    }

    fn msg_timer_start(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("starttimer\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_split(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("split\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_reset(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("reset\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_undo_split(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("unsplit\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_skip_split(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("skipsplit\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_set_time(
        &self,
        socket: &mut net::TcpStream,
        time: livesplit_auto_splitting::time::Duration,
    ) -> Result<(), ExitReason> {
        let result = socket.write(
            format!(
                "setgametime {}:{}:{}.{}\r\n",
                time.whole_hours(),
                time.whole_minutes() % 60,
                time.whole_seconds() % 60,
                time.whole_milliseconds() % 1000
            )
            .as_bytes(),
        );

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_pause_game_time(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("pausegametime\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_resume_game_time(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        match socket.write("resumegametime\r\n".as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        }
    }

    fn msg_timer_get_timer_state(&self, socket: &mut net::TcpStream) -> Result<(), ExitReason> {
        let mut read_buffer = [0u8; 128];

        match socket.write("getcurrenttimerphase\r\n".as_bytes()) {
            Ok(_) => {}
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        };

        let state = match socket.read(&mut read_buffer) {
            Ok(read_bytes) => match str::from_utf8(&read_buffer[0..read_bytes]) {
                Ok(s) => match s.trim() {
                    "NotRunning" => TimerState::NotRunning,
                    "Running" => TimerState::Running,
                    "Ended" => TimerState::Ended,
                    "Paused" => TimerState::Paused,
                    _ => TimerState::NotRunning,
                },
                Err(e) => {
                    self.log(format!("Failed to parse timer state response: {}", e));
                    TimerState::NotRunning
                }
            },
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        };

        match socket.write("getsplitindex\r\n".as_bytes()) {
            Ok(_) => {}
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        };

        let split_index = match socket.read(&mut read_buffer) {
            Ok(read_bytes) => match str::from_utf8(&read_buffer[0..read_bytes]) {
                Ok(s) => match s.trim().parse::<i32>() {
                    Ok(v) => {
                        if v < 0 {
                            0u32
                        } else {
                            v as u32
                        }
                    }
                    Err(e) => {
                        self.log(format!("Failed to parse timer split index response: {}", e));
                        0
                    }
                },
                Err(e) => {
                    self.log(format!("Failed to parse timer split index response: {}", e));
                    0
                }
            },
            Err(e) => {
                self.log(format!("Socket error: {}", e));
                return Err(ExitReason::LostConnection);
            }
        };

        self.sender
            .send(RoutedMessage::Autosplitter(
                AutosplitterMessage::TimerGetStateResponse(state, split_index),
            ))
            .unwrap();

        Ok(())
    }

    fn log(&self, msg: String) {
        self.sender
            .send(RoutedMessage::UI(UIMessage::Log(msg)))
            .unwrap();
    }
}
