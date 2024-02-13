use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{IpAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
extern crate if_addrs;
use if_addrs::get_if_addrs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Info(String),
    Leave(String),
    Message(String, String), // Source and the message itself
    Error(String),
    Command(String),
}

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

    fn add_client(&self, client: TcpStream, addr: String) {
        let mut client_stream = client.try_clone().unwrap();
        let mut clients = self.clients.lock().unwrap();
        let addr_clone = addr.clone();
        clients.push((client, addr));
        println!("{} has entered the chat.", addr_clone);

        // Notify all existing clients about the new client
        let join_message = format!("{} has entered the chat.", addr_clone);
        for (existing_client, _) in &*clients {
            let mut existing_client = existing_client.try_clone().unwrap();
            send_message(&mut existing_client, &Message::Info(join_message.clone())).unwrap();
        }
    }

    fn broadcast(&self, message: &Message) {
        let mut clients = self.clients.lock().unwrap();
        // println!("In broadcast: {:?}", clients);
        match message {
            Message::Message(sender_addr, ref message_string) => {
                for (client, receiver_addr) in clients.iter_mut() {
                    send_message(client, &message).unwrap();
                }
                println!("({}): {}", sender_addr, message_string);
            }
            Message::Leave(addr) => {
                self.remove_client(&addr);
            }
            _ => {}
        }
    }

    fn remove_client(&self, addr: &str) {
        let mut clients = self.clients.lock().unwrap();
        // Find and remove the client by address
        if let Some(index) = clients.iter().position(|(_, a)| a == addr) {
            clients.remove(index);
            // Notify all clients about the departure
            for (client, _) in &mut *clients {
                send_message(client, &Message::Leave(addr.to_string())).unwrap();
                client.flush().unwrap();
            }
        }
        println!("{} has left the chat.", addr);
    }
}

// pub fn get_local_ip() -> Result<String> {
//     if let Ok(interfaces) = get_if_addrs() {
//         if let IpAddr::V4(ipv4) = interfaces[1].ip() {
//             Ok(ipv4.to_string())
//         } else {
//             Err(anyhow!("Failed to retrieve local IP address."))
//         }
//     } else {
//         Err(anyhow!("Failed to retrieve network interface information."))
//     }
// }

pub fn run_server(server_ip: &str) {
    // println!("Initializing server");
    let server = Server::new();

    // let listener = TcpListener::bind(format!("{server_ip}:0")).unwrap();
    let listener = TcpListener::bind(format!("{server_ip}:0")).unwrap();
    println!("Server listening on {}", listener.local_addr().unwrap());
    println!(
        "To join the chat, use the following command: lan-chat client --server-ip {}",
        listener.local_addr().unwrap()
    );
    println!(
        "Running program version {}. Created by {}",
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
                    Message::Leave(addr) => {
                        server.remove_client(&addr);
                    }
                    Message::Message(_, _) => {
                        server.broadcast(&message);
                    }
                    _ => {}
                }
            }
            server.remove_client(&client_addr);
        });
    }
}

// fn get_nickname(stream: &mut TcpStream) -> std::io::Result<String> {
//     // Set a read timeout to prevent hanging if the client does not respond.
//     // stream.set_read_timeout(Some(Duration::from_secs(30)))?;

//     // Ask the client for their nickname.
//     stream.write_all(b"Enter your nickname: ")?;

//     // Read the response.
//     let mut buffer = Vec::new();
//     stream.read_to_end(&mut buffer)?;

//     // Convert the response to a string.
//     let nickname = String::from_utf8_lossy(&buffer).trim().to_string();

//     Ok(nickname)
// }

pub fn send_message(stream: &mut TcpStream, message: &Message) -> std::io::Result<()> {
    let bytes = bincode::serialize(&message).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    stream.write_all(&bytes);
    stream.flush()
}

fn receive_message(stream: &mut TcpStream) -> std::io::Result<Message> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;
    let message: Message = bincode::deserialize(&buffer).unwrap();
    Ok(message)
}

pub fn run_client(stream: &mut TcpStream, message_vector: Arc<Mutex<Vec<Message>>>) {
    if let Ok(s) = stream.local_addr() {
        message_vector
            .lock()
            .unwrap()
            .push(Message::Info(format!("Your ip is: {}", s)));
    };

    // Print the server's address
    match stream.peer_addr() {
        Ok(addr) => {
            message_vector.lock().unwrap().push(Message::Info(format!(
                "Connected to server at address: {}",
                addr
            )));
        }
        Err(e) => {
            panic!("Failed to retrieve server address: {}", e);
        }
    }
    message_vector.lock().unwrap().push(Message::Info(
        "To quit the chat, type /quit and press enter".to_string(),
    ));
    message_vector.lock().unwrap().push(Message::Info(
        "To send a message, type your message and press enter".to_string(),
    ));
    // make a lot of black lines after this
    for _ in 0..2 {
        message_vector.lock().unwrap().push(Message::Info("".to_string()));
    }

    // Spawn a thread to read messages from the server
    let mut server_stream = stream.try_clone().unwrap();
    thread::spawn(move || {
        while let Ok(message) = receive_message(&mut server_stream) {
            message_vector.lock().unwrap().push(message);
        }
    });

    // loop {
    //     // Read the message from user input
    //     // print!("Enter a message to send to the server:");
    //     let mut message = String::new();
    //     std::io::stdin()
    //         .read_line(&mut message)
    //         .expect("Failed to read input");
    //     let message = message.trim(); // Remove trailing newline

    //     stream.write_all(message.as_bytes()).unwrap();
    //     // thread::sleep(Duration::from_secs(5));
    //     stream.flush().unwrap();

    //     if message == "/quit" {
    //         break;
    //     }
    // }
}
