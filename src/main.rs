use clap::{self, Arg, SubCommand};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::channel;
use std::thread;

const PORT_NUMBER: u32 = 7878;
const PACKET_BYTES: usize = 1100;

struct Server {
    socket: UdpSocket,
}

impl Server {
    fn new(addr: &String) -> Self {
        let socket = UdpSocket::bind(addr).unwrap();

        Self { socket }
    }
}

struct Client {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl Client {
    fn new<A: ToSocketAddrs>(server_addr: A) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        /*socket
        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
        .unwrap();*/

        let server_addr = server_addr.to_socket_addrs().unwrap().next().unwrap();
        println!("Connecting to {}", server_addr);

        Self {
            socket,
            server_addr,
        }
    }

    fn send(&self, bytes: &[u8]) {
        self.socket.send_to(bytes, self.server_addr).unwrap();
    }
}

fn run_server(matches: &clap::ArgMatches) {
    let addr = format!(
        "{}:{}",
        matches.value_of("bind").unwrap_or("localhost"),
        PORT_NUMBER,
    );

    let server = Server::new(&addr);

    println!("Server ready");

    let (tx, rx) = channel();

    let server_socket = server.socket.try_clone().unwrap();

    let server_thread = thread::spawn(move || loop {
        let mut buf: [u8; PACKET_BYTES] = unsafe { std::mem::uninitialized() };

        match server_socket.recv_from(&mut buf) {
            Ok((byte_count, from)) => {
                let _ = tx.send((byte_count, from));
            }
            Err(e) => println!("recv_from: encountered IO error: {}", e),
        }
    });

    while let Ok((byte_count, from)) = rx.recv() {
        println!("Received {} bytes from {}", byte_count, from);
        let _ = server.socket.send_to(&[byte_count as u8], from);
    }

    server_thread.join().unwrap();
}

fn run_client(matches: &clap::ArgMatches) {
    let addr = format!(
        "{}:{}",
        matches.value_of("addr").unwrap_or("localhost"),
        PORT_NUMBER,
    );

    let client = Client::new(addr);
    client.send(&[123, 66, 6, 1, 1, 2, 3, 5, 8, 13]);

    let mut buf: [u8; PACKET_BYTES] = unsafe { std::mem::uninitialized() };

    match client.socket.recv_from(&mut buf) {
        Ok((byte_count, _from)) => {
            println!(
                "Received {} bytes back from the server; [0]: {}",
                byte_count, buf[0]
            );
        }
        Err(e) => println!("recv_from: encountered IO error: {}", e),
    }
}

fn main() {
    let matches = clap::App::new("nettest")
        .version("0.1.0")
        .arg(
            Arg::with_name("addr")
                .long("addr")
                .takes_value(true)
                .help("address to connect to"),
        )
        .subcommand(
            SubCommand::with_name("serve").about("runs a server").arg(
                Arg::with_name("bind")
                    .long("bind")
                    .takes_value(true)
                    .help("address to listen on"),
            ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("serve") {
        run_server(matches);
    } else {
        run_client(&matches);
    }
}
