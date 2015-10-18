extern crate mio;

struct WebSocketServer {
    socket: mio::tcp::TcpListener,
    clients: std::collections::HashMap<mio::Token, mio::tcp::TcpStream>,
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

                self.clients.insert(new_token, client_socket);
                event_loop.register(&self.clients[&new_token],
                                    new_token,
                                    mio::EventSet::readable(),
                                    mio::PollOpt::edge() | mio::PollOpt::oneshot()
                                    ).unwrap();
            },
            _ => println!("something else")
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
