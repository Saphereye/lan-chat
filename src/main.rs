use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

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
            for (client, _) in &*clients {
                let leave_message = format!("{} has left the chat.", addr);
                let mut client = client.clone();
                let _ = client.write_all(leave_message.as_bytes());
            }
        }
        println!("{} has left the chat.", addr);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: lan-chat <server/client> <server_ip>");
        return;
    }

    let server_ip = &args[2];

    match args[1].as_str() {
        "server" => run_server(server_ip),
        "client" => run_client(server_ip),
        _ => println!("Invalid argument"),
    }
}

fn run_server(server_ip: &str) {
    println!("Initializing server");
    let server = Server::new();

    let listener = TcpListener::bind(server_ip).unwrap();
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
                match String::from_utf8_lossy(&buffer).as_ref() {
                    "/quit" => {
                        server.remove_client(&client_addr);
                    }
                    _ => {}
                }
                // println!(
                //     "Got this in the buffer yo: {}",
                //     String::from_utf8_lossy(&buffer)
                // );
                server.broadcast(&buffer[..len], &client_addr);
                // Send a response back to the client
                // stream.write_all(b"Message received by server").unwrap();
            }

            server.remove_client(&client_addr);
        });
    }
}

fn run_client(server_ip: &str) {
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

        stream.write(message.as_bytes()).unwrap();
        // thread::sleep(Duration::from_secs(5));
        stream.flush().unwrap();

        match message {
            "/quit" => {
                break;
            }
            _ => {}
        }
    }
}
