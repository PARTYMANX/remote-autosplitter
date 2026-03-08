use std::{sync::mpsc, time};

use ratatui::{layout::Rect, style::{Color, Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Block, List, ListDirection, Paragraph}};

use crate::message::{AutosplitterStatus, ConnectionStatus, RoutedMessage, UIMessage};

pub struct UI {
    tick_duration: time::Duration,
    receiver: mpsc::Receiver<UIMessage>,
    sender: mpsc::Sender<RoutedMessage>,
    splitter_status: AutosplitterStatus,
    connection_status: ConnectionStatus,
    log_lines: Vec<String>,
    log_front: u32,
}

impl UI {
    pub fn new(receiver: mpsc::Receiver<UIMessage>, sender: mpsc::Sender<RoutedMessage>) -> Self {
        Self { 
            tick_duration: time::Duration::from_secs_f64(1.0 / 60.0),
            receiver, 
            sender,
            splitter_status: AutosplitterStatus::NotRunning,
            connection_status: ConnectionStatus::Disconnected,
            log_lines: Vec::new(),
            log_front: 0,
        }
    }

    pub fn run(&mut self) {
        let light_red = Style::new()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD);
        let light_red_blink = Style::new()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::RAPID_BLINK);
        let light_green = Style::new()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD);
        let light_blue = Style::new()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD);

        ratatui::run(|terminal| {
            let mut next_tick = time::Instant::now();
            loop {
                //let render_start_time = time::Instant::now();
                terminal.draw(|frame| {
                    let block = Block::bordered().title("Log");
                    let mut log_lines = Vec::new();

                    let log_height = (frame.area().height - 3) as usize;

                    let (lower, upper) = if self.log_lines.len() < log_height {
                        (0, self.log_lines.len())
                    } else {
                        let lower = (self.log_front as usize + 1 + 1000 - log_height) % 1000;
                        let upper = lower + log_height;
                        (lower, upper)
                    };

                    for i in lower..upper {
                        let idx = i % 1000;
                        log_lines.push(self.log_lines[idx].clone());
                        //log_lines.push(format!("Lower: {}, Upper: {}, Log Front: {}, Log Height: {}", lower, upper, self.log_front, log_height));
                    }
                    //log_lines.push(format!("Lower: {}, Upper: {}", lower, upper));

                    let list = List::new(log_lines)
                        .block(block)
                        .direction(ListDirection::TopToBottom);
                    frame.render_widget(list, Rect::new(frame.area().x, frame.area().y, frame.area().width, frame.area().height - 1) );

                    /*if self.log_lines.len() > 0 {
                        let text = Paragraph::new(self.log_lines[self.log_front as usize].clone())
                            .block(block);
                        frame.render_widget(text, Rect::new(frame.area().x, frame.area().y, frame.area().width, frame.area().height - 1) );
                    }*/
                    
                    let status = Line::from(vec![
                        Span::raw(
                            "Connection ",
                        ),
                        Span::styled(
                            "● ",
                            match self.connection_status {
                                ConnectionStatus::Disconnected => light_red,
                                ConnectionStatus::Connecting => light_red_blink,
                                ConnectionStatus::Connected => light_green,
                            },
                        ),
                        Span::raw(
                            "Autosplitter ",
                        ),
                        Span::styled(
                            "● ",
                            match self.splitter_status {
                                AutosplitterStatus::NotRunning => light_red,
                                AutosplitterStatus::Running => light_green,
                                AutosplitterStatus::Attached => light_blue,
                            },
                        ),
                    ]).style(
                        Style::new()
                        .fg(Color::Black)
                        .bg(Color::DarkGray)
                        //.add_modifier(Modifier::BOLD)
                    )
                    .right_aligned();
                    frame.render_widget(status, Rect::new(frame.area().x, frame.area().height - 1, frame.area().width, 1) );
                })?;

                //let current_time = time::Instant::now();
                //println!("RENDER TIME: {}", (current_time - render_start_time).as_millis());
                next_tick += self.tick_duration;

                let should_quit = self.wait_poll_messages(next_tick);

                if should_quit {
                    break;
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }).unwrap()
    }

    /// Performs a safe sleep but with `recv_timeout`, so we can read messages while
    /// waiting on the next tick.
    /// Returns a bool that is true if the UI should terminate.
    fn wait_poll_messages(&mut self, target: std::time::Instant) -> bool {
        let mut cur_time = std::time::Instant::now();
        while cur_time < target {
            cur_time = std::time::Instant::now();

            if target - cur_time > std::time::Duration::from_millis(3) {
                match self.receiver.recv_timeout(std::time::Duration::from_millis(1)) {
                    Ok(message) => if self.service_message(message) {
                        return true;
                    },
                    Err(e) => match e {
                        mpsc::RecvTimeoutError::Timeout => {},
                        mpsc::RecvTimeoutError::Disconnected => panic!("Receiver disconnected! Error: {}", e),
                    },
                }
            }
        }

        false
    }

    fn service_message(&mut self, message: UIMessage) -> bool {
        match message {
            UIMessage::Log(msg) => {
                if self.log_lines.len() < 1000 {
                    self.log_lines.push(msg);
                    self.log_front = (self.log_lines.len() - 1) as u32;
                } else {
                    self.log_front = (self.log_front + 1) % 1000;
                    self.log_lines[self.log_front as usize] = msg;
                }

                false
            },
            UIMessage::AutosplitterStatus(status) => {
                self.splitter_status = status;

                false
            },
            UIMessage::ConnectionStatus(status) => {
                self.connection_status = status;

                false
            },
            UIMessage::Stop => true,
        }
    }
}
