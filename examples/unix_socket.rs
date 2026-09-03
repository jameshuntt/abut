//! Two threads talk over a Unix socket pair with typed CBOR frames.
//!
//! Run: `cargo run --example unix_socket`

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use abut::frame::cbor::{FramedCborReader, FramedCborWriter};
    use serde::{Deserialize, Serialize};
    use std::os::unix::net::UnixStream;
    use std::thread;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Request {
        Ping(u32),
        Shutdown,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Reply {
        seq: u32,
        text: String,
    }

    let (client_side, server_side) = UnixStream::pair()?;

    // The server owns one end: reads requests, writes replies, stops on Shutdown.
    let server = thread::spawn(move || -> Result<u32, abut::AbutError> {
        let mut reader = FramedCborReader::new(server_side.try_clone().map_err(abut::AbutError::io)?);
        let mut writer = FramedCborWriter::new(server_side);
        let mut handled = 0;
        loop {
            match reader.recv::<Request>()? {
                Request::Ping(seq) => {
                    writer.send(&Reply { seq, text: format!("pong {seq}") })?;
                    handled += 1;
                }
                Request::Shutdown => return Ok(handled),
            }
        }
    });

    // The client owns the other end.
    let mut writer = FramedCborWriter::new(client_side.try_clone()?);
    let mut reader = FramedCborReader::new(client_side);
    for seq in 1..=3 {
        writer.send(&Request::Ping(seq))?;
        let reply: Reply = reader.recv()?;
        assert_eq!(reply, Reply { seq, text: format!("pong {seq}") });
        println!("client got {reply:?}");
    }
    writer.send(&Request::Shutdown)?;

    let handled = server.join().expect("server thread")?;
    println!("server handled {handled} pings");
    Ok(())
}

#[cfg(not(unix))]
fn main() {
    println!("this example needs a Unix socket pair");
}
