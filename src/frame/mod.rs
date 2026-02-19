//! Length-prefixed framing for stream transports.
//!
//! Format: `<u32_le_len><frame_bytes...>`

use crate::{AbutError, FrameSink, FrameSource, ReaderConfig};

use super::BufferTooSmall;

use std::io::{Read, Write};

/// Number of bytes used for the length prefix.
pub const LEN_PREFIX: usize = 4;

/// A writer that frames telemetry frames with a u32 length prefix.
#[derive(Debug)]
pub struct FramedWriter<W: Write> {
    inner: W,
}

impl<W: Write> FramedWriter<W> {
    pub fn new(inner: W) -> Self { Self { inner } }

    /// Convenience wrapper that delegates to the `TelemetrySink` implementation.
    ///
    /// This lets you call `writer.send_bytes(..)` without importing the trait.
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<(), AbutError> {
        <Self as FrameSink>::send_frame(self, bytes)
    }

    pub fn into_inner(self) -> W { self.inner }
    pub fn inner_mut(&mut self) -> &mut W { &mut self.inner }
    
    /// Writes one frame. Does NOT flush (caller controls flushing).
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<(), AbutError> {
        let len: u32 = bytes
            .len()
            .try_into()
            .map_err(|_| AbutError::frame_too_large(bytes.len(), u32::MAX as usize))?;

        self.inner.write_all(&len.to_le_bytes())?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), AbutError> {
        self.inner.flush()?;
        Ok(())
    }
}

impl<W: Write> FrameSink for FramedWriter<W> {
    type Error = AbutError;
    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_frame(bytes)
    }
}


impl<R: Read> FramedReader<R> {
    pub fn max_frame_len(&self) -> usize {
        self.cfg.max_frame_len
    }
}

/// A reader that consumes length-prefixed telemetry frames.
#[derive(Debug)]
pub struct FramedReader<R: Read> {
    inner: R,
    cfg: ReaderConfig,
}

impl<R: Read> FramedReader<R> {
    pub fn new(inner: R) -> Self { Self::with_config(inner, ReaderConfig::default()) }
    pub fn with_max(inner: R, max_frame_len: usize) -> Self {
        Self::with_config(inner, ReaderConfig { max_frame_len, ..Default::default() })
    }
    pub fn with_config(inner: R, cfg: ReaderConfig) -> Self { Self { inner, cfg } }

    pub fn into_inner(self) -> R { self.inner }
    pub fn inner_mut(&mut self) -> &mut R { &mut self.inner }
    pub fn config(&self) -> ReaderConfig { self.cfg }

    fn drain_exact(&mut self, len: usize) -> Result<(), AbutError> {
        let mut sink = std::io::sink();
        std::io::copy(&mut self.inner.by_ref().take(len as u64), &mut sink)?;
        Ok(())
    }

    fn read_len(&mut self) -> Result<usize, AbutError> {
        let mut len_buf = [0u8; LEN_PREFIX];
        self.inner.read_exact(&mut len_buf)?;
        Ok(u32::from_le_bytes(len_buf) as usize)
    }

    /// Reads the next frame into `dst`, resizing it exactly to the frame length.
    pub fn recv_into(&mut self, dst: &mut Vec<u8>) -> Result<(), AbutError> {
        let len = self.read_len()?;

        if len > self.cfg.max_frame_len {
            if self.cfg.drain_oversize_up_to != 0 && len <= self.cfg.drain_oversize_up_to {
                self.drain_exact(len)?;
            }
            return Err(AbutError::frame_too_large(len, self.cfg.max_frame_len));
        }

        dst.clear();
        dst.resize(len, 0u8);
        self.inner.read_exact(dst)?;
        Ok(())
    }

    /// Reads the next frame into a caller-provided slice.
    pub fn read_frame(&mut self, dst: &mut [u8]) -> Result<usize, AbutError> {
        let len = self.read_len()?;

        if len > self.cfg.max_frame_len {
            if self.cfg.drain_oversize_up_to != 0 && len <= self.cfg.drain_oversize_up_to {
                self.drain_exact(len)?;
            }
            return Err(AbutError::frame_too_large(len, self.cfg.max_frame_len));
        }

        if dst.len() < len {
            if self.cfg.drain_on_small_buffer {
                self.drain_exact(len)?;
            }
            return Err(AbutError::buffer_too_small(len));
        }

        self.inner.read_exact(&mut dst[..len])?;
        Ok(len)
    }
}

