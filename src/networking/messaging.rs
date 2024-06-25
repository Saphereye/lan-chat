//! Contains the message type and functions to send and receive messages between clients and the server.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;

/// Message size in bytes
pub const MAX_MESSAGE_SIZE: usize = 100_000;

/// A message that can be sent between clients and the server.
///
/// The numerous types of messages are categorized to help display the same in a better manner.
/// Info, Leave, Error and Command (in progress) just need the text
/// Message requires the content and the sender information
/// Pseudonym is used to initiliaze or update a pseuodonym
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessageType {
    Info(String),            // Info message by server
    Leave(String),           // Leaving message
    Message(String, String), // Pseudonym and the message itself
    Error(String),           // Error message by server
    Command(String),         // Not yet implemented
    Pseudonym(String),       // User pseudonym
    File(String, Vec<u8>),   // File name, file content. This will be downloaded on client
    Image(String, Vec<u8>),  // Image name, image content. Will be shown in sixel format on client
                             // ? can prolly add an incomplete message, to get message larger than MAX_MESSAGE_SIZE
}

/// Responsible for sending a message given stream and message enum
pub fn send_message(stream: &mut TcpStream, message: &MessageType) -> std::io::Result<()> {
    let bytes = bincode::serialize(&message)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if bytes.len() > MAX_MESSAGE_SIZE && !matches!(message, MessageType::Leave(_)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Message is too large to send",
        ));
    }

    stream.write_all(&bytes)?;
    stream.flush()?;

    Ok(())
}

/// Responsible for receiving a message given stream
pub fn receive_message(stream: &mut TcpStream) -> Result<MessageType, Box<dyn std::error::Error>> {
    // BUG Files with larger sizes may give issues
    let mut buffer = [0; MAX_MESSAGE_SIZE];
    match stream.read(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            return Err(format!(
                "Couldn't read from stream properly. Receiving from: {}. Read error: {}",
                stream.peer_addr()?,
                e
            )
            .into());
        }
    }

    let message: MessageType = bincode::deserialize(&buffer)?;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;
    #[test]
    fn test_send_message() {
        let sender = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = sender.local_addr().unwrap();
        let mut stream = TcpStream::connect(addr).unwrap();

        let message = MessageType::Info("Test message".to_string());
        assert!(send_message(&mut stream, &message).is_ok());
    }

    #[test]
    fn test_receive_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a new thread to send the message.
        let sender_handle = std::thread::spawn(move || {
            let mut sender_stream = TcpStream::connect(addr).unwrap();
            let message = MessageType::Info("Test message".to_string());
            send_message(&mut sender_stream, &message).unwrap();
        });

        // Accept the connection from the sender.
        let (mut receiver_stream, _) = listener.accept().unwrap();
        let output_message = receive_message(&mut receiver_stream).unwrap();

        // Wait for the sender thread to finish.
        sender_handle.join().unwrap();

        assert_eq!(
            MessageType::Info("Test message".to_string()),
            output_message
        );
    }
}
