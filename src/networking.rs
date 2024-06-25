use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
extern crate if_addrs;
use if_addrs::get_if_addrs;
use lazy_static::lazy_static;
use log::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

const MAX_MESSAGE_SIZE: usize = 1024;

lazy_static! {
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
}

/// The chat server. Contains a list of clients and can broadcast messages to all of them.
#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<Vec<(TcpStream, String, String)>>>, // Stream, address, pseudonym
}

impl Server {
    fn new() -> Self {
        Server {
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a new client to the server. On addition it broadcasts a message to all the existing clients that a new client has joined and logs to server.
    fn add_client(
        &self,
        client: TcpStream,
        addr: String,
        pseudonym: String,
    ) -> Result<(), Box<dyn std::error::Error + '_>> {
        let mut clients = self.clients.lock()?;
        clients.push((client, addr, pseudonym.clone()));

        Ok(())
    }

    /// Implementation of broadcasting a message to all the clients. Also logs the message to the server.
    fn broadcast(&self, message: &MessageType) -> Result<(), Box<dyn std::error::Error + '_>> {
        let mut clients = self.clients.lock()?;
        // println!("In broadcast: {:?}", clients);
        match message {
            MessageType::Message(pseudonym, ref message_string) => {
                for (client, _, _) in clients.iter_mut() {
                    send_message(client, message)?;
                }
                info!("({}): {}", pseudonym, message_string);
            }
            MessageType::Leave(addr) => {
                self.remove_client(addr)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Removes a client from the server. Also broadcasts a message to all the clients that the client has left and logs to server.
    fn remove_client(&self, addr: &str) -> Result<(), Box<dyn std::error::Error + '_>> {
        let mut clients = self.clients.lock()?;
        // Find and remove the client by address
        if let Some(index) = clients.iter().position(|(_, a, _)| a == addr) {
            let (_, _, p) = clients.remove(index);
            // Notify all clients about the departure
            for (client, _, _) in &mut *clients {
                send_message(client, &MessageType::Leave(p.clone()))?;
                client.flush()?;
            }
            warn!("{} (pseudonym: {}) has left the chat.", addr, p);
        }
        Ok(())
    }
}

pub fn get_local_ip() -> io::Result<String> {
    if let Ok(interfaces) = get_if_addrs() {
        for interface in interfaces {
            if !interface.is_loopback() && !interface.addr.is_link_local() {
                if let if_addrs::IfAddr::V4(ref addr) = interface.addr {
                    return Ok(addr.ip.to_string());
                }
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to retrieve local IP address.",
    ))
}

/// Runs the server. The server listens for incoming connections and spawns a new thread for each one.
/// TODO when server is SIGTERM kick all clients and close
pub fn run_server(server_ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new();

    // let listener = TcpListener::bind(format!("{server_ip}:0")).unwrap();
    let listener = TcpListener::bind(format!("{server_ip}:0"))?;
    println!("Server listening on {}", listener.local_addr()?);
    println!(
        "To join the chat, use the following command: lan-chat -s {}",
        listener.local_addr()?
    );
    println!(
        "Running program version {}, Created by {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );

    for stream in listener.incoming() {
        let mut stream = stream?;
        let server = server.clone();
        let client_addr = stream.peer_addr()?.to_string();
        let client_addr_clone = client_addr.clone();

        server
            .add_client(
                stream.try_clone().unwrap(),
                client_addr.clone(),
                "[blank]".to_string(),
            )
            .unwrap();
        thread::spawn(move || {
            let server = server.clone();
            while let Ok(message) = receive_message(&mut stream) {
                match message {
                    MessageType::Leave(addr) => {
                        if let Err(e) = server.remove_client(&addr) {
                            error!(
                                "Failed to remove client: {}. Client removal error: {}",
                                addr, e
                            );
                            std::process::exit(1);
                        }
                    }
                    MessageType::Message(_, _) => {
                        if let Err(e) = server.broadcast(&message) {
                            error!("Failed to broadcast message. Broadcasting error: {}", e);
                            std::process::exit(1);
                        }
                    }
                    MessageType::Command(command) => {
                        info!(
                            "Client {} has run the command '{}'",
                            client_addr_clone, command
                        );
                    }
                    MessageType::Pseudonym(pseudonym) => {
                        let mut clients = server.clients.lock().unwrap();
                        if let Some(index) = clients.iter().position(|(_, a, _)| a == &client_addr)
                        {
                            clients[index].2.clone_from(&pseudonym);
                        }

                        info!(
                            "{} has entered the chat with the pseudonym '{}'",
                            client_addr_clone, pseudonym
                        );

                        // Notify all existing clients about the new client
                        let join_message = format!("{} has entered the chat.", pseudonym);
                        for (existing_client, _, _) in &mut *clients {
                            send_message(existing_client, &MessageType::Info(join_message.clone()))
                                .unwrap();
                        }
                    }
                    MessageType::File(file_name, file_contents) => {
                        info!("{} has sent a file: {}", client_addr_clone, file_name);
                        let mut clients = server.clients.lock().unwrap();
                        for (client, _, _) in &mut *clients {
                            if client.peer_addr().unwrap().to_string() == client_addr {
                                continue;
                            }

                            send_message(
                                client,
                                &MessageType::File(file_name.clone(), file_contents.clone()),
                            )
                            .unwrap();
                        }
                    }
                    _ => {}
                }
            }

            if let Err(e) = server.remove_client(&client_addr) {
                error!("Failed to remove client: {}. Reason: {}", client_addr, e);
            };
        });
    }

    Ok(())
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
fn receive_message(stream: &mut TcpStream) -> Result<MessageType, Box<dyn std::error::Error>> {
    // BUG Images with larger sizes may give issues
    let mut buffer = [0; MAX_MESSAGE_SIZE];
    match stream.read(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            // return nice error
            return Err(Box::new(e));
            // error!(
            //     "Couldn't read from stream properly. Receiving from: {}. Read error: {}",
            //     stream.peer_addr().unwrap().to_string(),
            //     e
            // );
            // std::process::exit(1)
        }
    }

    let message: MessageType = bincode::deserialize(&buffer)?;
    Ok(message)
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

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_is_local_ip_ok() {
        assert!(get_local_ip().is_ok());
    }
}


