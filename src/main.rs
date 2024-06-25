#![doc = include_str!("../README.md")]

use clap::Parser;
use env_logger::Builder;
use crate::networking::client::run_client;
use crate::networking::messaging::MessageType;
use crate::networking::server::{get_local_ipv4, run_server};
use crate::tui_handler::{handle_events, ui, MAX_NAME_LENGTH};
use log::*;
use std::io::{self, stdout};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
mod networking;
mod tui_handler;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tui_textarea::TextArea;

/// Defines the command line arguments used by clap crate.
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
        run_server(get_local_ipv4()?.as_str())?;
        return Ok(());
    }

    let message_vector: Arc<Mutex<Vec<MessageType>>> = Arc::new(Mutex::new(Vec::new()));
    let message_vector_clone = Arc::clone(&message_vector);

    let pseudonym = match args.pseudonym {
        Some(pseudonym) if (pseudonym.len() <= MAX_NAME_LENGTH && pseudonym.is_empty()) => {
            pseudonym
        }
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
            return Ok(());
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

    // Main loop
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
