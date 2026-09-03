//! Length-prefixed frames over any `Read` and `Write`.
//!
//! To abut is to share a boundary. Two processes that abut, over a Unix
//! socket, a pipe or a TCP stream, need to agree where one message ends and
//! the next begins. This crate is that agreement and nothing more:
//!
//! * [`frame::FramedWriter`] and [`frame::FramedReader`] put a little-endian
//!   `u32` length in front of each frame and read it back. The reader has a
//!   maximum frame length, and when a frame is refused, because it is too big
//!   or the caller's buffer is too small, it can drain the bytes so the stream
//!   is still aligned for the next call. That is the property that keeps a
//!   long-lived connection usable after one bad message.
//! * [`frame::cbor`] and [`frame::postcard`] (features `cbor` and `postcard`,
//!   both on by default) send and receive `serde` values as frames.
//! * [`ring`] (feature `ring-heapless`) is a bounded, allocation-free
//!   single-producer single-consumer ring of whole frames, for handing frames
//!   from a hot thread to a slower one that writes them out.
//!
//! The [`FrameSink`] and [`FrameSource`] traits are the seam: a ring producer
//! and a framed writer both accept frames, so code that emits frames does not
//! care which it holds.
//!
//! Errors carry a stable code (`[ABUT0002] Buffer too small (need 12 bytes)`)
//! through the `liaise` crate, plus the underlying I/O or codec error as a
//! source.
//!
//! What is not here: message schemas, request/response matching, sockets
//! themselves. Bring your own `Read` and `Write`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

// The `liaise` derive expands to paths under `alloc::`.
extern crate alloc;

pub mod error;
pub mod frame;
#[cfg(feature = "ring-heapless")]
pub mod ring;
pub mod traits;
pub mod types;

pub use error::{AbutCode, AbutError, AbutSource};
pub use frame::{FramedReader, FramedWriter, LEN_PREFIX};
pub use traits::{FrameSink, FrameSource};
pub use types::{BufferTooSmall, ReaderConfig, DEFAULT_MAX_FRAME_LEN};

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctests {}
