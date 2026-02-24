//! Fixed-capacity ring buffer helpers (no_std-friendly).
//!
//! Enable with feature: `ring-heapless`.
//!
//! This module provides a bounded SPSC queue that stores *complete telemetry frames*.
//! It is useful when you want:
//! - zero allocations (heapless)
//! - bounded memory usage
//! - a hot-path producer (stage/task) handing off to a drain/flush loop

#![cfg(feature = "ring-heapless")]

use crate::traits::FrameSink;

use heapless::spsc::{Consumer, Producer, Queue};
use heapless::Vec;

// /// Errors for heapless ring operations.
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum RingError {
//     /// The provided frame is larger than the configured per-frame capacity.
//     FrameTooLarge,
//     /// The ring is full.
//     Full,
// }
use liaise::{Liaise, LiaiseCodes};

#[derive(LiaiseCodes, Debug)]
#[liaise(prefix = "RING")]
pub enum RingError {
    #[liaise(code = 0, msg = "Frame too large")]
    FrameTooLarge,
    
    #[liaise(code = 1, msg = "Full")]
    Full,
}

/// An owned heapless ring you can place in a struct and then `split()`.
///
/// - `FRAME_MAX`: maximum bytes per frame
/// - `DEPTH`: number of frames buffered
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Split into producer/consumer halves.
    pub fn split(&mut self) -> (
        HeaplessProducer<'_, FRAME_MAX, DEPTH>,
        HeaplessConsumer<'_, FRAME_MAX, DEPTH>,
    ) {
        let (p, c) = self.q.split();
        (HeaplessProducer { inner: p }, HeaplessConsumer { inner: c })
    }
}

/// Producer side: implements `TelemetrySink`.
// #[derive(Debug)]
pub struct HeaplessProducer<'a, const FRAME_MAX: usize, const DEPTH: usize> {
    inner: Producer<'a, Vec<u8, FRAME_MAX>, DEPTH>,
}

impl<'a, const FRAME_MAX: usize, const DEPTH: usize> FrameSink
    for HeaplessProducer<'a, FRAME_MAX, DEPTH>
{
    type Error = RingError;

    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        if bytes.len() > FRAME_MAX {
            return Err(RingError::FrameTooLarge);
        }
        let mut v: Vec<u8, FRAME_MAX> = Vec::new();
        v.extend_from_slice(bytes).map_err(|_| RingError::FrameTooLarge)?;
        self.inner.enqueue(v).map_err(|_| RingError::Full)?;
        Ok(())
    }
}

/// Consumer side.
// #[derive(Debug)]
pub struct HeaplessConsumer<'a, const FRAME_MAX: usize, const DEPTH: usize> {
    inner: Consumer<'a, Vec<u8, FRAME_MAX>, DEPTH>,
}

impl<'a, const FRAME_MAX: usize, const DEPTH: usize> HeaplessConsumer<'a, FRAME_MAX, DEPTH> {
    /// Pop the next frame, if available.
    pub fn pop(&mut self) -> Option<Vec<u8, FRAME_MAX>> {
        self.inner.dequeue()
    }
}
