//! Typed frames encoded with postcard (feature `postcard`).
//!
//! postcard is compact (varints, no field names), which suits frames between
//! processes that share the same type definitions. Each `send` is one frame,
//! each `recv` one frame; a decode failure leaves the stream aligned.

use std::io::{Read, Write};

use serde::{de::DeserializeOwned, Serialize};

use crate::frame::{FramedReader, FramedWriter};
use crate::AbutError;

/// Writes `serde` values as postcard frames, reusing one buffer across sends.
#[derive(Debug)]
pub struct FramedPostcardWriter<W: Write> {
    inner: FramedWriter<W>,
    buf: Vec<u8>,
}

impl<W: Write> FramedPostcardWriter<W> {
    /// A writer over `inner`.
    pub fn new(inner: W) -> Self {
        Self { inner: FramedWriter::new(inner), buf: Vec::new() }
    }

    /// Encode `value` and write it as one frame.
    pub fn send<T: Serialize>(&mut self, value: &T) -> Result<(), AbutError> {
        self.buf.clear();
        let buf = std::mem::take(&mut self.buf);
        self.buf = postcard::to_extend(value, buf).map_err(AbutError::postcard_encode)?;
        self.inner.write_frame(&self.buf)
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

/// Reads postcard frames into `serde` values, reusing one buffer across reads.
#[derive(Debug)]
pub struct FramedPostcardReader<R: Read> {
    inner: FramedReader<R>,
    buf: Vec<u8>,
}

impl<R: Read> FramedPostcardReader<R> {
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
        postcard::from_bytes(&self.buf).map_err(AbutError::postcard_decode)
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
