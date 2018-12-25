use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::{self, Arg, SubCommand};

use std::io::{Cursor, Read};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};
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
        let server_addr = server_addr.to_socket_addrs().unwrap().next().unwrap();

        let bind_addr = if server_addr.is_ipv4() {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)
        } else {
            SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), 0)
        };

        let bind_addr = bind_addr.to_socket_addrs().unwrap().next().unwrap();
        let socket = UdpSocket::bind(bind_addr).unwrap();

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
                let mut rdr = Cursor::new(&buf[..]);
                if let Ok(val) = rdr.read_u32::<LittleEndian>() {
                    let _ = tx.send((val, from));
                }
            }
            Err(e) => println!("recv_from: encountered IO error: {}", e),
        }
    });

    while let Ok((packet_count, from)) = rx.recv() {
        println!(
            "Received request for {} packets from {}",
            packet_count, from
        );

        let packet: [u8; PACKET_BYTES] = [0; PACKET_BYTES];
        for _ in 0..packet_count {
            let _ = server.socket.send_to(&packet, from);

            /*let ten_millis = std::time::Duration::from_nanos(1);
            let now = std::time::Instant::now();
            thread::sleep(ten_millis);*/
        }
    }

    server_thread.join().unwrap();
}

fn run_client(matches: &clap::ArgMatches) {
    let addr = format!(
        "{}:{}",
        matches.value_of("addr").unwrap_or("localhost"),
        PORT_NUMBER,
    );

    let packet_count: u32 = matches
        .value_of("packet-count")
        .unwrap_or("1")
        .parse()
        .unwrap();

    let client = Client::new(addr);

    let mut wtr = vec![];
    wtr.write_u32::<LittleEndian>(packet_count).unwrap();
    client.send(&wtr);

    let mut buf: [u8; PACKET_BYTES] = unsafe { std::mem::uninitialized() };

    for i in 0..packet_count {
        match client.socket.recv_from(&mut buf) {
            Ok((byte_count, _from)) => {
                println!(
                    "{}: Received {} bytes back from the server; [0]: {}",
                    i, byte_count, buf[0]
                );
            }
            Err(e) => println!("recv_from: encountered IO error: {}", e),
        }
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
        .arg(
            Arg::with_name("packet-count")
                .long("packet-count")
                .takes_value(true)
                .help("spam spam spam"),
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
