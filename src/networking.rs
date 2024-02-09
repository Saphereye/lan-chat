use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{IpAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;
extern crate if_addrs;
use if_addrs::get_if_addrs;

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
        let mut clients = self.clients.lock().unwrap();
        let addr_clone = addr.clone();
        clients.push((client, addr));
        println!("{} has entered the chat.", addr_clone);

        // Notify all existing clients about the new client
        let join_message = format!("{} has entered the chat.", addr_clone);
        for (existing_client, _) in &*clients {
            let mut existing_client = existing_client.try_clone().unwrap();
            existing_client.write_all(join_message.as_bytes()).unwrap();
        }
    }

    fn broadcast(&self, message: &[u8], sender_addr: &str) {
        let clients = self.clients.lock().unwrap();
        // println!("In broadcast: {:?}", clients);
        for (client, receiver_addr) in &*clients {
            if receiver_addr != sender_addr {
                let mut client = client;
                client
                    .write_all(
                        format!("({}): {}", sender_addr, String::from_utf8_lossy(message))
                            .as_bytes(),
                    )
                    .unwrap();
            }
        }
        println!("({}): {}", sender_addr, String::from_utf8_lossy(message));
    }

    fn remove_client(&self, addr: &str) {
        let mut clients = self.clients.lock().unwrap();
        // Find and remove the client by address
        if let Some(index) = clients.iter().position(|(_, a)| a == addr) {
            clients.remove(index);
            // Notify all clients about the departure
            for (client, _) in &mut *clients {
                let leave_message = format!("{} has left the chat.", addr);
                let _ = client.write_all(leave_message.as_bytes());
            }
        }
        println!("{} has left the chat.", addr);
    }
}

pub fn get_local_ip() -> Result<String> {
    if let Ok(interfaces) = get_if_addrs() {
        if let IpAddr::V4(ipv4) = interfaces[1].ip() {
            return Ok(ipv4.to_string());
        } else {
            return Err(anyhow!("Failed to retrieve local IP address."));
        }
    } else {
        return Err(anyhow!("Failed to retrieve network interface information."));
    }
}

pub fn run_server(server_ip: &str) {
    println!("Initializing server");
    let server = Server::new();

    let listener = TcpListener::bind(format!("{server_ip}:0")).unwrap();
    println!("Server listening on {}", listener.local_addr().unwrap());
    println!(
        "To join the chat, use the following command: lan-chat client --server-ip {}",
        listener.local_addr().unwrap()
    );
    println!("Running program version {}. Created by Saphereye <adarshdas950@gmail.com>", env!("CARGO_PKG_VERSION"));

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let server = server.clone();
        let client_addr = stream.peer_addr().unwrap().to_string();
        server.add_client(stream.try_clone().unwrap(), client_addr.clone());
        thread::spawn(move || {
            let server = server.clone();
            let mut buffer = [0; 512];
            while let Ok(len) = stream.read(&mut buffer) {
                if len == 0 {
                    break;
                }

                if String::from_utf8_lossy(&buffer).as_ref() == "/quit" {
                    server.remove_client(&client_addr);
                }

                server.broadcast(&buffer[..len], &client_addr);
            }

            server.remove_client(&client_addr);
        });
    }
}

pub fn run_client(server_ip: &str) {
    let mut stream = TcpStream::connect(server_ip).unwrap();
    if let Ok(s) = stream.local_addr() {
        println!("Your ip is: {}", s);
    };

    // Print the server's address
    match stream.peer_addr() {
        Ok(addr) => {
            println!("Connected to server at address: {}", addr);
        }
        Err(e) => {
            eprintln!("Failed to retrieve server address: {}", e);
        }
    }
    println!("To quit the chat, type /quit and press enter");

    // Spawn a thread to read messages from the server
    let mut server_stream = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut buffer = [0; 512];
        while let Ok(len) = server_stream.read(&mut buffer) {
            if len == 0 {
                break;
            }
            println!("{}", String::from_utf8_lossy(&buffer[..len]));
        }
    });

    loop {
        // Read the message from user input
        // print!("Enter a message to send to the server:");
        let mut message = String::new();
        std::io::stdin()
            .read_line(&mut message)
            .expect("Failed to read input");
        let message = message.trim(); // Remove trailing newline

        stream.write_all(message.as_bytes()).unwrap();
        // thread::sleep(Duration::from_secs(5));
        stream.flush().unwrap();

        if message == "/quit" {
            break;
        }
    }
}
