use std::{io::{Read, Write}, net, sync::mpsc};

use livesplit_auto_splitting::TimerState;

use crate::message::{AutosplitterMessage, ConnectionStatus, LiveSplitServerMessage, RoutedMessage, UIMessage};

#[derive(Debug)]
enum ClientError {
    LostConnection,
}

pub struct LiveSplitClient {
    address: String,
    receiver: mpsc::Receiver<LiveSplitServerMessage>,
    sender: mpsc::Sender<RoutedMessage>,
}

impl LiveSplitClient {
    pub fn new(address: String, receiver: mpsc::Receiver<LiveSplitServerMessage>, sender: mpsc::Sender<RoutedMessage>) -> Self {
        Self {
            address,
            receiver,
            sender,
        }
    }

    pub fn run(&self) {
        loop {
            self.sender.send(RoutedMessage::UI(UIMessage::ConnectionStatus(ConnectionStatus::Connecting))).unwrap();
            self.log(format!("Connecting to {}...", self.address));

            match net::TcpStream::connect(&self.address) {
                Ok(mut socket) => {
                    self.log(format!("Connected to server!"));
                    self.sender.send(RoutedMessage::UI(UIMessage::ConnectionStatus(ConnectionStatus::Connected))).unwrap();
                    let result = self.handle_messages(&mut socket);

                    match result {
                        Ok(()) => break,
                        Err(e) => {
                            self.log(format!("Client error: {:?}", e));
                        }
                    }
                },
                Err(e) => {
                    self.log(format!("Failed to connect to {}: {}", self.address, e));
                }
            }
            self.sender.send(RoutedMessage::UI(UIMessage::ConnectionStatus(ConnectionStatus::Disconnected))).unwrap();
        }
    }

    fn handle_messages(&self, socket: &mut net::TcpStream) -> Result<(), ClientError> {
        loop {
            match self.receiver.recv().unwrap() {
                LiveSplitServerMessage::TimerStart => match socket.write("starttimer\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerSplit => match socket.write("split\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerReset => match socket.write("reset\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerUndoSplit => match socket.write("unsplit\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerSkipSplit => match socket.write("skipsplit\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerSetGameTime(time) => {
                    let result = socket.write(format!("setgametime {}:{}:{}.{}\r\n", time.whole_hours(), time.whole_minutes() % 60, time.whole_seconds() % 60, time.whole_milliseconds() % 1000).as_bytes());

                    match result {
                        Ok(_) => {},
                        Err(e) => {
                            self.log(format!("Socket error: {}", e));
                            return Err(ClientError::LostConnection)
                        }
                    }
                },
                LiveSplitServerMessage::TimerPauseGameTime => match socket.write("pausegametime\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerResumeGameTime => match socket.write("unpausegametime\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        self.log(format!("Socket error: {}", e));
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerGetState => {
                    let mut read_buffer = [0u8; 128];

                    match socket.write("getcurrenttimerphase\r\n".as_bytes()) {
                        Ok(_) => {},
                        Err(e) => {
                            self.log(format!("Socket error: {}", e));
                            return Err(ClientError::LostConnection)
                        }
                    };

                    let state = match socket.read(&mut read_buffer) {
                        Ok(read_bytes) => {
                            match str::from_utf8(&read_buffer[0..read_bytes]) {
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
                                },
                            }
                        },
                        Err(e) => {
                            self.log(format!("Socket error: {}", e));
                            return Err(ClientError::LostConnection)
                        },
                    };

                    match socket.write("getsplitindex\r\n".as_bytes()) {
                        Ok(_) => {},
                        Err(e) => {
                            self.log(format!("Socket error: {}", e));
                            return Err(ClientError::LostConnection)
                        }
                    };

                    let split_index = match socket.read(&mut read_buffer) {
                        Ok(read_bytes) => {
                            match str::from_utf8(&read_buffer[0..read_bytes]) {
                                Ok(s) => match s.trim().parse::<i32>() {
                                    Ok(v) => if v < 0 {
                                        0u32
                                    } else {
                                        v as u32
                                    },
                                    Err(e) => {
                                        self.log(format!("Failed to parse timer split index response: {}", e));
                                        0
                                    },
                                },
                                Err(e) => {
                                    self.log(format!("Failed to parse timer split index response: {}", e));
                                    0
                                },
                            }
                        },
                        Err(e) => {
                            self.log(format!("Socket error: {}", e));
                            return Err(ClientError::LostConnection)
                        },
                    };

                    self.sender.send(RoutedMessage::Autosplitter(AutosplitterMessage::TimerGetStateResponse(state, split_index))).unwrap();
                }
                LiveSplitServerMessage::Stop => break,
            };
        }

        Ok(())
    }

    fn log(&self, msg: String) {
        self.sender.send(RoutedMessage::UI(UIMessage::Log(msg))).unwrap();
    }
}