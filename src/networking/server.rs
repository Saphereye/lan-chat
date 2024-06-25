//! The server module contains the server implementation for the chat application.
//! 
//! It listens for incoming connections and broadcasts messages to all the clients.
//! and maintains a list of clients from which it can remove them.

use std::io::{self, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
extern crate if_addrs;
use if_addrs::get_if_addrs;
use log::*;

use crate::networking::messaging::{receive_message, send_message, MessageType};

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

// TODO when server is SIGTERM kick all clients and close
/// Runs the server. The server listens for incoming connections and spawns a new thread for each one.
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

/// Returns the local IPv4 address of the machine.
pub fn get_local_ipv4() -> io::Result<String> {
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
        "Failed to retrieve local IPv4 address.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_local_ip_ok() {
        assert!(get_local_ipv4().is_ok());
    }
}
