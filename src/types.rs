//! Reader configuration and the small shared error value.

/// The reader's default maximum frame length, 64 KiB.
pub const DEFAULT_MAX_FRAME_LEN: usize = 64 * 1024;

/// Returned by sources that need a larger destination buffer to receive a
/// frame. Converts into [`crate::AbutError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferTooSmall {
    /// The frame's length.
    pub needed: usize,
}

impl core::fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "buffer too small (need {} bytes)", self.needed)
    }
}

impl std::error::Error for BufferTooSmall {}

/// How a [`crate::FramedReader`] treats frames it cannot deliver.
///
/// The two drain settings decide whether the reader consumes a refused
/// frame's bytes so the stream stays aligned for the next call. Draining a
/// frame the caller's buffer cannot hold is always safe, because the length
/// was already checked against `max_frame_len`. Draining an oversize frame
/// means reading however many bytes the peer claimed, so it is off by
/// default and bounded when on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReaderConfig {
    /// Frames longer than this are refused with `FrameTooLarge`.
    pub max_frame_len: usize,
    /// When the caller's buffer is shorter than the frame, consume the frame
    /// anyway so the next read starts at the next frame. Default `true`.
    pub drain_on_small_buffer: bool,
    /// When a frame exceeds `max_frame_len` but not this value, consume it so
    /// the stream stays aligned. `0` never drains an oversize frame, which is
    /// the default: a peer claiming a huge frame should not make the reader
    /// pull that many bytes.
    pub drain_oversize_up_to: usize,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self { max_frame_len: DEFAULT_MAX_FRAME_LEN, drain_on_small_buffer: true, drain_oversize_up_to: 0 }
    }
}

impl ReaderConfig {
    /// The default configuration with a different maximum frame length.
    pub fn with_max_frame_len(max_frame_len: usize) -> Self {
        Self { max_frame_len, ..Self::default() }
    }
}
