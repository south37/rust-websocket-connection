extern crate mio;
use mio::TryRead;
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

struct HttpParser;
impl http_muncher::ParserHandler for HttpParser { }

struct WebSocketClient {
    socket: mio::tcp::TcpStream,
    http_parser: http_muncher::Parser<HttpParser>
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
                        // something ....
                        break;
                    }
                }
            }
        }
    }

    fn new(socket: mio::tcp::TcpStream) -> WebSocketClient {
        WebSocketClient {
            socket: socket,
            http_parser: http_muncher::Parser::request(HttpParser)
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
                                    mio::EventSet::readable(),
                                    mio::PollOpt::edge() | mio::PollOpt::oneshot()
                                    ).unwrap();
            }
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
