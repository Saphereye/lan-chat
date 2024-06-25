//! A simple chat application.
//!
//! The application uses a client-server model where many clients are connected to a single server.
//! When the user sends a message to server using TCP, its broadcasted to all the connected clients.
//! The chat application uses a terminal based interface to allow usage even in the absence of a GUI.

extern crate clap;
use clap::Parser;
mod networking;
use env_logger::Builder;
use log::*;
use networking::*;
use std::io::{self, stdout};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

fn replace_keywords_with_emojis(text: &str) -> String {
    let mut output = String::new();
    let mut current_word = String::new();
    let mut inside_keyword = false;

    for ch in text.chars() {
        match ch {
            ':' => {
                if inside_keyword {
                    if let Some(emoji) = emojis::get_by_shortcode(&current_word) {
                        output.push_str(emoji.as_str());
                    } else {
                        output.push(':');
                        output.push_str(&current_word);
                        output.push(':');
                    }
                    current_word.clear();
                }
                inside_keyword = !inside_keyword;
            }
            _ => {
                if inside_keyword {
                    current_word.push(ch);
                } else {
                    output.push(ch);
                }
            }
        }
    }

    output
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use tui_textarea::{Input, Key, TextArea};

const MAX_NAME_LENGTH: usize = 10;

/// The main function of the application. It is responsible for parsing the command line arguments and starting the server or client based on the arguments.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Use to start the application as a server.
    #[arg(short, long)]
    is_server: bool,
    /// The IP address of the target server.
    #[arg(short, long)]
    server_ip: Option<String>,
    /// The pseudonym of the user.
    #[arg(short, long)]
    pseudonym: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    Builder::new().filter(None, LevelFilter::Info).init();

    if args.is_server {
        run_server(get_local_ip()?.as_str())?;
        return Ok(());
    }

    let message_vector: Arc<Mutex<Vec<MessageType>>> = Arc::new(Mutex::new(Vec::new()));
    let message_vector_clone = Arc::clone(&message_vector);

    let pseudonym = match args.pseudonym {
        Some(pseudonym) if (pseudonym.len() <= MAX_NAME_LENGTH && pseudonym.len() > 0) => pseudonym,
        Some(_) | None => {
            let mut pseudonym = String::new();

            loop {
                print!("Enter your pseudonym (0 <= size <= {}): ", MAX_NAME_LENGTH);
                io::Write::flush(&mut io::stdout())?;
                io::stdin().read_line(&mut pseudonym)?;
                pseudonym = pseudonym.trim().to_string();

                if pseudonym.len() > MAX_NAME_LENGTH {
                    println!("Pseudonym too long (currently {} chars). Please enter a pseudonym with less than {} characters", pseudonym.len(), MAX_NAME_LENGTH);
                    pseudonym = String::new();
                    continue;
                } else if pseudonym.is_empty() {
                    println!("Pseudonym cannot be empty. Please enter a pseudonym");
                    pseudonym = String::new();
                    continue;
                } else {
                    break;
                }
            }

            pseudonym
        }
    };

    let server_ip = match args.server_ip {
        Some(server_ip) => server_ip,
        None => {
            println!("Please provide a target server IP address to connect to it. Try lan-chat --help for more info");
            std::process::exit(1);
        }
    };

    let mut stream = TcpStream::connect(server_ip)?;
    let mut stream_clone = stream.try_clone()?;

    let pseduonym_clone = pseudonym.clone();
    std::thread::spawn(move || {
        run_client(&mut stream, message_vector_clone, pseudonym.clone()).unwrap();
    });

    enable_raw_mode()?;
    crossterm::execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.show_cursor()?;
    let mut text_area = TextArea::default();
    let mut scroll = 0;
    text_area.set_cursor_line_style(Style::default());
    text_area.set_placeholder_text("Enter message here");

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| ui(f, Arc::clone(&message_vector), &mut text_area, &mut scroll))?;
        should_quit = match handle_events(
            Arc::clone(&message_vector),
            &mut text_area,
            &mut stream_clone,
            &mut scroll,
            pseduonym_clone.clone(),
        ) {
            Ok(should_quit) => should_quit,
            Err(e) => {
                message_vector
                    .lock()
                    .unwrap()
                    .push(MessageType::Error(e.to_string()));
                false
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Handles the events for the UI. Returns true if the user wants to quit the application.
fn handle_events(
    message_vector: Arc<Mutex<Vec<MessageType>>>,
    text_area: &mut TextArea,
    stream: &mut TcpStream,
    scroll: &mut u16,
    pseudonym: String,
) -> io::Result<bool> {
    let mut message_vector = message_vector.lock().unwrap();
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Enter => {
                        let message = text_area.lines()[0].clone();

                        let message = message.trim().to_string();
                        let message = replace_keywords_with_emojis(&message);

                        if let Some(prefix) = message.strip_prefix('/') {
                            let args: Vec<&str> = prefix.split_whitespace().collect();
                            match args[0] {
                                "help" => {
                                    message_vector.push(MessageType::Info("".to_string()));
                                    message_vector.push(MessageType::Info(format!(
                                        "Running program version {}, Created by {}",
                                        env!("CARGO_PKG_VERSION"),
                                        env!("CARGO_PKG_AUTHORS")
                                    )));

                                    message_vector.push(MessageType::Info("Commands:".to_string()));
                                    message_vector.push(MessageType::Info(
                                        "/help - Display this message".to_string(),
                                    ));
                                    message_vector.push(MessageType::Info(
                                        "/quit - Quit the chat".to_string(),
                                    ));
                                    message_vector.push(MessageType::Info(
                                        "/file <file path> - Send file at file path".to_string(),
                                    ));
                                    message_vector.push(MessageType::Info(
                                        "/image <file path> - Send image at file path".to_string(),
                                    ));

                                    message_vector.push(MessageType::Info("".to_string()));
                                    message_vector.push(MessageType::Info(
                                        "To put emojis use the ':description:' format, e.g. use :smile: to send 😊"
                                            .to_string(),
                                    ));

                                    message_vector.push(MessageType::Info("".to_string()));
                                }
                                "quit" => {
                                    send_message(
                                        stream,
                                        &MessageType::Leave(
                                            stream.local_addr().unwrap().to_string(),
                                        ),
                                    )?;
                                    return Ok(true);
                                }
                                "file" => {
                                    if let Some(file_path) = args.get(1) {
                                        match std::fs::read(file_path) {
                                            Ok(file_contents) => {
                                                message_vector.push(MessageType::Info(format!(
                                                    "Sending file : {}",
                                                    args[1]
                                                )));

                                                send_message(
                                                    stream,
                                                    &MessageType::File(
                                                        file_path.to_string(),
                                                        file_contents,
                                                    ),
                                                )?;
                                            }
                                            Err(e) => {
                                                // Handle file read error
                                                message_vector.push(MessageType::Error(format!(
                                                    "Failed to read file: {}",
                                                    e
                                                )));
                                            }
                                        }
                                    } else {
                                        // Handle case where file path is not provided
                                        message_vector.push(MessageType::Error(
                                            "File path not provided".to_string(),
                                        ));
                                    }
                                }
                                "image" => {
                                    message_vector.push(MessageType::Error(
                                        "Image transfer not implemented yet".to_string(),
                                    ));
                                }
                                _ => {
                                    message_vector.push(MessageType::Error(
                                        "Invalid command. Type /help for a list of commands"
                                            .to_string(),
                                    ));
                                }
                            }

                            send_message(stream, &MessageType::Command(prefix.to_string()))?;
                            message_vector.push(MessageType::Command(prefix.to_string()));

                            while !text_area.is_empty() {
                                text_area.delete_char();
                            }

                            return Ok(false);
                        }

                        if !message.is_empty() {
                            send_message(stream, &MessageType::Message(pseudonym, message))?;
                            *scroll = scroll.saturating_add(1);
                        }

                        while !text_area.is_empty() {
                            text_area.delete_char();
                        }
                    }
                    KeyCode::Up => {
                        *scroll = scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        *scroll = scroll.saturating_add(1);
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

/// Responsible for drawing the UI. Interfaces with the message vector of the screen.
fn ui(
    frame: &mut Frame,
    message_vector: Arc<Mutex<Vec<MessageType>>>,
    text_area: &mut TextArea,
    scroll: &mut u16,
) {
    // Lock the Mutex and get a reference to the Vec<Message>
    let messages = message_vector.lock().unwrap();

    // Create a new Vec and append each Message to it
    let mut message_lines = vec![];
    for message in messages.iter() {
        let span = match message {
            MessageType::Info(info) => {
                Span::styled(info.clone(), Style::default().fg(Color::Green))
            }
            MessageType::Leave(leave) => {
                let formatted_leave = format!("{} has left the chat", leave);
                Span::styled(formatted_leave, Style::default().fg(Color::Yellow))
            }
            MessageType::Message(source, message) => {
                let formatted_message =
                    format!("{:^width$}: {}", source, message, width = MAX_NAME_LENGTH);
                Span::styled(formatted_message, Style::default().fg(Color::White))
            }
            MessageType::Error(error) => {
                Span::styled(error.clone(), Style::default().fg(Color::Red))
            }
            MessageType::File(file_name, file_contents) => {
                // Extract the file name, ignoring any path components
                let file_name_only = std::path::Path::new(&file_name)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("downloaded_file");

                // Attempt to get the current directory
                match std::env::current_dir() {
                    Ok(current_dir) => {
                        let full_path = current_dir.join(file_name_only);

                        // Attempt to write the file_contents to the file in the current directory
                        match std::fs::write(&full_path, file_contents) {
                            Ok(_) => {
                                let formatted_file = format!("Received file: {}", file_name_only);
                                Span::styled(formatted_file, Style::default().fg(Color::Blue))
                            }
                            Err(e) => {
                                let error_message = format!("Failed to write file: {}", e);
                                Span::styled(error_message, Style::default().fg(Color::Red))
                            }
                        }
                    }
                    Err(e) => {
                        let error_message = format!("Failed to get current directory: {}", e);
                        Span::styled(error_message, Style::default().fg(Color::Red))
                    }
                }
            }
            _ => continue,
        };
        message_lines.push(Line::from(span));
    }

    // Split the frame into two rows, one for the messages and one for the text area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(frame.size());

    // Display the messages on the screen
    frame.render_widget(
        Paragraph::new(message_lines)
            .scroll((*scroll, 0))
            .block(Block::default().title("Lan Chat 💬").borders(Borders::ALL)),
        chunks[0],
    );
    frame.render_widget(text_area.widget(), chunks[1]);
}
