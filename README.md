# abut

Length-prefixed frames over any `Read` and `Write`.

To abut is to share a boundary. Two processes that abut, over a Unix socket,
a pipe or a TCP stream, need to agree where one message ends and the next
begins. This crate is that agreement and nothing more.

- **Framing.** A little-endian `u32` length, then the bytes. The reader has a
  maximum frame length, and when it refuses a frame, because it is too big or
  the caller's buffer is too small, it can drain the bytes so the stream is
  still aligned for the next call. That is what keeps a long-lived connection
  usable after one bad message.
- **Typed frames.** `frame::cbor` and `frame::postcard` (features `cbor` and
  `postcard`, both on by default) send and receive `serde` values, one value
  per frame. A payload that fails to decode does not misalign the stream.
- **A bounded frame ring.** `ring` (feature `ring-heapless`) is a
  single-producer single-consumer ring of whole frames with no allocation,
  for handing frames from a hot thread to a slower one that writes them out.
- **Stable error codes.** Every failure renders as `[ABUT0002] Buffer too
  small (need 12 bytes)`, with the underlying I/O or codec error as the source.

## Frames of bytes

```rust
use abut::{AbutCode, FramedReader, FramedWriter};
use std::io::Cursor;

let mut w = FramedWriter::new(Vec::new());
w.write_frame(b"hello").unwrap();
w.write_frame(b"a much longer frame").unwrap();
w.write_frame(b"world").unwrap();

let mut r = FramedReader::new(Cursor::new(w.into_inner()));
let mut buf = [0u8; 8];

let n = r.read_frame(&mut buf).unwrap();
assert_eq!(&buf[..n], b"hello");

// Too long for `buf`: refused, but drained, so the stream stays aligned.
let err = r.read_frame(&mut buf).unwrap_err();
assert!(matches!(err.code, AbutCode::BufferTooSmall { needed: 19 }));
assert_eq!(err.to_string(), "[ABUT0002] Buffer too small (need 19 bytes)");

let n = r.read_frame(&mut buf).unwrap();
assert_eq!(&buf[..n], b"world");
```

## Frames of values

```rust
use abut::frame::cbor::{FramedCborReader, FramedCborWriter};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Command {
    SetGain(u16),
    Status { active: bool, battery: u8 },
}

let mut w = FramedCborWriter::new(Vec::new());
w.send(&Command::SetGain(500)).unwrap();
w.send(&Command::Status { active: true, battery: 88 }).unwrap();

let mut r = FramedCborReader::new(Cursor::new(w.into_inner()));
assert_eq!(r.recv::<Command>().unwrap(), Command::SetGain(500));
assert_eq!(r.recv::<Command>().unwrap(), Command::Status { active: true, battery: 88 });
```

Swap `cbor` for `postcard` to get the same API with a more compact encoding.

## Handing frames between threads

With `--features ring-heapless`:

```rust,ignore
use abut::ring::{HeaplessRing, RingError};
use abut::FrameSink;

// Up to 63 frames of at most 32 bytes, stored in place.
let mut ring: HeaplessRing<32, 64> = HeaplessRing::new();
let (mut producer, mut consumer) = ring.split();

std::thread::scope(|s| {
    s.spawn(move || {
        for i in 0..1000u32 {
            let frame = format!("sample-{i}");
            // A full ring is back-pressure: wait for the consumer.
            while let Err(RingError::Full) = producer.send_frame(frame.as_bytes()) {
                std::thread::yield_now();
            }
        }
    });
    let mut got = 0;
    while got < 1000 {
        if consumer.pop().is_some() { got += 1; } else { std::thread::yield_now(); }
    }
});
```

`FrameSink` is implemented by both the ring producer and `FramedWriter`, so
the code that emits frames does not care which it holds.

## Reader configuration

| field | default | meaning |
|---|---|---|
| `max_frame_len` | 64 KiB | frames longer than this are refused with `FrameTooLarge` |
| `drain_on_small_buffer` | `true` | when the caller's buffer is too short, consume the frame so the next read starts at the next frame |
| `drain_oversize_up_to` | `0` (never) | consume a refused oversize frame if it is no longer than this, so the stream stays aligned |

Draining a frame the buffer cannot hold is always safe: its length was already
checked against the maximum. Draining an oversize frame means reading as many
bytes as the peer claimed, so it is off by default and bounded when on.

## Examples

- `cargo run --example unix_socket`: two threads exchanging typed CBOR frames
  over a Unix socket pair.
- `cargo run --example ring --features ring-heapless`: a producer thread
  feeding a consumer thread through the ring, drained to a framed writer.

## What is not here

Message schemas, request and response matching, sockets themselves. Bring
your own `Read` and `Write`.

## License

MIT OR Apache-2.0.
