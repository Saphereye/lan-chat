//! Contains the client code for the chat program
//! 
//! Includes the main client loop and the function to run the client.
//! Also contains the tips that are displayed to the user when they join the chat.

use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
extern crate if_addrs;
use lazy_static::lazy_static;
use log::*;
use rand::Rng;

use crate::networking::messaging::{receive_message, send_message, MessageType};

lazy_static! {
    /// A vector of tips that are displayed to the user when they join the chat.
    static ref TIPS: Mutex<Vec<String>> = Mutex::new(vec![
        "Type /help in the chat".to_string(),
        "Use arrow keys to see chat history".to_string(),
        "Type /quit to leave program".to_string(),
        "Use :smile: to insert a smiley, try :laughing: and :thumbsup: too. Look at 'gemoji' to learn more.".to_string(),
        "If you get 'file received' message, make sure to check your pwd (^ u ^)".to_string(),
        "Petting a cat increases your life span by 101% ^._.^".to_string(),
        "If you have any suggestions or feedback, please let us know!".to_string(),
        "If you have any issues, please report them on the GitHub page.".to_string(),
    ]);
}

/// Runs the client. Connects to the server and receives server messages.
pub fn run_client(
    stream: &mut TcpStream,
    message_vector: Arc<Mutex<Vec<MessageType>>>,
    pseudonym: String,
) -> Result<(), Box<dyn std::error::Error>> {
    match send_message(stream, &MessageType::Pseudonym(pseudonym)) {
        Ok(_) => {}
        Err(e) => {
            message_vector
                .lock()
                .unwrap()
                .push(MessageType::Error(format!(
                    "Failed to send pseudonym to server: {}",
                    e
                )));
        }
    } // Send the pseudonym to the server

    if let Ok(s) = stream.local_addr() {
        message_vector
            .lock()
            .unwrap()
            .push(MessageType::Info(format!("Your ip is: {}", s)));
    };

    // Print the server's address
    match stream.peer_addr() {
        Ok(addr) => {
            message_vector
                .lock()
                .unwrap()
                .push(MessageType::Info(format!(
                    "Connected to server at address: {}",
                    addr
                )));
        }
        Err(e) => {
            error!("Failed to retrieve server address: {}", e);
            std::process::exit(1);
        }
    }

    // Choose a random tip
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..TIPS.lock().unwrap().len());
    let tip = TIPS.lock().unwrap()[index].clone();
    message_vector
        .lock()
        .unwrap()
        .push(MessageType::Info(format!("TIP: {}", tip)));

    // make a lot of black lines after this
    for _ in 0..2 {
        message_vector
            .lock()
            .unwrap()
            .push(MessageType::Info("".to_string()));
    }

    // Spawn a thread to read messages from the server
    let mut server_stream = stream.try_clone().unwrap();
    thread::spawn(move || {
        while let Ok(message) = receive_message(&mut server_stream) {
            message_vector.lock().unwrap().push(message);
        }
    });

    Ok(())
}
