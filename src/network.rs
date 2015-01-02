extern crate "rustc-serialize" as rustc_serialize;
extern crate bincode;

use std::io::net::udp::UdpSocket;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::collections::HashMap;
use std::io::{MemWriter, MemReader, IoError, IoResult, IoErrorKind};
use rustc_serialize::{Encodable, Decodable};
use bincode::{EncoderWriter, DecoderReader};

const PROTO_ID: u32 = 0xD05F1575;
const MAX_PACKET_SIZE: uint = 1400;

// ---------------------------------------------------------------------

#[deriving(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct Packet<A> {
    header: Header,
    body: A,
}

#[deriving(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
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

#[deriving(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct ConnInfo {
    local_seq: Seq,
    remote_seq: Seq,
}

#[deriving(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct Header {
    proto_id: u32,
    info: ConnInfo,
}

impl Header {
    fn new(info: ConnInfo) -> Header {
        Header{
            proto_id: PROTO_ID,
            info: info,
        }
    }
}

pub struct Client {
    connected_to: SocketAddr,
    socket: UdpSocket,
    info: ConnInfo,
}

// FIXME: we shouldn't allocate the buffers on the heap, but it's a mess
// otherwise with the lifetime pars.
impl Client {
    pub fn new<A: ToSocketAddr, B: ToSocketAddr>(connect_to: A, listen_on: B) -> IoResult<Client> {
        let connected_to = try!(connect_to.to_socket_addr());
        let sock = try!(UdpSocket::bind(listen_on));
        Ok(Client{
            info: ConnInfo{
                local_seq: Seq(0),
                remote_seq: Seq(0),
            },
            socket: sock,
            connected_to: connected_to,
        })
    }

    pub fn send<'a, T: Encodable<EncoderWriter<'a, MemWriter>, IoError>>(&mut self, body: &T) -> IoResult<()> {
        self.info.local_seq.bump();
        let packet = Packet {
            header: Header::new(self.info),
            body: body,
        };
        encode_and_send(&mut self.socket, self.connected_to, &packet)
    }

    pub fn recv<'a, T: Decodable<DecoderReader<'a, MemReader>, IoError>>(&mut self) -> IoResult<IoResult<T>> {
        let (addr, packet): (SocketAddr, IoResult<Packet<T>>) = try!(recv_and_decode(&mut self.socket));
        match packet {
            Err(err) =>
                Ok(Err(err)),
            Ok(packet) => {
                let header = packet.header;
                match check_proto_id(&header, "Client.recv: wrong proto id") {
                    Err(err) =>
                        Ok(Err(err)),
                    Ok(()) => {
                        if addr != self.connected_to {
                            other_io_error("Client.recv: got message from unknown sender")
                        } else {
                            self.info.remote_seq = Seq::more_recent(header.info.local_seq, self.info.remote_seq);
                            Ok(Ok(packet.body))
                        }
                    }
                }
            }
        }
    }
}

pub struct Server {
    socket: UdpSocket,
    clients: HashMap<SocketAddr, ServerConn>,
}

struct ServerConn {
    // The last remote_seq received from the client.  This tells us
    // what's the last message we know the client received.
    info: ConnInfo,
    last_remote_seq: Seq,
}

impl Server {
    pub fn new<A: ToSocketAddr>(addr: A) -> IoResult<Server> {
        let sock = try!(UdpSocket::bind(addr));
        Ok(Server{
            socket: sock,
            clients: HashMap::new()
        })
    }

    pub fn send<'a, T: Encodable<EncoderWriter<'a, MemWriter>, IoError>>(&mut self, addr: SocketAddr, body: &T) -> IoResult<()> {
        match self.clients.get_mut(&addr) {
            None =>
                other_io_error("Server.send: Sending to unknown client"),
            Some(conn) => {
                conn.info.local_seq.bump();
                let packet = Packet{
                    header: Header::new(conn.info),
                    body: body
                };
                encode_and_send(&mut self.socket, addr, &packet)
            }
        }
    }

