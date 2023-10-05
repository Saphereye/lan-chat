extern crate clap;
use clap::{App, Arg};

extern crate if_addrs;
use if_addrs::get_if_addrs;

mod chat_logic;
mod user_interface;

fn main() {
    match user_interface::create_gui() {
        Ok(()) => (),
        Err(e) => eprintln!("{}", e)
    };
    // let matches = App::new("Lan Chat")
    //     .version("1.0")
    //     .author("Saphereye")
    //     .about("A LAN chat application")
    //     .subcommand(
    //         App::new("server").about("Run as server").arg(
    //             Arg::with_name("server-ip")
    //                 .long("server-ip")
    //                 .takes_value(true),
    //         ),
    //     )
    //     .subcommand(
    //         App::new("client").about("Run as server").arg(
    //             Arg::with_name("server-ip")
    //                 .long("server-ip")
    //                 .takes_value(true),
    //         ),
    //     )
    //     .subcommand(App::new("get-ip").about("Get your local IP address"))
    //     .get_matches();

    // if let Some(server_matches) = matches.subcommand_matches("server") {
    //     // User wants to start the server
    //     let server_ip = server_matches.value_of("server-ip").unwrap_or("127.0.0.1");
    //     chat_logic::run_server(server_ip);
    // } else if let Some(server_matches) = matches.subcommand_matches("client") {
    //     // User wants to start the server
    //     let server_ip = server_matches.value_of("server-ip").unwrap_or("127.0.0.1");
    //     chat_logic::run_client(server_ip);
    // } else if matches.subcommand_matches("get-ip").is_some() {
    //     // User wants to get their local IP address
    //     if let Ok(interfaces) = get_if_addrs() {
    //         println!("{:#?}", interfaces);
    //     } else {
    //         println!("Failed to retrieve network interface information.");
    //     }
    // } else {
    //     println!("Usage: lan-chat <server/get-ip> or lan-chat get-ip");
    // }
}
