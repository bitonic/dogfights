#![feature(slicing_syntax)]
#![allow(unstable)]
#![warn(unused_results)]
extern crate "rustc-serialize" as rustc_serialize;
extern crate sdl2;
extern crate bincode;
#[macro_use] extern crate log;

extern crate conf;

use std::io::net::udp::UdpSocket;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{IoError, IoResult, IoErrorKind, BufWriter, BufReader};
use std::sync::{Arc, Mutex};
use std::ops::DerefMut;
use std::thread::{Thread};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::ptr;
use rustc_serialize::{Encodable, Decodable};

use conf::*;

// ---------------------------------------------------------------------
// Packet

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
pub struct Seq(u32);

impl Seq {
    #[inline]
    fn bump(&mut self) {
        self.0 += 1;
    }
}

impl Seq {
    // Returns if it's more recent and the difference between the two.
    // FIXME: actually wrap around
    #[inline]
    fn more_recent(x: Seq, y: Seq) -> Seq {
        if x.0 > y.0 { x } else { y }
    }
}

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct Local {
    /// Our local seq number
    seq: Seq,
    /// The last remote message we have acked
    ack: Seq,
}

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct Remote {
    /// Last message the remote ack'ed
    ack: Seq,
    /// The last time we received a message from remote
    received: u32,
}

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
enum MsgType {
    Ping,
    Pong,
    Normal,
}

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct Header {
    proto_id: u32,
    local: Local,
    msg_type: MsgType,
}

impl Header {
    fn new(local: Local, msg_type: MsgType) -> Header {
        Header{
            proto_id: PROTO_ID,
            local: local,
            msg_type: msg_type,
        }
    }
}

// ---------------------------------------------------------------------
// Lightweight connection

#[derive(Copy, Clone)]
struct Conn {
    local: Local,
    remote: Remote,
}

impl Conn {
    fn new() -> Conn {
        Conn{
            local: Local{
                seq: Seq(0),
                ack: Seq(0),
            },
            remote: Remote{
                ack: Seq(0),
                received: sdl2::get_ticks(),
            }
        }
    }

    fn tickle(&mut self, remote_local: &Local) {
        self.local.ack = Seq::more_recent(self.local.ack, remote_local.seq);
        self.remote.received = sdl2::get_ticks();
        self.remote.ack = Seq::more_recent(self.remote.ack, remote_local.ack);
    }
}

fn encode_and_send<T: Encodable>(conn: &mut Conn, sock: &mut UdpSocket, buf: &mut [u8], addr: SocketAddr, msg_type: MsgType, body: &T) -> IoResult<()> {
    let now = sdl2::get_ticks();
    if now - conn.remote.received > CONN_TIMEOUT {
        debug!("Connection {} timed out", addr);
        return Err(IoError{
            kind: IoErrorKind::Closed,
            desc: "network::encode_and_send: Connection timed out",
            detail: None,
        });
    }

    #[derive(RustcEncodable)]
    struct Packet<'a, T: 'a> {
        header: Header,
        body: &'a T,
    }

    conn.local.seq.bump();
    let packet = Packet{
        header: Header::new(conn.local, msg_type),
        body: body
    };
    let len = {
        let mut w = BufWriter::new(buf);
        try!(bincode::encode_into(&packet, &mut w));
        (try!(w.tell()) as usize)
    };
    try!(sock.send_to(buf.slice_to(len), addr));
    debug!("Message sent to {}", addr);
    Ok(())
}

fn send_ping(conn: &mut Conn, sock: &mut UdpSocket, addr: SocketAddr) -> IoResult<()> {
    let mut buf: [u8; 200] = [0; 200];
    encode_and_send(conn, sock, &mut buf, addr, MsgType::Ping, &())
}

fn send_pong(conn: &mut Conn, sock: &mut UdpSocket, addr: SocketAddr) -> IoResult<()> {
    let mut buf: [u8; 200] = [0; 200];
    encode_and_send(conn, sock, &mut buf, addr, MsgType::Pong, &())
}

fn recv_and_decode_1(sock: &mut UdpSocket, buf: &mut [u8]) -> IoResult<SocketAddr> {
    debug!("Blocking to receive");
    let (_, addr) = try!(sock.recv_from(buf));
    debug!("Received message from {}", addr);
    Ok(addr)
}