impl<R: Read> FrameSource for FramedReader<R> {
    type Error = AbutError;
    fn recv_frame(&mut self, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.read_frame(dst)
    }
}
impl From<BufferTooSmall> for AbutError {
    fn from(e: BufferTooSmall) -> Self {
        AbutError::buffer_too_small(e.needed)
    }
}

#[allow(unused)]
fn send_structured_log<W: Write>(mut sink: impl FrameSink<Error = std::io::Error>) {
    let payload = b"hello";
    let mut framed = vec![];
    framed.extend_from_slice(payload);
    sink.send_frame(&framed).unwrap();
}



pub mod cbor;
pub mod postcard;



#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_roundtrip_basic() {
        let mut buffer = Vec::new();
        let mut writer = FramedWriter::new(&mut buffer);

        let frame1 = b"hello world";
        let frame2 = b"rust is cool";

        writer.write_frame(frame1).expect("Write frame 1");
        writer.write_frame(frame2).expect("Write frame 2");

        let mut reader = FramedReader::new(Cursor::new(buffer));
        let mut dst = Vec::new();

        reader.recv_into(&mut dst).expect("Read frame 1");
        assert_eq!(dst, frame1);

        reader.recv_into(&mut dst).expect("Read frame 2");
        assert_eq!(dst, frame2);
    }

    #[test]
    fn test_zero_length_frame() {
        let mut buffer = Vec::new();
        let mut writer = FramedWriter::new(&mut buffer);

        writer.write_frame(b"").expect("Write empty frame");

        let mut reader = FramedReader::new(Cursor::new(buffer));
        let mut dst = vec![1, 2, 3]; // Pre-fill to ensure it clears
        
        reader.recv_into(&mut dst).expect("Read empty frame");
        assert!(dst.is_empty());
    }

    #[test]
    fn test_max_frame_size_enforcement() {
        let mut buffer = Vec::new();
        let mut writer = FramedWriter::new(&mut buffer);

        let large_frame = vec![0u8; 100];
        writer.write_frame(&large_frame).unwrap();

        // Set max size smaller than the frame we just wrote
        let mut reader = FramedReader::with_max(Cursor::new(buffer), 50);
        let mut dst = Vec::new();

        let result = reader.recv_into(&mut dst);
        assert!(result.is_err(), "Should fail because frame exceeds max_frame_len");
    }

    #[test]
    fn test_drain_on_small_buffer() {
        let mut buffer = Vec::new();
        let mut writer = FramedWriter::new(&mut buffer);

        writer.write_frame(b"long_payload").unwrap();
        writer.write_frame(b"next_frame").unwrap();

        // Config: Drain if buffer is too small
        let cfg = ReaderConfig {
            max_frame_len: 1024,
            drain_on_small_buffer: true,
            ..Default::default()
        };
        
        let mut reader = FramedReader::with_config(Cursor::new(buffer), cfg);
        let mut small_dst = [0u8; 4];

        // This should fail but DRAIN the 12 bytes of "long_payload"
        let res = reader.read_frame(&mut small_dst);
        assert!(res.is_err());

        // This should now read "next_frame" because the stream stayed in sync
        let mut next_dst = [0u8; 10];
        let len = reader.read_frame(&mut next_dst).expect("Should read next frame");
        assert_eq!(&next_dst[..len], b"next_frame");
    }

    #[test]
    fn test_incomplete_length_prefix() {
        let short_data = vec![0u8; 2]; // Only 2 bytes, but we need 4 for u32
        let mut reader = FramedReader::new(Cursor::new(short_data));
        let mut dst = Vec::new();

        let res = reader.recv_into(&mut dst);
        assert!(res.is_err(), "Should fail due to UnexpectedEof");
    }
}