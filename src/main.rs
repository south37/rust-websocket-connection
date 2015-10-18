extern crate mio;

struct WebSocketServer;
impl mio::Handler for WebSocketServer {
  type Timeout = usize;
  type Message = ();
}

fn main() {
  let mut event_loop = mio::EventLoop::new().unwrap();

  let address = "0.0.0.0:10000".parse::<std::net::SocketAddr>().unwrap();
  let server_socket = mio::tcp::TcpListener::bind(&address).unwrap();

  event_loop.register(&server_socket,
             mio::Token(0),
             mio::EventSet::readable(),
             mio::PollOpt::edge()).unwrap();

  let mut handler = WebSocketServer;
  event_loop.run(&mut handler).unwrap();
}
