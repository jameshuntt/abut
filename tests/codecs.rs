//! Typed frames: CBOR and postcard round trips, and recovery after a bad payload.

use std::error::Error;
use std::io::Cursor;

use abut::{AbutCode, FramedReader, FramedWriter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Command {
    Reboot,
    SetGain(u16),
    Status { active: bool, battery: u8 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Sample {
    id: u32,
    label: String,
    values: Vec<f32>,
}

#[cfg(feature = "cbor")]
mod cbor {
    use super::*;
    use abut::frame::cbor::{FramedCborReader, FramedCborWriter};

    #[test]
    fn values_round_trip_in_order_and_across_types() {
        let mut w = FramedCborWriter::new(Vec::new());
        let sample = Sample { id: 42, label: "alpha".into(), values: vec![1.0, 2.5, 3.25] };
        w.send(&sample).unwrap();
        w.send(&Command::SetGain(500)).unwrap();
        w.send(&"plain string").unwrap();
        w.send(&12345u64).unwrap();
        w.flush().unwrap();

        let mut r = FramedCborReader::new(Cursor::new(w.into_inner()));
        assert_eq!(r.recv::<Sample>().unwrap(), sample);
        assert_eq!(r.recv::<Command>().unwrap(), Command::SetGain(500));
        assert_eq!(r.recv::<String>().unwrap(), "plain string");
        assert_eq!(r.recv::<u64>().unwrap(), 12345);
        assert!(matches!(r.recv::<u64>().unwrap_err().code, AbutCode::Io), "end of stream");
    }

    #[test]
    fn a_frame_that_is_not_valid_cbor_fails_without_misaligning_the_stream() {
        let mut raw = FramedWriter::new(Vec::new());
        raw.write_frame(&[0xff, 0xff, 0xff]).unwrap();
        let mut w = FramedCborWriter::new(raw.into_inner());
        w.send(&"after the garbage").unwrap();

        let mut r = FramedCborReader::new(Cursor::new(w.into_inner()));
        let e = r.recv::<String>().unwrap_err();
        assert!(matches!(e.code, AbutCode::CborDecode), "{e}");
        assert!(e.source().is_some());
        assert!(e.to_string().starts_with("[ABUT0021] CBOR decode failed"), "{e}");
        assert_eq!(r.recv::<String>().unwrap(), "after the garbage");
    }

    #[test]
    fn a_wrong_type_is_a_decode_error_not_a_panic() {
        let mut w = FramedCborWriter::new(Vec::new());
        w.send(&"text").unwrap();
        let mut r = FramedCborReader::new(Cursor::new(w.into_inner()));
        assert!(matches!(r.recv::<u32>().unwrap_err().code, AbutCode::CborDecode));
    }

    #[test]
    fn the_frame_limit_applies_before_decoding() {
        let mut w = FramedCborWriter::new(Vec::new());
        w.send(&"a string longer than the limit").unwrap();
        let mut r = FramedCborReader::with_inner(FramedReader::with_max(Cursor::new(w.into_inner()), 2));
        assert!(matches!(r.recv::<String>().unwrap_err().code, AbutCode::FrameTooLarge));
        assert_eq!(r.inner_mut().max_frame_len(), 2);
    }
}

#[cfg(feature = "postcard")]
mod postcard {
    use super::*;
    use abut::frame::postcard::{FramedPostcardReader, FramedPostcardWriter};

    #[test]
    fn values_round_trip_in_order() {
        let mut w = FramedPostcardWriter::new(Vec::new());
        w.send(&Command::Reboot).unwrap();
        w.send(&Command::SetGain(500)).unwrap();
        w.send(&Command::Status { active: true, battery: 88 }).unwrap();
        w.flush().unwrap();
        let mut r = FramedPostcardReader::new(Cursor::new(w.into_inner()));
        assert_eq!(r.recv::<Command>().unwrap(), Command::Reboot);
        assert_eq!(r.recv::<Command>().unwrap(), Command::SetGain(500));
        assert_eq!(r.recv::<Command>().unwrap(), Command::Status { active: true, battery: 88 });
    }

    #[test]
    fn postcard_frames_are_compact() {
        let mut w = FramedPostcardWriter::new(Vec::new());
        w.send(&Command::SetGain(1)).unwrap();
        // 4 bytes of prefix, then a one-byte tag and a one-byte varint
        assert_eq!(w.into_inner().len(), 6);
    }

    #[test]
    fn a_bad_payload_is_a_decode_error_and_the_next_frame_still_reads() {
        let mut raw = FramedWriter::new(Vec::new());
        raw.write_frame(&[0xff, 0xff, 0xff]).unwrap();
        let mut w = FramedPostcardWriter::new(raw.into_inner());
        w.send(&"fine").unwrap();
        let mut r = FramedPostcardReader::new(Cursor::new(w.into_inner()));
        let e = r.recv::<String>().unwrap_err();
        assert!(matches!(e.code, AbutCode::PostcardDecode), "{e}");
        assert!(e.to_string().starts_with("[ABUT0011] Postcard decode failed"), "{e}");
        assert_eq!(r.recv::<String>().unwrap(), "fine");
    }

    #[test]
    fn the_frame_limit_applies_before_decoding() {
        let mut w = FramedPostcardWriter::new(Vec::new());
        w.send(&"a string longer than the limit").unwrap();
        let mut r = FramedPostcardReader::with_inner(FramedReader::with_max(Cursor::new(w.into_inner()), 2));
        assert!(matches!(r.recv::<String>().unwrap_err().code, AbutCode::FrameTooLarge));
    }
}
