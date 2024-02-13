extern crate clap;
use clap::{App, Arg};
mod networking;
use crossterm::style::Stylize;
use networking::*;
use std::io::{self, stdout};
use std::sync::{Arc, Mutex};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut message_vector: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
    let message_vector_clone = Arc::clone(&message_vector);

    if args.contains(&"server".to_string()) {
        run_server("172.16.50.209");
    } else if args.len() > 1 {
        let server_ip = args[1].clone();
        std::thread::spawn(move || {
            run_client(server_ip.as_str(), message_vector_clone);
        });

        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        let mut should_quit = false;
        while !should_quit {
            let message_vector_clone = Arc::clone(&message_vector);
            terminal.draw(|f| ui(f, message_vector_clone))?;
            should_quit = handle_events()?;
        }

        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
    } else {
        panic!("Please provide a server IP address");
    }

    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, message_vector: Arc<Mutex<Vec<Message>>>) {
    // Lock the Mutex and get a reference to the Vec<Message>
    let messages = message_vector.lock().unwrap();

    // Create a new Vec and append each Message to it
    let mut message_lines = vec![];
    for message in messages.iter() {
        let span = match message {
            Message::Info(info) => Span::styled(info.clone(), Style::default().fg(Color::Green)),
            Message::Leave(leave) => Span::styled(leave.clone(), Style::default().fg(Color::Gray)),
            Message::Message(message) => Span::styled(message.clone(), Style::default().fg(Color::White)),
            Message::Error(error) => Span::styled(error.clone(), Style::default().fg(Color::Red)),
            Message::Command(command) => Span::styled(command.clone(), Style::default().fg(Color::Blue)),
        };
        message_lines.push(Line::from(span));
    }

    // Display the messages on the screen
    frame.render_widget(
        Paragraph::new(message_lines)
            .block(Block::default().title("Lan Chat 💬").borders(Borders::ALL)),
        frame.size(),
    );
}
