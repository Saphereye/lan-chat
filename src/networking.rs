use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::net::{IpAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;
extern crate if_addrs;
use if_addrs::get_if_addrs;
use log::*;
use serde::{Deserialize, Serialize};

/// A message that can be sent between clients and the server.
///
/// The numerous types of messages are categorized to help display the same in a better manner.
/// Info, Leave, Error and Command (in progress) just need the text
/// Message requires the content and the sender information
#[derive(Serialize, Deserialize)]
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
    fn add_client(&self, client: TcpStream, addr: String) {
        let mut clients = self.clients.lock().unwrap();
        let addr_clone = addr.clone();
        clients.push((client, addr));
        info!("{} has entered the chat.", addr_clone);

        // Notify all existing clients about the new client
        let join_message = format!("{} has entered the chat.", addr_clone);
        for (existing_client, _) in &*clients {
            let mut existing_client = existing_client.try_clone().unwrap();
            send_message(&mut existing_client, &MessageType::Info(join_message.clone())).unwrap();
        }
    }

    /// Implementation of broadcasting a message to all the clients. Also logs the message to the server.
    fn broadcast(&self, message: &MessageType) {
        let mut clients = self.clients.lock().unwrap();
        // println!("In broadcast: {:?}", clients);
        match message {
            MessageType::Message(sender_addr, ref message_string) => {
                for (client, _) in clients.iter_mut() {
                    send_message(client, message).unwrap();
                }
                info!("({}): {}", sender_addr, message_string);
            }
            MessageType::Leave(addr) => {
                self.remove_client(addr);
            }
            _ => {}
        }
    }

    /// Removes a client from the server. Also broadcasts a message to all the clients that the client has left and logs to server.
    fn remove_client(&self, addr: &str) {
        let mut clients = self.clients.lock().unwrap();
        // Find and remove the client by address
        if let Some(index) = clients.iter().position(|(_, a)| a == addr) {
            clients.remove(index);
            // Notify all clients about the departure
            for (client, _) in &mut *clients {
                send_message(client, &MessageType::Leave(addr.to_string())).unwrap();
                client.flush().unwrap();
            }
        }
        warn!("{} has left the chat.", addr);
    }
}

pub fn get_local_ip() -> io::Result<String> {
    if let Ok(interfaces) = get_if_addrs() {
        for interface in interfaces {
            if let IpAddr::V4(ipv4) = interface.ip() {
                return Ok(ipv4.to_string());
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to retrieve local IP address.",
    ))
}

/// Runs the server. The server listens for incoming connections and spawns a new thread for each one.
pub fn run_server(server_ip: &str) {
    // println!("Initializing server");
    let server = Server::new();

    // let listener = TcpListener::bind(format!("{server_ip}:0")).unwrap();
    let listener = TcpListener::bind(format!("{server_ip}:0")).unwrap();
    println!("Server listening on {}", listener.local_addr().unwrap());
    println!(
        "To join the chat, use the following command: lan-chat {}",
        listener.local_addr().unwrap()
    );
    println!(
        "Running program version {}, Created by {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let server = server.clone();
        let client_addr = stream.peer_addr().unwrap().to_string();
        server.add_client(stream.try_clone().unwrap(), client_addr.clone());
        thread::spawn(move || {
            let server = server.clone();
            while let Ok(message) = receive_message(&mut stream) {
                match message {
                    MessageType::Leave(addr) => {
                        server.remove_client(&addr);
                    }
                    MessageType::Message(_, _) => {
                        server.broadcast(&message);
                    }
                    _ => {}
                }
            }
            server.remove_client(&client_addr);
        });
    }
}

/// Responsible for sending a message given stream and message enum
pub fn send_message(stream: &mut TcpStream, message: &MessageType) -> std::io::Result<()> {
    let bytes = bincode::serialize(&message)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    stream.write_all(&bytes)?;
    stream.flush()
}

/// Responsible for receiving a message given stream
fn receive_message(stream: &mut TcpStream) -> std::io::Result<MessageType> {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    }
    let message: MessageType = bincode::deserialize(&buffer).unwrap();
    Ok(message)
}

/// Runs the client. Connects to the server and receives server messages.
pub fn run_client(stream: &mut TcpStream, message_vector: Arc<Mutex<Vec<MessageType>>>) {
    if let Ok(s) = stream.local_addr() {
        message_vector
            .lock()
            .unwrap()
            .push(MessageType::Info(format!("Your ip is: {}", s)));
    };

    // Print the server's address
    match stream.peer_addr() {
        Ok(addr) => {
            message_vector.lock().unwrap().push(MessageType::Info(format!(
                "Connected to server at address: {}",
                addr
            )));
        }
        Err(e) => {
            error!("Failed to retrieve server address: {}", e);
            std::process::exit(1);
        }
    }
    message_vector.lock().unwrap().push(MessageType::Info(
        "To quit the chat, type /quit and press enter".to_string(),
    ));
    message_vector.lock().unwrap().push(MessageType::Info(
        "To send a message, type your message and press enter".to_string(),
    ));
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
}
