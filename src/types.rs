/// Returned by sources that need a larger destination buffer to receive a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferTooSmall {
    pub needed: usize,
}

impl core::fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "buffer too small (need {} bytes)", self.needed)
    }
}

impl std::error::Error for BufferTooSmall {}


#[derive(Debug, Clone, Copy)]
pub struct ReaderConfig {
    pub max_frame_len: usize,

    /// Drain frame bytes if dst buffer is too small (safe because len <= max_frame_len).
    pub drain_on_small_buffer: bool,

    /// If the peer claims an oversize frame, only drain it if len <= drain_oversize_up_to.
    /// 0 = never drain oversize (recommended default).
    pub drain_oversize_up_to: usize,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            max_frame_len: 64 * 1024,
            drain_on_small_buffer: true,
            drain_oversize_up_to: 0,
        }
    }
}