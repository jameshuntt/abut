//! The framing layer: round trips, limits, and the drain rules that keep a
//! stream aligned after a refused frame.

use std::error::Error;
use std::io::Cursor;

use abut::{AbutCode, AbutError, FrameSink, FrameSource, FramedReader, FramedWriter, ReaderConfig, LEN_PREFIX};

fn framed(frames: &[&[u8]]) -> Vec<u8> {
    let mut w = FramedWriter::new(Vec::new());
    for f in frames {
        w.write_frame(f).unwrap();
    }
    w.into_inner()
}

#[test]
fn the_wire_format_is_a_little_endian_length_then_the_bytes() {
    let bytes = framed(&[b"abc", b""]);
    assert_eq!(bytes, [3, 0, 0, 0, b'a', b'b', b'c', 0, 0, 0, 0]);
    assert_eq!(LEN_PREFIX, 4);
}

#[test]
fn round_trip_through_both_read_paths() {
    let bytes = framed(&[b"hello", b"", b"world"]);

    let mut r = FramedReader::new(Cursor::new(bytes.clone()));
    let mut buf = [0u8; 16];
    let n = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"hello");
    assert_eq!(r.read_frame(&mut buf).unwrap(), 0);
    let n = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"world");
    let eof = r.read_frame(&mut buf).unwrap_err();
    assert!(matches!(eof.code, AbutCode::Io));
    assert_eq!(eof.source().and_then(|s| s.downcast_ref::<std::io::Error>()).map(|e| e.kind()), Some(std::io::ErrorKind::UnexpectedEof));

    let mut r = FramedReader::new(Cursor::new(bytes));
    let mut v = vec![9u8; 3];
    r.recv_into(&mut v).unwrap();
    assert_eq!(v, b"hello");
    r.recv_into(&mut v).unwrap();
    assert!(v.is_empty(), "an empty frame clears the vec");
    r.recv_into(&mut v).unwrap();
    assert_eq!(v, b"world");
}

#[test]
fn a_frame_over_the_maximum_is_refused_and_not_drained_by_default() {
    let bytes = framed(&[b"0123456789", b"ok"]);
    let mut r = FramedReader::with_max(Cursor::new(bytes), 4);
    let mut v = Vec::new();
    let e = r.recv_into(&mut v).unwrap_err();
    assert!(matches!(e.code, AbutCode::FrameTooLarge { .. }), "{e}");
    assert_eq!(e.to_string(), "[ABUT0003] Frame too large: len 10 exceeds max 4");
    // nothing was consumed past the prefix, so the stream is now misaligned:
    // the payload bytes "0123" read as a length, and that is also too large
    let e2 = r.recv_into(&mut v).unwrap_err();
    assert!(matches!(e2.code, AbutCode::FrameTooLarge { .. }), "{e2}");
}

#[test]
fn an_oversize_frame_is_drained_when_the_bound_allows_it() {
    let bytes = framed(&[b"0123456789", b"ok"]);
    let cfg = ReaderConfig { max_frame_len: 4, drain_oversize_up_to: 64, ..Default::default() };
    let mut r = FramedReader::with_config(Cursor::new(bytes), cfg);
    let mut v = Vec::new();
    assert!(matches!(r.recv_into(&mut v).unwrap_err().code, AbutCode::FrameTooLarge { .. }));
    r.recv_into(&mut v).unwrap();
    assert_eq!(v, b"ok", "the stream stayed aligned");
}

#[test]
fn an_oversize_frame_beyond_the_drain_bound_is_not_drained() {
    let bytes = framed(&[b"0123456789", b"ok"]);
    let cfg = ReaderConfig { max_frame_len: 4, drain_oversize_up_to: 8, ..Default::default() };
    let mut r = FramedReader::with_config(Cursor::new(bytes), cfg);
    let mut v = Vec::new();
    assert!(matches!(r.recv_into(&mut v).unwrap_err().code, AbutCode::FrameTooLarge { .. }));
    assert!(r.recv_into(&mut v).is_err(), "misaligned, as configured");
}

