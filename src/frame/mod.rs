//! Length-prefixed framing for stream transports.
//!
//! Wire format: a little-endian `u32` length, then that many bytes.
//!
//! ```text
//! <len: u32 le><frame bytes ...><len: u32 le><frame bytes ...>...
//! ```
//!
//! [`FramedWriter`] writes frames; [`FramedReader`] reads them back with a
//! maximum length and the drain rules from [`ReaderConfig`].

use std::io::{Read, Write};

use crate::{AbutError, BufferTooSmall, FrameSink, FrameSource, ReaderConfig};

#[cfg(feature = "cbor")]
pub mod cbor;
#[cfg(feature = "postcard")]
pub mod postcard;

/// Number of bytes in the length prefix.
pub const LEN_PREFIX: usize = 4;

/// Writes frames with a `u32` length prefix to any `Write`.
///
/// Nothing is flushed implicitly; call [`FramedWriter::flush`] when the
/// transport buffers.
#[derive(Debug)]
pub struct FramedWriter<W: Write> {
    inner: W,
}

impl<W: Write> FramedWriter<W> {
    /// A writer over `inner`.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Write one frame: the length prefix, then the bytes. Fails with
    /// `FrameTooLarge` if the frame is longer than a `u32` can describe.
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<(), AbutError> {
        let len: u32 = bytes
            .len()
            .try_into()
            .map_err(|_| AbutError::frame_too_large(bytes.len(), u32::MAX as usize))?;
        self.inner.write_all(&len.to_le_bytes())?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    /// [`FramedWriter::write_frame`] under the name the [`FrameSink`] trait
    /// uses, without importing the trait.
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<(), AbutError> {
        self.write_frame(bytes)
    }

    /// Flush the underlying writer.
    pub fn flush(&mut self) -> Result<(), AbutError> {
        self.inner.flush()?;
        Ok(())
    }

    /// The underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// The underlying writer, mutably.
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Take the underlying writer back.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> FrameSink for FramedWriter<W> {
    type Error = AbutError;
    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_frame(bytes)
    }
}

/// Reads length-prefixed frames from any `Read`, refusing frames longer than
/// its maximum and keeping the stream aligned according to its
/// [`ReaderConfig`].
#[derive(Debug)]
pub struct FramedReader<R: Read> {
    inner: R,
    cfg: ReaderConfig,
}

impl<R: Read> FramedReader<R> {
    /// A reader over `inner` with the default configuration.
    pub fn new(inner: R) -> Self {
        Self::with_config(inner, ReaderConfig::default())
    }

    /// A reader with a different maximum frame length and default drain rules.
    pub fn with_max(inner: R, max_frame_len: usize) -> Self {
        Self::with_config(inner, ReaderConfig::with_max_frame_len(max_frame_len))
    }

    /// A reader with an explicit configuration.
    pub fn with_config(inner: R, cfg: ReaderConfig) -> Self {
        Self { inner, cfg }
    }

    /// The configuration in effect.
    pub fn config(&self) -> ReaderConfig {
        self.cfg
    }

    /// The maximum frame length in effect.
    pub fn max_frame_len(&self) -> usize {
        self.cfg.max_frame_len
    }

    /// The underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// The underlying reader, mutably.
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Take the underlying reader back.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Read the next frame into `dst`, resizing it to exactly the frame's
    /// length. An oversize frame is refused with `FrameTooLarge` and drained
    /// only if the configuration allows.
    pub fn recv_into(&mut self, dst: &mut Vec<u8>) -> Result<(), AbutError> {
        let len = self.read_len()?;
        self.refuse_oversize(len)?;
        dst.clear();
        dst.resize(len, 0u8);
        self.inner.read_exact(dst)?;
        Ok(())
    }

    /// Read the next frame into the caller's slice and return its length.
    ///
    /// An oversize frame is refused with `FrameTooLarge`. A frame longer than
    /// `dst` is refused with `BufferTooSmall { needed }` and, by default,
    /// drained so the following call reads the following frame.
    pub fn read_frame(&mut self, dst: &mut [u8]) -> Result<usize, AbutError> {
        let len = self.read_len()?;
        self.refuse_oversize(len)?;
        if dst.len() < len {
            if self.cfg.drain_on_small_buffer {
                self.drain_exact(len)?;
            }
            return Err(AbutError::buffer_too_small(len));
        }
        self.inner.read_exact(&mut dst[..len])?;
        Ok(len)
    }

    fn read_len(&mut self) -> Result<usize, AbutError> {
        let mut len_buf = [0u8; LEN_PREFIX];
        self.inner.read_exact(&mut len_buf)?;
        Ok(u32::from_le_bytes(len_buf) as usize)
    }

    fn refuse_oversize(&mut self, len: usize) -> Result<(), AbutError> {
        if len <= self.cfg.max_frame_len {
            return Ok(());
        }
        if self.cfg.drain_oversize_up_to != 0 && len <= self.cfg.drain_oversize_up_to {
            self.drain_exact(len)?;
        }
        Err(AbutError::frame_too_large(len, self.cfg.max_frame_len))
    }

    fn drain_exact(&mut self, len: usize) -> Result<(), AbutError> {
        let mut sink = std::io::sink();
        std::io::copy(&mut self.inner.by_ref().take(len as u64), &mut sink)?;
        Ok(())
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
