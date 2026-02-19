// // #![cfg(feature = "std")]
// 
// use abut::frame::{FramedReader, FramedWriter};
// 
// #[test]
// fn framed_roundtrip() {
//     let mut wbuf = Vec::new();
//     {
//         let mut w = FramedWriter::new(&mut wbuf);
//         w.send_bytes(b"abc").unwrap();
//         w.send_bytes(b"defgh").unwrap();
//     }
// 
//     let mut r = FramedReader::new(std::io::Cursor::new(wbuf));
//     let mut buf = Vec::new();
// 
//     r.recv_into(&mut buf).unwrap();
//     assert_eq!(&buf, b"abc");
// 
//     r.recv_into(&mut buf).unwrap();
//     assert_eq!(&buf, b"defgh");
// }
// 
// 
use std::io::Cursor;

use abut::{ReaderConfig, frame::{FramedReader, FramedWriter}};

#[test]
fn roundtrip_one_frame() {
    let mut io = Cursor::new(Vec::<u8>::new());

    {
        let mut w = FramedWriter::new(&mut io);
        w.write_frame(b"hello").unwrap();
        w.write_frame(b"world").unwrap();
    }

    io.set_position(0);

    let mut r = FramedReader::new(&mut io);

    let mut buf = vec![0u8; 16];
    let n1 = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n1], b"hello");

    let n2 = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n2], b"world");
}

#[test]
fn buffer_too_small_drains_by_default() {
    let mut io = Cursor::new(Vec::<u8>::new());

    {
        let mut w = FramedWriter::new(&mut io);
        w.write_frame(b"12345678").unwrap();
        w.write_frame(b"ok").unwrap();
    }

    io.set_position(0);
    let mut r = FramedReader::new(&mut io);

    let mut tiny = [0u8; 2];
    let e = r.read_frame(&mut tiny).unwrap_err();
    assert!(format!("{e}").contains("Buffer too small"));

    // because we drained the first frame, we can read the next one
    let mut buf = [0u8; 8];
    let n = r.read_frame(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"ok");
}

#[test]
fn oversize_does_not_drain_by_default() {
    let cfg = ReaderConfig { max_frame_len: 2, drain_on_small_buffer: false, ..Default::default() };
    let mut io = Cursor::new(vec![
        0x05, 0x00, 0x00, 0x00, // len = 5
        b'a', b'b', b'c', b'd', b'e'
    ]);

    let mut r = FramedReader::with_config(&mut io, cfg);
    let mut buf = [0u8; 8];
    let e = r.read_frame(&mut buf).unwrap_err();
    assert!(format!("{e}").contains("Frame too large"));
}
