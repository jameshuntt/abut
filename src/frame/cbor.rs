#![cfg(feature = "cbor")]

use crate::{
    AbutError,
    frame::{FramedReader, FramedWriter}
};
use std::io::{Read, Write};
use {
    serde::{Serialize, de::DeserializeOwned},
};

pub struct FramedCborWriter<W: Write> {
    inner: FramedWriter<W>,
}

impl<W: Write> FramedCborWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner: FramedWriter::new(inner) }
    }

    pub fn send<T: Serialize>(&mut self, value: &T) -> Result<(), AbutError> {
        let encoded = ::serde_cbor::to_vec(value)
            .map_err(|e| AbutError::new(crate::AbutCode::Io).ctx(e))?; // or add CborEncode code (recommended)
        self.inner.write_frame(&encoded)
    }
}

pub struct FramedCborReader<R: Read> {
    inner: FramedReader<R>,
    buf: Vec<u8>,
}

impl<R: Read> FramedCborReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner: FramedReader::new(inner), buf: Vec::new() }
    }

    pub fn with_inner(inner: FramedReader<R>) -> Self {
        Self { inner, buf: Vec::new() }
    }

    pub fn recv<T: DeserializeOwned>(&mut self) -> Result<T, AbutError> {
        self.inner.recv_into(&mut self.buf)?;
        ::serde_cbor::from_slice(&self.buf)
            .map_err(|e| AbutError::new(crate::AbutCode::Io).ctx(e)) // or add CborDecode code
    }

    pub fn inner_mut(&mut self) -> &mut FramedReader<R> { &mut self.inner }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::io::Cursor;

    // A sample complex struct to test Serde integration
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TelemetryData {
        id: u32,
        label: String,
        values: Vec<f32>,
    }

    #[test]
    fn test_cbor_roundtrip_single() {
        let mut buffer = Vec::new();
        let mut writer = FramedCborWriter::new(&mut buffer);

        let original = TelemetryData {
            id: 42,
            label: "sensor_alpha".to_string(),
            values: vec![1.0, 2.5, 3.14],
        };

        writer.send(&original).expect("Send should succeed");

        let mut reader = FramedCborReader::new(Cursor::new(buffer));
        let decoded: TelemetryData = reader.recv().expect("Recv should succeed");

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_cbor_multi_frame_sequence() {
        let mut buffer = Vec::new();
        let mut writer = FramedCborWriter::new(&mut buffer);

        // Send three different types/messages in a row
        writer.send(&"first message").unwrap();
        writer.send(&12345u64).unwrap();
        writer.send(&vec![1, 2, 3]).unwrap();

        let mut reader = FramedCborReader::new(Cursor::new(buffer));

        let msg1: String = reader.recv().unwrap();
        let msg2: u64 = reader.recv().unwrap();
        let msg3: Vec<i32> = reader.recv().unwrap();

        assert_eq!(msg1, "first message");
        assert_eq!(msg2, 12345);
        assert_eq!(msg3, vec![1, 2, 3]);
    }

    #[test]
    fn test_cbor_decode_error_recovery() {
        let mut buffer = Vec::new();
        
        // 1. Write a valid frame but with "garbage" CBOR data inside
        let mut writer = FramedWriter::new(&mut buffer);
        writer.write_frame(&[0xFF, 0xFF, 0xFF]).unwrap(); // Invalid CBOR
        
        // 2. Write a valid CBOR frame after it
        let mut cbor_writer = FramedCborWriter::new(&mut buffer);
        cbor_writer.send(&"I am valid").unwrap();

        let mut reader = FramedCborReader::new(Cursor::new(buffer));

        // The first read should fail CBOR decoding
        let first_res: Result<String, _> = reader.recv();
        assert!(first_res.is_err(), "Should fail to decode garbage CBOR");

        // IMPORTANT: Because of your framing, the stream is still aligned!
        // We should be able to read the next valid frame.
        let second_res: String = reader.recv().expect("Should recover and read next frame");
        assert_eq!(second_res, "I am valid");
    }

    #[test]
    fn test_oversized_frame_rejected() {
        let mut buffer = Vec::new();
        let mut writer = FramedCborWriter::new(&mut buffer);
        
        writer.send(&"This is a relatively small string").unwrap();

        // Create a reader with an extremely tiny max frame size (e.g., 2 bytes)
        let framed_reader = FramedReader::with_max(Cursor::new(buffer), 2);
        let mut reader = FramedCborReader::with_inner(framed_reader);

        let res: Result<String, _> = reader.recv();
        
        // This should fail at the Framing layer before even reaching CBOR logic
        assert!(res.is_err());
    }
}