#[test]
fn a_small_buffer_drains_by_default_so_the_next_frame_is_readable() {
    let bytes = framed(&[b"12345678", b"ok"]);
    let mut r = FramedReader::new(Cursor::new(bytes));
    let mut tiny = [0u8; 2];
    let e = r.read_frame(&mut tiny).unwrap_err();
    assert!(matches!(e.code, AbutCode::BufferTooSmall { needed: 8 }), "{e}");
    assert_eq!(e.to_string(), "[ABUT0002] Buffer too small (need 8 bytes)");
    let mut buf = [0u8; 8];
    let n = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"ok");
}

#[test]
fn a_small_buffer_without_draining_leaves_the_frame_in_the_stream() {
    let bytes = framed(&[b"12345678", b"ok"]);
    let cfg = ReaderConfig { drain_on_small_buffer: false, ..Default::default() };
    let mut r = FramedReader::with_config(Cursor::new(bytes), cfg);
    let mut tiny = [0u8; 2];
    assert!(matches!(r.read_frame(&mut tiny).unwrap_err().code, AbutCode::BufferTooSmall { needed: 8 }));
    // the eight payload bytes are still there; a caller that knows the length can read them raw
    let mut raw = [0u8; 8];
    std::io::Read::read_exact(r.inner_mut(), &mut raw).unwrap();
    assert_eq!(&raw, b"12345678");
    let mut buf = [0u8; 8];
    let n = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"ok");
}

#[test]
fn a_truncated_prefix_or_body_is_an_io_error() {
    let mut r = FramedReader::new(Cursor::new(vec![5u8, 0]));
    let e = r.recv_into(&mut Vec::new()).unwrap_err();
    assert!(matches!(e.code, AbutCode::Io), "{e}");
    assert!(e.to_string().starts_with("[ABUT0001] I/O failure"), "{e}");

    let mut r = FramedReader::new(Cursor::new(vec![5u8, 0, 0, 0, b'a', b'b']));
    let e = r.recv_into(&mut Vec::new()).unwrap_err();
    assert!(matches!(e.code, AbutCode::Io), "{e}");
}

#[test]
fn the_traits_let_frames_flow_through_any_sink_and_source() {
    fn pump<S: FrameSink<Error = AbutError>>(sink: &mut S) {
        sink.send_frame(b"one").unwrap();
        sink.send_frame(b"two").unwrap();
    }
    fn count<S: FrameSource<Error = AbutError>>(src: &mut S) -> usize {
        let mut buf = [0u8; 8];
        let mut n = 0;
        while src.recv_frame(&mut buf).is_ok() {
            n += 1;
        }
        n
    }
    let mut w = FramedWriter::new(Vec::new());
    pump(&mut w);
    w.send_bytes(b"three").unwrap();
    w.flush().unwrap();
    assert_eq!(w.get_ref().len(), 3 * LEN_PREFIX + 3 + 3 + 5);
    let mut r = FramedReader::new(Cursor::new(w.into_inner()));
    assert_eq!(count(&mut r), 3);
    assert_eq!(r.max_frame_len(), abut::DEFAULT_MAX_FRAME_LEN);
    assert_eq!(r.config(), ReaderConfig::default());
}

#[test]
fn error_values_carry_code_context_and_source() {
    let e = AbutError::frame_too_large(10, 4);
    assert!(e.source().is_none());
    assert!(matches!(e.code, AbutCode::FrameTooLarge { len: 10, max: 4 }));
    assert_eq!(e.to_string(), "[ABUT0003] Frame too large: len 10 exceeds max 4");

    let io = AbutError::from(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "gone"));
    assert!(matches!(io.code, AbutCode::Io));
    assert!(io.source().is_some());
    assert!(io.to_string().contains("gone"));

    let small: AbutError = abut::BufferTooSmall { needed: 3 }.into();
    assert!(matches!(small.code, AbutCode::BufferTooSmall { needed: 3 }));
    assert_eq!(abut::BufferTooSmall { needed: 3 }.to_string(), "buffer too small (need 3 bytes)");
}
