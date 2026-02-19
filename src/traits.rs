pub trait FrameSink {
    type Error;
    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait FrameSource {
    type Error;
    fn recv_frame(&mut self, dst: &mut [u8]) -> Result<usize, Self::Error>;
}
