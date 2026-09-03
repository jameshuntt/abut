//! Typed frames encoded with CBOR (feature `cbor`).
//!
//! Each `send` encodes one `serde` value into one frame; each `recv` reads
//! one frame and decodes it. Because the frame boundary is independent of the
//! payload, a value that fails to decode leaves the stream aligned for the
//! next one.

use std::io::{Read, Write};

use serde::{de::DeserializeOwned, Serialize};

use crate::frame::{FramedReader, FramedWriter};
use crate::AbutError;

/// Writes `serde` values as CBOR frames.
#[derive(Debug)]
pub struct FramedCborWriter<W: Write> {
    inner: FramedWriter<W>,
}

impl<W: Write> FramedCborWriter<W> {
    /// A writer over `inner`.
    pub fn new(inner: W) -> Self {
        Self { inner: FramedWriter::new(inner) }
    }

    /// Encode `value` and write it as one frame.
    pub fn send<T: Serialize>(&mut self, value: &T) -> Result<(), AbutError> {
        let encoded = serde_cbor::to_vec(value).map_err(AbutError::cbor_encode)?;
        self.inner.write_frame(&encoded)
    }

    /// Flush the underlying writer.
    pub fn flush(&mut self) -> Result<(), AbutError> {
        self.inner.flush()
    }

    /// The framed writer underneath.
    pub fn inner_mut(&mut self) -> &mut FramedWriter<W> {
        &mut self.inner
    }

    /// Take the underlying writer back.
    pub fn into_inner(self) -> W {
        self.inner.into_inner()
    }
}

/// Reads CBOR frames into `serde` values, reusing one buffer across reads.
#[derive(Debug)]
pub struct FramedCborReader<R: Read> {
    inner: FramedReader<R>,
    buf: Vec<u8>,
}

impl<R: Read> FramedCborReader<R> {
    /// A reader over `inner` with the default frame configuration.
    pub fn new(inner: R) -> Self {
        Self::with_inner(FramedReader::new(inner))
    }

    /// A reader over an already-configured [`FramedReader`].
    pub fn with_inner(inner: FramedReader<R>) -> Self {
        Self { inner, buf: Vec::new() }
    }

    /// Read one frame and decode it.
    pub fn recv<T: DeserializeOwned>(&mut self) -> Result<T, AbutError> {
        self.inner.recv_into(&mut self.buf)?;
        serde_cbor::from_slice(&self.buf).map_err(AbutError::cbor_decode)
    }

    /// The framed reader underneath.
    pub fn inner_mut(&mut self) -> &mut FramedReader<R> {
        &mut self.inner
    }

    /// Take the underlying reader back.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}
