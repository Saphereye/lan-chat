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

lazy_static! {
    static ref TIPS: Mutex<Vec<String>> = Mutex::new(vec![
        "Type /help in the chat".to_string(),
        "Use arrow keys to see chat history".to_string(),
        "Type /quit to leave program".to_string(),
    ]);
}

/// A message that can be sent between clients and the server.
///
/// The numerous types of messages are categorized to help display the same in a better manner.
/// Info, Leave, Error and Command (in progress) just need the text
/// Message requires the content and the sender information
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessageType {
    Info(String),
    Leave(String),
    Message(String, String), // Source and the message itself
    Error(String),
    Command(String), // Not yet implemented
}

/// The chat server. Contains a list of clients and can broadcast messages to all of them.
#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<Vec<(TcpStream, String)>>>,
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
    ) -> Result<(), Box<dyn std::error::Error + '_>> {
        let mut clients = self.clients.lock()?;
        let addr_clone = addr.clone();
        clients.push((client, addr));
        info!("{} has entered the chat.", addr_clone);

        // Notify all existing clients about the new client
        let join_message = format!("{} has entered the chat.", addr_clone);
        for (existing_client, _) in &*clients {
            let mut existing_client = existing_client.try_clone()?;
            send_message(
                &mut existing_client,
                &MessageType::Info(join_message.clone()),
            )?;
        }

        Ok(())
    }

    /// Implementation of broadcasting a message to all the clients. Also logs the message to the server.
    fn broadcast(&self, message: &MessageType) -> Result<(), Box<dyn std::error::Error + '_>> {
        let mut clients = self.clients.lock()?;
        // println!("In broadcast: {:?}", clients);
        match message {
            MessageType::Message(sender_addr, ref message_string) => {
                for (client, _) in clients.iter_mut() {
                    send_message(client, message)?;
                }
                info!("({}): {}", sender_addr, message_string);
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
        if let Some(index) = clients.iter().position(|(_, a)| a == addr) {
            clients.remove(index);
            // Notify all clients about the departure
            for (client, _) in &mut *clients {
                send_message(client, &MessageType::Leave(addr.to_string()))?;
                client.flush()?;
            }
        }
        warn!("{} has left the chat.", addr);
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
pub fn run_server(server_ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    // println!("Initializing server");
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
            .add_client(stream.try_clone().unwrap(), client_addr.clone())
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
                            error!("Failes to broadcast message. Broadcasting error: {}", e);
                            std::process::exit(1);
                        }
                    }
                    MessageType::Command(command) => {
                        info!("Client {} has run the command '{}'", client_addr_clone, command);

                        match command.as_str() {
                            "smile" => {
                                send_message(&mut stream, &MessageType::Message(client_addr_clone.clone(), "😊".to_string()))
                                    .unwrap();
                            }
                            "laugh" => {
                                send_message(&mut stream, &MessageType::Message(client_addr_clone.clone(), "😂".to_string()))
                                    .unwrap();
                            }
                            "thumbs_up" => {
                                send_message(&mut stream, &MessageType::Message(client_addr_clone.clone(), "👍".to_string()))
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            if let Err(e) = server.remove_client(&client_addr) {
                error!(
                    "Failed to remove client: {}. Client removal error: {}",
                    client_addr, e
                );
            };
        });
    }

    Ok(())
}

/// Responsible for sending a message given stream and message enum
pub fn send_message(stream: &mut TcpStream, message: &MessageType) -> std::io::Result<()> {
    let bytes = bincode::serialize(&message)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    stream.write_all(&bytes)?;
    stream.flush()?;

    Ok(())
}

/// Responsible for receiving a message given stream
fn receive_message(stream: &mut TcpStream) -> Result<MessageType, Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            error!(
                "Couldn't read from stream properly. Receiving from: {}. Read error: {}",
                stream.peer_addr().unwrap().to_string(),
                e
            );
            std::process::exit(1)
        }
    }

    let message: MessageType = bincode::deserialize(&buffer)?;
    Ok(message)
}

/// Runs the client. Connects to the server and receives server messages.
pub fn run_client(
    stream: &mut TcpStream,
    message_vector: Arc<Mutex<Vec<MessageType>>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_local_ip() {
        assert!(get_local_ip().is_ok());
    }
}
