//! The seam between things that produce frames and things that carry them.

/// Something frames can be handed to: a framed writer, a ring producer, a
/// test recorder.
pub trait FrameSink {
    /// Why a frame was not accepted.
    type Error;
    /// Accept one whole frame.
    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Something frames can be taken from, one at a time, into a caller-owned
/// buffer.
pub trait FrameSource {
    /// Why a frame could not be delivered.
    type Error;
    /// Receive the next frame into `dst` and return its length.
    fn recv_frame(&mut self, dst: &mut [u8]) -> Result<usize, Self::Error>;
}
