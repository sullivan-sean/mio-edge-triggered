use std::io::{self, Read, Write};
use std::net::{TcpListener, SocketAddr};
use std::thread;
use std::time::Duration;

use mio::net::TcpStream;
use mio::{Interest, Token};
use mio::event::{Event, Events};
use mio::Poll;

const DATA1: &[u8] = b"Hello world!";
const ID1: Token = Token(0);

#[allow(dead_code)]
fn assert_would_block<T>(result: io::Result<T>) {
    match result {
        Ok(_) => panic!("unexpected OK result, expected a `WouldBlock` error"),
        Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {}
        Err(err) => panic!("unexpected error result: {}", err),
    }
}

pub fn expect_events<F>(poll: &mut Poll, events: &mut Events, f: F)
where F: Fn(&Event) -> bool {
    // Poll a few times in case there are other events that come first.
    for _ in 0..3 {
        poll.poll(events, Some(Duration::from_millis(500))).expect("unable to poll");
        for event in events.iter() {
            if f(event) {
                return;
            }
        }
    }

    panic!("Did not receive any matching events");
}

fn main() {
    let mut poll = Poll::new().expect("unable to create Poll instance");
    let mut events = Events::with_capacity(16);

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = TcpListener::bind(addr).unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = thread::spawn(move || {
        let mut buf = [0; 128];
        let (mut stream, _) = listener.accept().unwrap();

        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    stream.write(&buf[..n]).unwrap();
                }
                Err(ref err) => {
                    if err.kind() == io::ErrorKind::ConnectionReset {
                        break;
                    }
                    panic!("error reading");
                }
            }
        }
    });

    let mut stream = TcpStream::connect(addr).unwrap();

    poll.registry()
        .register(&mut stream, ID1, Interest::WRITABLE.add(Interest::READABLE))
        .expect("unable to register TCP stream");

    expect_events(&mut poll, &mut events, |e| e.token() == ID1 && e.is_writable());

    let mut buf = [0; 16];
    stream.write(DATA1).unwrap();
    stream.flush().unwrap();

    expect_events(&mut poll, &mut events, |e| e.token() == ID1 && e.is_readable());

    stream.read(&mut buf).unwrap();

    assert!(stream.take_error().unwrap().is_none());

    // Program panics if this is commented, exits cleanly if it's here.
    // assert_would_block(stream.read(&mut buf));

    // Check write, then read, but don't try another "would block" read before writing.
    stream.write(DATA1).unwrap();
    stream.flush().unwrap();
    expect_events(&mut poll, &mut events, |e| e.token() == ID1 && e.is_readable());

    // Close the connection to allow the listener to shutdown.
    drop(stream);
    handle.join().expect("unable to join thread");
}