fn recv_and_decode_2<T: Decodable>(conn: &mut Conn, addr: SocketAddr, sock: &mut UdpSocket, buf: &mut [u8]) -> IoResult<Option<T>> {
    #[derive(RustcDecodable)]
    struct Packet<T> {
        header: Header,
        body: T,
    }

    let mut r = BufReader::new(buf);
    let packet: bincode::DecodingResult<Packet<T>> = bincode::decode_from(&mut r);
    match packet {
        Err(err) => {
            warn!("Error while decoding: {}, dropping", err);
            Ok(None)
        },
        Ok(packet) => {
            let proto_id = packet.header.proto_id;
            if proto_id != PROTO_ID {
                warn!("Mismatching proto-id, got {}, expecting {}", packet.header.proto_id, PROTO_ID);
                Ok(None)
            } else {
                conn.tickle(&packet.header.local);
                match packet.header.msg_type {
                    MsgType::Ping => {
                        try!(send_pong(conn, sock, addr));
                        Ok(None)
                    },
                    MsgType::Pong => Ok(None),
                    MsgType::Normal => Ok(Some(packet.body))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// Client

pub struct ClientHandle {
    connected_to: SocketAddr,
    socket: UdpSocket,
    conn: Arc<Mutex<Conn>>,
    buf: [u8; MAX_PACKET_SIZE],
}

impl Clone for ClientHandle {
    fn clone(&self) -> ClientHandle {
        unsafe {
            ClientHandle{
                connected_to: self.connected_to,
                socket: self.socket.clone(),
                conn: self.conn.clone(),
                buf: ptr::read(&self.buf),
            }
        }
    }
}

pub struct Client {
    handle: ClientHandle,
    ping_worker: Option<Sender<()>>,
}

impl Client {
    pub fn new<A: ToSocketAddr, B: ToSocketAddr>(connect_to: A, listen_on: B, ping: bool) -> IoResult<Client> {
        let connected_to = try!(connect_to.to_socket_addr());
        let sock = try!(UdpSocket::bind(listen_on));
        let conn = Arc::new(Mutex::new(Conn::new()));
        let tx = if ping {
            let (tx, rx) = channel();
            Client::ping_worker(&sock, &conn, connected_to, rx);
            Some(tx)
        } else {
            None
        };
        Ok(Client{
            handle: ClientHandle{
                connected_to: connected_to,
                socket: sock,
                conn: conn,
                buf: [0; MAX_PACKET_SIZE],
            },
            ping_worker: tx,
        })
    }

    pub fn handle(&self) -> ClientHandle {
        self.handle.clone()
    }

    fn ping_worker(sock: &UdpSocket, conn: &Arc<Mutex<Conn>>, addr: SocketAddr, close_signal: Receiver<()>) {
        let mut sock = sock.clone();
        let conn: Arc<Mutex<Conn>> = conn.clone();
        let _ = Thread::spawn(move || {
            loop {
                let close = close_signal.try_recv().is_ok();
                if close {
                    break;
                };

                // This block is crucial: we don't want to hold the lock
                // until the delay is done!
                {
                    let mut conn = conn.lock().unwrap();
                    match send_ping(conn.deref_mut(), &mut sock, addr) {
                        Ok(()) => (),
                        Err(err) => warn!("network::Client::ping_worker: got error {}", err),
                    };
                }
                sdl2::timer::delay(PING_INTERVAL as usize);
            }
        });
    }
}

impl ClientHandle {
    pub fn send<T: Encodable>(&mut self, body: &T) -> IoResult<()> {
        // TODO handle disconnections
        let mut conn = self.conn.lock().unwrap();
        encode_and_send(conn.deref_mut(), &mut self.socket, &mut self.buf, self.connected_to, MsgType::Normal, &body)
    }

    pub fn recv<T: Decodable>(&mut self) -> IoResult<T> {
        loop {
            let addr = try!(recv_and_decode_1(&mut self.socket, &mut self.buf));
            if addr == self.connected_to {
                let mut conn = self.conn.lock().unwrap();
                match try!(recv_and_decode_2(conn.deref_mut(), addr, &mut self.socket, &mut self.buf)) {
                    None => (),
                    Some(body) => return Ok(body)
                }
            } else {
                warn!("Got message from unknown sender {}, expected {}", addr, self.connected_to);
            }
        }
    }

    pub fn set_timeout(&mut self, ms: Option<u64>) {
        self.socket.set_timeout(ms)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        match self.ping_worker {
            None         => (),
            Some(ref tx) => tx.send(()).ok().unwrap(),
        }
    }
}

// ---------------------------------------------------------------------
// Server

#[derive(Clone)]
pub struct Server {
    socket: UdpSocket,
    clients: Arc<Mutex<HashMap<SocketAddr, Conn>>>,
}

impl Server {
    pub fn new<A: ToSocketAddr>(addr: A) -> IoResult<Server> {
        let sock = try!(UdpSocket::bind(addr));
        Ok(Server{
            socket: sock,
            clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn send<T : Encodable>(&mut self, addr: SocketAddr, body: &T) -> IoResult<()> {
        let mut clients = self.clients.lock().unwrap();
        match clients.entry(addr) {
            Entry::Vacant(_) => {
                error!("Sending to unknown client {}", addr);
                Err(IoError{
                    kind: IoErrorKind::NotConnected,
                    desc: "network::Server::send: unknown address",
                    detail: Some(format!("Address received: {}", addr))
                })
            },
            Entry::Occupied(mut entry) => {
                let mut buf = [0; MAX_PACKET_SIZE];
                match encode_and_send(entry.get_mut(), &mut self.socket, &mut buf, addr, MsgType::Normal, body) {
                    Err(err) => {
                        match err.kind {
                            IoErrorKind::Closed => {
                                debug!("Removing connection to {}", addr);
                                let _ = entry.remove();
                            },
                            _ => (),
                        };
                        Err(err)
                    },
                    Ok(()) => Ok(())
                }
            }
        }
    }

    pub fn recv<T: Decodable>(&mut self) -> IoResult<(SocketAddr, T)> {
        let mut buf = [0; MAX_PACKET_SIZE];
        loop {
            let addr = try!(recv_and_decode_1(&mut self.socket, &mut buf));
            let body = {
                let mut clients = self.clients.lock().unwrap();
                // Create new connection if needed
                match clients.entry(addr) {
                    // TODO is there a nice way to float the conn out?
                    // do I have to define a closure or another
                    // function?
                    Entry::Vacant(entry) => {
                        let conn = entry.insert(Conn::new());
                        try!(recv_and_decode_2(conn, addr, &mut self.socket, &mut buf))
                    },
                    Entry::Occupied(mut entry) => {
                        let conn = entry.get_mut();
                        try!(recv_and_decode_2(conn, addr, &mut self.socket, &mut buf))
                    }
                }
            };
            match body {
                None       => {},
                Some(body) => return Ok((addr, body))
            }
        }
    }

    pub fn active_conn(&self, addr: &SocketAddr) -> bool {
        let clients = self.clients.lock().unwrap();
        clients.get(addr).is_some()
    }

    #[cfg(test)]
    fn get_conn(&self, addr: &SocketAddr) -> Option<Conn> {
        let clients = self.clients.lock().unwrap();
        match clients.get(addr) {
            None       => None,
            Some(conn) => Some(*conn),
        }
    }
}

// ---------------------------------------------------------------------
// Multiple seqs

// #[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
// struct RemoteSeqs {
//     last: Seq,
//     // A bitfield that records whether the previous 32 messages were
//     // received
//     previous: u64,
// }

// impl RemoteSeqs {
//     fn update(&mut self, new: Seq) {
//         let (recent, diff) = more_recent(new, self.last);
//         let diff = diff as uint;
//         if recent {
//             self.last = new;
//             self.previous = (self.previous >> diff) | (1 >> diff-1);
//         } else {
//             if diff > 0 {
//                 self.previous = self.previous | 1 >> diff-1;
//             }
//         }
//     }
// }

// ---------------------------------------------------------------------
// Tests

#[test]
fn test() {
    let server_addr = "127.0.0.1:10000".to_socket_addr().ok().unwrap();
    let client_addr = "127.0.0.1:10001".to_socket_addr().ok().unwrap();
    let mut server = Server::new(server_addr).ok().unwrap();
    let mut client = Client::new(server_addr, client_addr, false).ok().unwrap();
    let mut client_handle = client.handle();

    let body: isize = 1234;
    client_handle.send(&body).ok().unwrap();
    {
        let conn = client_handle.conn.lock().unwrap();
        assert!(conn.local.seq == Seq(1));
        assert!(conn.local.ack == Seq(0));
        assert!(conn.remote.ack == Seq(0));
    }

    let (recv_addr, recv_body): (SocketAddr, isize) = server.recv().ok().unwrap();
    assert!(recv_body == body);
    assert!(recv_addr == client_addr);
    {
        let server_client_conn = server.get_conn(&client_addr).unwrap();
        assert!(server_client_conn.local.seq == Seq(0));
        assert!(server_client_conn.local.ack == Seq(1));
        assert!(server_client_conn.remote.ack == Seq(0));
    }

    let body: isize = 4321;
    server.send(client_addr, &body).ok().unwrap();
    {
        let server_client_conn = server.get_conn(&client_addr).unwrap();
        assert!(server_client_conn.local.ack == Seq(1));
        assert!(server_client_conn.local.seq == Seq(1));
        assert!(server_client_conn.remote.ack == Seq(0));
    }

    let recv_body: isize = client_handle.recv().ok().unwrap();
    assert!(recv_body == body);
    {
        let conn = client_handle.conn.lock().unwrap();
        assert!(conn.local.seq == Seq(1));
        assert!(conn.local.ack == Seq(1));
    }
}
