//! Error codes and the error type.
//!
//! Every failure has a stable code rendered as `[ABUT<code>] <message>`, so a
//! log line or a peer can be matched on the code rather than on prose. The
//! underlying I/O or codec error is kept as the [`std::error::Error::source`].

use std::{fmt, io};

use liaise::{Liaise, LiaiseCodes};

/// What went wrong, as a stable code.
#[derive(LiaiseCodes, Debug)]
#[liaise(prefix = "ABUT")]
pub enum AbutCode {
    /// The underlying `Read` or `Write` failed; the I/O error is the source.
    #[liaise(code = 1, msg = "I/O failure")]
    Io,

    /// The caller's buffer is shorter than the frame that arrived.
    #[liaise(code = 2, msg = "Buffer too small (need {needed} bytes)")]
    BufferTooSmall {
        /// The frame's length.
        needed: usize,
    },

    /// A frame's declared length exceeds the reader's maximum, or a frame to
    /// write is longer than a `u32` can describe.
    #[liaise(code = 3, msg = "Frame too large")]
    FrameTooLarge,

    /// `postcard` could not encode the value; the codec error is the source.
    #[cfg(feature = "postcard")]
    #[liaise(code = 10, msg = "Postcard encode failed")]
    PostcardEncode,

    /// `postcard` could not decode the frame; the codec error is the source.
    #[cfg(feature = "postcard")]
    #[liaise(code = 11, msg = "Postcard decode failed")]
    PostcardDecode,

    /// CBOR could not encode the value; the codec error is the source.
    #[cfg(feature = "cbor")]
    #[liaise(code = 20, msg = "CBOR encode failed")]
    CborEncode,

    /// CBOR could not decode the frame; the codec error is the source.
    #[cfg(feature = "cbor")]
    #[liaise(code = 21, msg = "CBOR decode failed")]
    CborDecode,
}

/// The underlying error behind an [`AbutError`], when there is one.
#[derive(Debug)]
pub enum AbutSource {
    /// A `Read` or `Write` failed.
    Io(io::Error),
    /// A postcard encode or decode failed.
    #[cfg(feature = "postcard")]
    Postcard(postcard::Error),
    /// A CBOR encode or decode failed.
    #[cfg(feature = "cbor")]
    Cbor(serde_cbor::Error),
}

/// The crate's error: a code, optional context, and the underlying source.
#[derive(Debug)]
pub struct AbutError {
    /// The stable code.
    pub code: AbutCode,
    /// Free text about this occurrence, such as the lengths involved.
    pub ctx: Option<String>,
    /// The underlying error, if any.
    pub source: Option<AbutSource>,
}

impl AbutError {
    /// An error with only a code.
    #[inline]
    pub fn new(code: AbutCode) -> Self {
        Self { code, ctx: None, source: None }
    }

    /// The same error with context text attached.
    #[inline]
    pub fn ctx(mut self, ctx: impl fmt::Display) -> Self {
        self.ctx = Some(ctx.to_string());
        self
    }

    /// An I/O failure, keeping the I/O error as the source.
    #[inline]
    pub fn io(err: io::Error) -> Self {
        Self { code: AbutCode::Io, ctx: Some(err.to_string()), source: Some(AbutSource::Io(err)) }
    }

    /// The caller's buffer cannot hold a frame of `needed` bytes.
    #[inline]
    pub fn buffer_too_small(needed: usize) -> Self {
        Self::new(AbutCode::BufferTooSmall { needed })
    }

    /// A frame of `len` bytes where at most `max` are allowed.
    #[inline]
    pub fn frame_too_large(len: usize, max: usize) -> Self {
        Self::new(AbutCode::FrameTooLarge).ctx(format_args!("len {len} exceeds max {max}"))
    }

    /// A postcard encode failure, keeping the codec error as the source.
    #[cfg(feature = "postcard")]
    #[inline]
    pub fn postcard_encode(err: postcard::Error) -> Self {
        Self { code: AbutCode::PostcardEncode, ctx: Some(err.to_string()), source: Some(AbutSource::Postcard(err)) }
    }

    /// A postcard decode failure, keeping the codec error as the source.
    #[cfg(feature = "postcard")]
    #[inline]
    pub fn postcard_decode(err: postcard::Error) -> Self {
        Self { code: AbutCode::PostcardDecode, ctx: Some(err.to_string()), source: Some(AbutSource::Postcard(err)) }
    }

    /// A CBOR encode failure, keeping the codec error as the source.
    #[cfg(feature = "cbor")]
    #[inline]
    pub fn cbor_encode(err: serde_cbor::Error) -> Self {
        Self { code: AbutCode::CborEncode, ctx: Some(err.to_string()), source: Some(AbutSource::Cbor(err)) }
    }

    /// A CBOR decode failure, keeping the codec error as the source.
    #[cfg(feature = "cbor")]
    #[inline]
    pub fn cbor_decode(err: serde_cbor::Error) -> Self {
        Self { code: AbutCode::CborDecode, ctx: Some(err.to_string()), source: Some(AbutSource::Cbor(err)) }
    }
}

impl fmt::Display for AbutError {
    /// `[ABUT0003] Frame too large: len 100 exceeds max 50`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = self.code.render();
        match &self.ctx {
            Some(ctx) => write!(f, "{base}: {ctx}"),
            None => write!(f, "{base}"),
        }
    }
}

impl std::error::Error for AbutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.source {
            Some(AbutSource::Io(e)) => Some(e),
            #[cfg(feature = "postcard")]
            Some(AbutSource::Postcard(e)) => Some(e),
            #[cfg(feature = "cbor")]
            Some(AbutSource::Cbor(e)) => Some(e),
            None => None,
        }
    }
}

impl From<io::Error> for AbutError {
    #[inline]
    fn from(e: io::Error) -> Self {
        AbutError::io(e)
    }
}
