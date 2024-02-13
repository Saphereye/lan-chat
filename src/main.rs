extern crate clap;
use clap::{App, Arg};
mod networking;
use crossterm::style::Stylize;
use networking::*;
use std::io::{self, stdout};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use tui_textarea::{Input, Key, TextArea};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let message_vector: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
    let message_vector_clone = Arc::clone(&message_vector);

    if args.contains(&"server".to_string()) {
        run_server("172.16.50.209");
    } else if args.len() > 1 {
        let server_ip = args[1].clone();
        let mut stream = TcpStream::connect(server_ip).unwrap();
        let mut stream_clone = stream.try_clone().unwrap();
        std::thread::spawn(move || {
            run_client(&mut stream, message_vector_clone);
        });

        enable_raw_mode()?;
        crossterm::execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.show_cursor()?;
        let mut text_area = TextArea::default();
        text_area.set_cursor_line_style(Style::default());
        text_area.set_placeholder_text("Enter message here");

        let mut should_quit = false;
        while !should_quit {
            terminal.draw(|f| ui(f, Arc::clone(&message_vector), &mut text_area))?;
            should_quit = handle_events(
                Arc::clone(&message_vector),
                &mut text_area,
                &mut stream_clone,
            )?;
        }

        disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    } else {
        panic!("Please provide a server IP address");
    }

    Ok(())
}

fn handle_events(
    message_vector: Arc<Mutex<Vec<Message>>>,
    text_area: &mut TextArea,
    stream: &mut TcpStream,
) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => {
                        send_message(
                            stream,
                            &Message::Leave(stream.local_addr().unwrap().to_string()),
                        );
                        return Ok(true);
                    }
                    KeyCode::Enter => {
                        let message = text_area.lines()[0].clone();
                        send_message(
                            stream,
                            &Message::Message(stream.local_addr().unwrap().to_string(), message),
                        );
                        text_area.delete_line_by_end();
                    }
                    ref key_code => {
                        // Handle other keys
                        let input = Input {
                            key: Key::from(*key_code),
                            ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
                            shift: key.modifiers.contains(KeyModifiers::SHIFT),
                            alt: key.modifiers.contains(KeyModifiers::ALT),
                        };
                        text_area.input(input);
                    }
                }
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, message_vector: Arc<Mutex<Vec<Message>>>, text_area: &mut TextArea) {
    // Lock the Mutex and get a reference to the Vec<Message>
    let messages = message_vector.lock().unwrap();

    // Create a new Vec and append each Message to it
    let mut message_lines = vec![];
    for message in messages.iter() {
        let span = match message {
            Message::Info(info) => Span::styled(info.clone(), Style::default().fg(Color::Green)),
            Message::Leave(leave) => {
                let formatted_leave = format!("{} has left the chat", leave);
                Span::styled(formatted_leave, Style::default().fg(Color::Yellow))
            },
            Message::Message(source, message) => {
                let formatted_message = format!("({}): {}", source, message);
                Span::styled(formatted_message, Style::default().fg(Color::White))
            }
            Message::Error(error) => Span::styled(error.clone(), Style::default().fg(Color::Red)),
            Message::Command(command) => {
                Span::styled(command.clone(), Style::default().fg(Color::Blue))
            }
        };
        message_lines.push(Line::from(span));
    }

    // Split the frame into two rows, one for the messages and one for the text area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(frame.size());

    let height_of_paragraph = chunks[0].height as usize;
    let scroll = if message_lines.len() > height_of_paragraph {
        message_lines.len() - height_of_paragraph
    } else {
        0
    };

    // Display the messages on the screen
    frame.render_widget(
        Paragraph::new(message_lines).scroll((scroll as u16, 0))
            .block(Block::default().title("Lan Chat 💬").borders(Borders::ALL)),
        chunks[0],
    );
    frame.render_widget(text_area.widget(), chunks[1]);
}
