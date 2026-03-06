use std::{io::Write, net, sync::mpsc};

use crate::message::{LiveSplitServerMessage, RoutedMessage};

#[derive(Debug)]
enum ClientError {
    LostConnection,
}

pub struct LiveSplitClient {
    address: String,
    reciever: mpsc::Receiver<LiveSplitServerMessage>,
    sender: mpsc::Sender<RoutedMessage>,
}

impl LiveSplitClient {
    pub fn new(address: String, reciever: mpsc::Receiver<LiveSplitServerMessage>, sender: mpsc::Sender<RoutedMessage>) -> Self {
        Self {
            address,
            reciever,
            sender,
        }
    }

    pub fn run(&self) {
        loop {
            println!("Connecting to {}...", self.address);

            match net::TcpStream::connect(&self.address) {
                Ok(mut socket) => {
                    println!("Connected to server!");
                    let result = self.handle_messages(&mut socket);

                    match result {
                        Ok(()) => break,
                        Err(e) => {
                            println!("Client error: {:?}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to connect to {}: {}", self.address, e);
                }
            }
        }
    }

    fn handle_messages(&self, socket: &mut net::TcpStream) -> Result<(), ClientError> {
        loop {
            match self.reciever.recv().unwrap() {
                LiveSplitServerMessage::TimerStart => match socket.write("starttimer\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Socket error: {}", e);
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerSplit => match socket.write("split\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Socket error: {}", e);
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerReset => match socket.write("reset\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Socket error: {}", e);
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerSetGameTime(time) => {
                    let result = socket.write(format!("setgametime {}:{}:{}.{}\r\n", time.whole_hours(), time.whole_minutes() % 60, time.whole_seconds() % 60, time.whole_milliseconds() % 1000).as_bytes());

                    match result {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Socket error: {}", e);
                            return Err(ClientError::LostConnection)
                        }
                    }
                },
                LiveSplitServerMessage::TimerPauseGameTime => match socket.write("pausegametime\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Socket error: {}", e);
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::TimerResumeGameTime => match socket.write("unpausegametime\r\n".as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Socket error: {}", e);
                        return Err(ClientError::LostConnection)
                    }
                },
                LiveSplitServerMessage::Stop => break,
            };
        }

        Ok(())
    }
}