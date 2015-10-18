extern crate mio;
use mio::{TryRead, TryWrite};
extern crate http_muncher;
extern crate sha1;
extern crate rustc_serialize;
use rustc_serialize::base64::ToBase64;

fn gen_key(key: &String) -> String {
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];

    m.update(key.as_bytes());
    m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());

    m.output(&mut buf);

    return buf.to_base64(rustc_serialize::base64::STANDARD);
}

struct HttpParser {
    current_key: Option<String>,
    headers: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, String>>>
}

impl http_muncher::ParserHandler for HttpParser {
    fn on_header_field(&mut self, s: &[u8]) -> bool {
        self.current_key = Some(std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, s: &[u8]) -> bool {
        self.headers.borrow_mut()
            .insert(self.current_key.clone().unwrap(),
                    std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_headers_complete(&mut self) -> bool {
        false
    }
}

#[derive(PartialEq)]
enum ClientState {
    AwaitingHandshake,
    HandshakeResponse,
    Connected
}

struct WebSocketClient {
    socket: mio::tcp::TcpStream,
    http_parser: http_muncher::Parser<HttpParser>,
    headers: std::rc::Rc<std::cell::RefCell<
        std::collections::HashMap<String, String>>>,
    interest: mio::EventSet,
    state: ClientState
}

impl WebSocketClient {
    fn read(&mut self) {
        loop {
            let mut buf = [0; 2048];
            match self.socket.try_read(&mut buf) {
                Err(e) => {
                    println!("Error while reading socket: {:?}", e);
                    return;
                },
                Ok(None) => {
                    // Socket buffer has got no more bytes.
                    break;
                },
                Ok(Some(len)) => {
                    self.http_parser.parse(&buf[0..len]);
                    if self.http_parser.is_upgrade() {
                        self.state = ClientState::HandshakeResponse;
                        self.interest.remove(mio::EventSet::readable());
                        self.interest.insert(mio::EventSet::writable());
                        break;
                    }
                }
            }
        }
    }

    fn write(&mut self) {
        let headers = self.headers.borrow();
        let response_key = gen_key(&headers.get("Sec-WebSocket-Key").unwrap());
        let response = std::fmt::format(format_args!(
                "HTTP/1.1 101 Switching Protocols\r\n\
                 Connection: Upgrade\r\n\
                 Sec-WebSocket-Accept: {}\r\n\
                 Upgrade: websocket\r\n\r\n", response_key));
        self.socket.try_write(response.as_bytes()).unwrap();
        self.state = ClientState::Connected;
        self.interest.remove(mio::EventSet::writable());
        self.interest.insert(mio::EventSet::readable());
    }

    fn new(socket: mio::tcp::TcpStream) -> WebSocketClient {
        let headers = std::rc::Rc::new(std::cell::RefCell::new(
                std::collections::HashMap::new()));

        WebSocketClient {
            socket: socket,
            headers: headers.clone(),
            http_parser: http_muncher::Parser::request(HttpParser {
                current_key: None,
                headers: headers.clone()
            }),
            interest: mio::EventSet::readable(),
            state: ClientState::AwaitingHandshake
        }
    }
}

struct WebSocketServer {
    socket: mio::tcp::TcpListener,
    clients: std::collections::HashMap<mio::Token, WebSocketClient>,
    token_counter: usize
}

const SERVER_TOKEN: mio::Token = mio::Token(0);

impl mio::Handler for WebSocketServer {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<WebSocketServer>,
             token: mio::Token, events: mio::EventSet)
    {
        if events.is_readable() {
            match token {
                SERVER_TOKEN => {
                    let client_socket = match self.socket.accept() {
                        Err(e) => {
                            println!("Accept error: {}", e);
                            return;
                        },
                        Ok(None) => unreachable!("Accept has returned 'None'"),
                        Ok(Some((sock, addr))) => sock
                    };

                    self.token_counter += 1;
                    let new_token = mio::Token(self.token_counter);

                    self.clients.insert(new_token, WebSocketClient::new(client_socket));
                    event_loop.register(&self.clients[&new_token].socket,
                                        new_token,
                                        mio::EventSet::readable(),
                                        mio::PollOpt::edge() | mio::PollOpt::oneshot()
                                        ).unwrap();
                },
                token => {
                    let mut client = self.clients.get_mut(&token).unwrap();
                    client.read();
                    event_loop.reregister(&client.socket,
                                        token,
                                        client.interest,
                                        mio::PollOpt::edge() | mio::PollOpt::oneshot()
                                        ).unwrap();
                }
            }
        }

        if events.is_writable() {
            let mut client = self.clients.get_mut(&token).unwrap();
            client.write();
            event_loop.reregister(&client.socket,
                                  token,
                                  client.interest,
                                  mio::PollOpt::edge() | mio::PollOpt::oneshot()
                                  ).unwrap();
        }
    }
}

fn main() {
    let mut event_loop = mio::EventLoop::new().unwrap();

    let address = "0.0.0.0:10000".parse::<std::net::SocketAddr>().unwrap();
    let server_socket = mio::tcp::TcpListener::bind(&address).unwrap();

    let mut server = WebSocketServer {
        token_counter: 1,
        clients: std::collections::HashMap::new(),
        socket: server_socket
    };

    event_loop.register(&server.socket,
               SERVER_TOKEN,
               mio::EventSet::readable(),
               mio::PollOpt::edge()).unwrap();

    event_loop.run(&mut server).unwrap();
}
