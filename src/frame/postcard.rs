#![cfg(feature = "postcard")]

use std::io::{Read, Write};
use crate::{AbutError, frame::{FramedReader, FramedWriter}};

#[cfg(feature = "postcard")]
use serde::{Serialize, de::DeserializeOwned};

#[cfg(feature = "postcard")]
pub struct FramedPostcardWriter<W: Write> {
    inner: FramedWriter<W>,
    buf: Vec<u8>,
}

#[cfg(feature = "postcard")]
impl<W: Write> FramedPostcardWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner: FramedWriter::new(inner), buf: Vec::new() }
    }

    pub fn send<T: Serialize>(&mut self, value: &T) -> Result<(), AbutError> {
        self.buf.clear();
        
        // Instead of to_extend, use the more flexible flavor 
        // or simply pass the buf by value and re-assign it.
        // However, postcard provides a better way for Vecs:
        
        let serialized = postcard::to_extend(value, std::mem::take(&mut self.buf))
            .map_err(AbutError::postcard_encode)?;
        
        // Put the buffer back into our struct so we can reuse the allocation
        self.buf = serialized;
        
        self.inner.write_frame(&self.buf)
    }

    pub fn flush(&mut self) -> Result<(), AbutError> {
        self.inner.flush()
    }
}

#[cfg(feature = "postcard")]
pub struct FramedPostcardReader<R: Read> {
    inner: FramedReader<R>,
    buf: Vec<u8>,
}

#[cfg(feature = "postcard")]
impl<R: Read> FramedPostcardReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner: FramedReader::new(inner), buf: Vec::new() }
    }

    pub fn with_inner(inner: FramedReader<R>) -> Self {
        Self { inner, buf: Vec::new() }
    }

    pub fn recv<T: DeserializeOwned>(&mut self) -> Result<T, AbutError> {
        self.inner.recv_into(&mut self.buf)?;
        postcard::from_bytes(&self.buf).map_err(AbutError::postcard_decode)
    }

    pub fn inner_mut(&mut self) -> &mut FramedReader<R> { &mut self.inner }
}



#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::io::Cursor;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum DeviceCommand {
        Reboot,
        SetGain(u16),
        Status { active: bool, battery: u8 },
    }

    #[test]
    fn test_postcard_roundtrip_enum() {
        let mut buffer = Vec::new();
        let mut writer = FramedPostcardWriter::new(&mut buffer);

        // Test a few variants of an enum
        let cmd1 = DeviceCommand::SetGain(500);
        let cmd2 = DeviceCommand::Status { active: true, battery: 88 };

        writer.send(&cmd1).expect("Send cmd1");
        writer.send(&cmd2).expect("Send cmd2");

        let mut reader = FramedPostcardReader::new(Cursor::new(buffer));
        
        let res1: DeviceCommand = reader.recv().expect("Recv cmd1");
        let res2: DeviceCommand = reader.recv().expect("Recv cmd2");

        assert_eq!(cmd1, res1);
        assert_eq!(cmd2, res2);
    }

    #[test]
    fn test_postcard_efficiency_vs_cbor() {
        // Postcard is Varint-based and should be very compact
        let mut buffer = Vec::new();
        let mut writer = FramedPostcardWriter::new(&mut buffer);

        let data = DeviceCommand::SetGain(1); // Should be very few bytes
        writer.send(&data).unwrap();

        // 4 bytes for length prefix + 2 bytes for postcard payload (tag + value)
        assert!(buffer.len() <= 6, "Postcard should be extremely compact");
    }

    #[test]
    fn test_postcard_reader_reuses_buffer() {
        let mut buffer = Vec::new();
        let mut writer = FramedPostcardWriter::new(&mut buffer);

        writer.send(&"short").unwrap();
        writer.send(&"a much longer string than the first one").unwrap();

        let mut reader = FramedPostcardReader::new(Cursor::new(buffer));

        // Read first
        let _: String = reader.recv().unwrap();
        let cap_after_first = reader.buf.capacity();

        // Read second (longer)
        let _: String = reader.recv().unwrap();
        
        // Ensure we aren't constantly shrinking/reallocating unnecessarily
        assert!(reader.buf.capacity() >= cap_after_first);
    }

    #[test]
    fn test_postcard_decode_error() {
        let mut buffer = Vec::new();
        let mut writer = FramedWriter::new(&mut buffer);

        // Write a frame that is NOT a valid postcard string (invalid varint/tag)
        writer.write_frame(&[0xFF, 0xFF, 0xFF]).unwrap();

        let mut reader = FramedPostcardReader::new(Cursor::new(buffer));
        let res: Result<String, _> = reader.recv();

        assert!(res.is_err(), "Postcard should fail to decode invalid bytes");
    }
}