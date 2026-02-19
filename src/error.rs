// 
// /// Errors that can occur while reading length-prefixed frames.
// #[derive(Debug, thiserror::Error)]
// pub enum FramedReadError {
//     #[error("i/o: {0}")]
//     Io(#[from] std::io::Error),
// 
//     #[error("buffer too small (need {needed} bytes)")]
//     BufferTooSmall { needed: usize },
// 
//     #[error("frame length {len} exceeds max_frame_len {max}")]
//     FrameTooLarge { len: usize, max: usize },
// }
// 


use std::{fmt, io};


use liaise::{Liaise, RegisterErrors};

#[derive(RegisterErrors, Debug, Copy, Clone)]
#[error_prefix = "FILE"] // Sets the reporting prefix
pub enum AbutCode {
    Io = 1,
    BufferTooSmall = 2,
    FrameTooLarge = 3,
    #[cfg(feature = "postcard")]
    PostcardEncode = 10,
    #[cfg(feature = "postcard")]
    PostcardDecode = 11
}

impl Liaise for AbutCode {
    fn code_id(self) -> u16 { self as u16 }
    
    fn message(self) -> &'static str {
        match self {
            Self::Io => "I/O error",
            Self::BufferTooSmall => "Buffer too small",
            Self::FrameTooLarge => "Frame too large",
            #[cfg(feature = "postcard")]
            Self::PostcardEncode => "Postcard encode failed",
            #[cfg(feature = "postcard")]
            Self::PostcardDecode => "Postcard decode failed",
        }
    }
}

/// Concrete runtime error type for the crate.
/// Uses `liaise` for stable IDs + formatting; no `thiserror`.
#[derive(Debug)]
pub struct AbutError {
    pub code: AbutCode,
    pub ctx: Option<String>,

    // Optional sources (keep if you want better debugging).
    pub source: Option<AbutSource>,
}

#[derive(Debug)]
pub enum AbutSource {
    Io(io::Error),
    #[cfg(feature = "postcard")]
    Postcard(postcard::Error),
}

impl AbutError {
    #[inline]
    pub fn new(code: AbutCode) -> Self {
        Self { code, ctx: None, source: None }
    }

    #[inline]
    pub fn ctx(mut self, ctx: impl fmt::Display) -> Self {
        self.ctx = Some(ctx.to_string());
        self
    }

    #[inline]
    pub fn io(err: io::Error) -> Self {
        Self {
            code: AbutCode::Io,
            ctx: Some(err.to_string()),
            source: Some(AbutSource::Io(err)),
        }
    }

    #[inline]
    pub fn buffer_too_small(needed: usize) -> Self {
        Self::new(AbutCode::BufferTooSmall).ctx(format_args!("need {needed} bytes"))
    }

    #[inline]
    pub fn frame_too_large(len: usize, max: usize) -> Self {
        Self::new(AbutCode::FrameTooLarge).ctx(format_args!("len {len} exceeds max {max}"))
    }

    #[cfg(feature = "postcard")]
    #[inline]
    pub fn postcard_encode(err: postcard::Error) -> Self {
        Self {
            code: AbutCode::PostcardEncode,
            ctx: Some(err.to_string()),
            source: Some(AbutSource::Postcard(err)),
        }
    }

    #[cfg(feature = "postcard")]
    #[inline]
    pub fn postcard_decode(err: postcard::Error) -> Self {
        Self {
            code: AbutCode::PostcardDecode,
            ctx: Some(err.to_string()),
            source: Some(AbutSource::Postcard(err)),
        }
    }
}

impl fmt::Display for AbutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Your canonical format: "[ABUT0001] msg"
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
            None => None,
        }
    }
}

impl From<std::io::Error> for AbutError {
    #[inline]
    fn from(e: std::io::Error) -> Self {
        AbutError::io(e)
    }
}
