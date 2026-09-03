//! A bounded, allocation-free ring of whole frames (feature `ring-heapless`).
//!
//! One producer, one consumer, fixed capacity, every frame stored in place:
//! the shape for a hot thread that must never block on the allocator handing
//! frames to a slower thread that writes them out. Split a [`HeaplessRing`]
//! into its [`HeaplessProducer`] and [`HeaplessConsumer`] halves and move each
//! to its thread; the producer implements [`FrameSink`], so the same code can
//! feed a ring or a [`crate::FramedWriter`].
//!
//! Sizes are const generics: `HeaplessRing<FRAME_MAX, DEPTH>` holds up to
//! `DEPTH - 1` frames of at most `FRAME_MAX` bytes, all on the stack or
//! wherever you put the ring. Large rings belong in a `Box` or a `static`.

use heapless::spsc::{Consumer, Producer, Queue};
use heapless::Vec;
use liaise::{Liaise, LiaiseCodes};

use crate::traits::FrameSink;

/// Why the ring refused a frame.
#[derive(LiaiseCodes, Debug, Clone, Copy, PartialEq, Eq)]
#[liaise(prefix = "RING")]
pub enum RingError {
    /// The frame is longer than `FRAME_MAX`.
    #[liaise(code = 1, msg = "Frame too large for this ring")]
    FrameTooLarge,
    /// The ring holds `DEPTH - 1` frames already; the consumer must pop first.
    #[liaise(code = 2, msg = "Ring full")]
    Full,
}

/// The ring itself. Own it, then [`HeaplessRing::split`] it.
#[derive(Debug)]
pub struct HeaplessRing<const FRAME_MAX: usize, const DEPTH: usize> {
    q: Queue<Vec<u8, FRAME_MAX>, DEPTH>,
}

impl<const FRAME_MAX: usize, const DEPTH: usize> Default for HeaplessRing<FRAME_MAX, DEPTH> {
    fn default() -> Self {
        Self { q: Queue::new() }
    }
}

impl<const FRAME_MAX: usize, const DEPTH: usize> HeaplessRing<FRAME_MAX, DEPTH> {
    /// An empty ring.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many frames the ring can hold at once: `DEPTH - 1`.
    pub const fn capacity() -> usize {
        DEPTH - 1
    }

    /// The largest frame the ring accepts.
    pub const fn frame_max() -> usize {
        FRAME_MAX
    }

    /// Split into the producer and consumer halves. Each may move to its own
    /// thread; both borrow the ring, so the ring outlives them.
    pub fn split(&mut self) -> (HeaplessProducer<'_, FRAME_MAX, DEPTH>, HeaplessConsumer<'_, FRAME_MAX, DEPTH>) {
        let (p, c) = self.q.split();
        (HeaplessProducer { inner: p }, HeaplessConsumer { inner: c })
    }
}

/// The producer half: accepts frames until the ring is full.
pub struct HeaplessProducer<'a, const FRAME_MAX: usize, const DEPTH: usize> {
    inner: Producer<'a, Vec<u8, FRAME_MAX>, DEPTH>,
}

impl<'a, const FRAME_MAX: usize, const DEPTH: usize> HeaplessProducer<'a, FRAME_MAX, DEPTH> {
    /// Whether the ring has room for another frame.
    pub fn ready(&self) -> bool {
        self.inner.ready()
    }
}

impl<'a, const FRAME_MAX: usize, const DEPTH: usize> FrameSink for HeaplessProducer<'a, FRAME_MAX, DEPTH> {
    type Error = RingError;

    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut v: Vec<u8, FRAME_MAX> = Vec::new();
        v.extend_from_slice(bytes).map_err(|_| RingError::FrameTooLarge)?;
        self.inner.enqueue(v).map_err(|_| RingError::Full)
    }
}

/// The consumer half: pops frames in the order they were sent.
pub struct HeaplessConsumer<'a, const FRAME_MAX: usize, const DEPTH: usize> {
    inner: Consumer<'a, Vec<u8, FRAME_MAX>, DEPTH>,
}

impl<'a, const FRAME_MAX: usize, const DEPTH: usize> HeaplessConsumer<'a, FRAME_MAX, DEPTH> {
    /// The next frame, if one is waiting.
    pub fn pop(&mut self) -> Option<Vec<u8, FRAME_MAX>> {
        self.inner.dequeue()
    }

    /// Whether a frame is waiting.
    pub fn ready(&self) -> bool {
        self.inner.ready()
    }
}
