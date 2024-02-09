extern crate clap;
use clap::{App, Arg};
mod networking;
use networking::*;

fn main() {
    let matches = App::new("Lan Chat")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Saphereye")
        .about("A LAN chat application")
        .subcommand(
            App::new("server").about("Run as server").arg(
                Arg::with_name("server-ip")
                    .long("server-ip")
                    .takes_value(false),
            ),
        )
        .subcommand(
            App::new("client").about("Run as server").arg(
                Arg::with_name("server-ip")
                    .long("server-ip")
                    .takes_value(true),
            ),
        )
        .subcommand(App::new("get-ip").about("Get your local IP address"))
        .get_matches();

    if let Some(server_matches) = matches.subcommand_matches("server") {
        // User wants to start the server
        let server_ip = match server_matches.value_of("server-ip") {
            Some(ip) => ip.to_string(),
            None => match get_local_ip() {
                Ok(ip) => ip,
                Err(e) => {
                    eprintln!("Failed to retrieve local IP address: {}", e);
                    return;
                }
            },
        };
        run_server(&server_ip);
    } else if let Some(server_matches) = matches.subcommand_matches("client") {
        // User wants to start the server
        let server_ip = match server_matches.value_of("server-ip") {
            Some(ip) => ip.to_string(),
            None => {
                eprintln!("Server IP address is required.");
                return;
            }
        };
        run_client(&server_ip);
    } else if matches.subcommand_matches("get-ip").is_some() {
        // User wants to get their local IP address
        match get_local_ip() {
            Ok(ip) => println!("Your local IP address is: {}", ip),
            Err(e) => eprintln!("Failed to retrieve local IP address: {}", e),
        }
    } else {
        println!("Usage: lan-chat <server/get-ip> or lan-chat get-ip");
    }
}
