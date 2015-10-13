extern crate mio;
use mio::*;
use mio::tcp::*;

struct WebSocketServer;
impl Handler for WebSocketServer {
  type Timeout = usize;
  type Message = ();
}

fn main() {
  let mut event_loop = EventLoop::new().unwrap();

  let address = "0.0.0.0:10000".parse().unwrap();
  let server_socket = TcpListener::bind(&address).unwrap();

  event_loop.register(&server_socket,
             Token(0),
             EventSet::readable(),
             PollOpt::edge()).unwrap();

  let mut handler = WebSocketServer;
  event_loop.run(&mut handler).unwrap();
}