    pub fn recv<'a, T: Decodable<DecoderReader<'a, MemReader>, IoError>>(&mut self) -> IoResult<(SocketAddr, IoResult<T>)> {
        let (addr, packet): (SocketAddr, IoResult<Packet<T>>) = try!(recv_and_decode(&mut self.socket));
        match packet {
            Err(err) =>
                Ok((addr, Err(err))),
            Ok(packet) => {
                match check_proto_id(&packet.header, "Server.recv: wrong proto id") {
                    Err(err) =>
                        Ok((addr, Err(err))),
                    Ok(()) => {
                        let conn = match self.clients.get(&addr) {
                            None => ServerConn{
                                info: ConnInfo{
                                    local_seq: Seq(0),
                                    remote_seq: packet.header.info.local_seq,
                                },
                                last_remote_seq: packet.header.info.remote_seq,
                            },
                            Some(conn) =>
                                ServerConn{
                                    info: ConnInfo{
                                        remote_seq: Seq::more_recent(packet.header.info.local_seq, conn.info.remote_seq),
                                        local_seq: conn.info.local_seq,
                                    },
                                    last_remote_seq: Seq::more_recent(packet.header.info.remote_seq, conn.last_remote_seq),
                                }
                        };
                        let _ = self.clients.insert(addr, conn);
                        Ok((addr, Ok(packet.body)))
                    }
                        }
                }
            }
        }
}

// ---------------------------------------------------------------------
// Utils

fn encode_and_send<'a, T: Encodable<EncoderWriter<'a, MemWriter>, IoError>>(sock: &mut UdpSocket, addr: SocketAddr, body: T) -> IoResult<()> {
    let data = try!(bincode::encode(&body));
    if data.len() > MAX_PACKET_SIZE {
        other_io_error("encode_and_send: packet too large")
    } else {
        sock.send_to(data.as_slice(), addr)
    }
}

fn recv_and_decode<'a, T: Decodable<DecoderReader<'a, MemReader>, IoError>>(sock: &mut UdpSocket) -> IoResult<(SocketAddr, IoResult<T>)> {
    let mut buf = [0, ..MAX_PACKET_SIZE];
    let (len, addr) = try!(sock.recv_from(&mut buf));
    let mut data = Vec::with_capacity(len);
    data.push_all(&buf);
    Ok((addr, bincode::decode(data)))
}

fn other_io_error<T>(msg: &'static str) -> IoResult<T> {
    Err(IoError {
        kind: IoErrorKind::OtherIoError,
        desc: msg,
        detail: None,
    })
}

fn check_proto_id(header: &Header, msg: &'static str) -> IoResult<()> {
    if header.proto_id != PROTO_ID {
        other_io_error(msg)
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Multiple seqs

// #[deriving(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
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
    let mut client = Client::new(server_addr, client_addr).ok().unwrap();

    let body: int = 1234;
    client.send(&body);
    assert!(client.info.local_seq == Seq(1));

    let (recv_addr, recv_body): (SocketAddr, int) = server.recv().ok().unwrap();
    assert!(recv_body == body);
    assert!(recv_addr == client_addr);
    {
        let server_client_conn = server.clients.get(&client_addr).unwrap();
        assert!(server_client_conn.info.remote_seq == Seq(1));
        assert!(server_client_conn.info.local_seq == Seq(0));
        assert!(server_client_conn.last_remote_seq == Seq(0));
    }

    let body: int = 4321;
    server.send(client_addr, &body);
    {
        let server_client_conn = server.clients.get(&client_addr).unwrap();
        assert!(server_client_conn.info.remote_seq == Seq(1));
        assert!(server_client_conn.info.local_seq == Seq(1));
        assert!(server_client_conn.last_remote_seq == Seq(0));
    }

    let recv_body: int = client.recv().ok().unwrap();
    assert!(recv_body == body);
    assert!(client.info.local_seq == Seq(1));
    assert!(client.info.remote_seq == Seq(1));
}
