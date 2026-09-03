//! A producer thread hands whole frames to a consumer thread through a
//! bounded, allocation-free ring, then the consumer drains it to a framed
//! writer. This is the shape of a hot path feeding a slower flush loop.
//!
//! Run: `cargo run --example ring --features ring-heapless`

use abut::frame::FramedWriter;
use abut::ring::{HeaplessRing, RingError};
use abut::FrameSink;
use std::thread;

fn main() {
    // 64 frames of at most 32 bytes each, on the stack.
    let mut ring: HeaplessRing<32, 64> = HeaplessRing::new();
    let (mut producer, mut consumer) = ring.split();

    let mut out = FramedWriter::new(Vec::new());

    thread::scope(|s| {
        s.spawn(move || {
            for i in 0..500u32 {
                let frame = format!("sample-{i}");
                // A full ring is back-pressure, not an error: wait for the consumer.
                while let Err(RingError::Full) = producer.send_frame(frame.as_bytes()) {
                    thread::yield_now();
                }
            }
        });

        let mut received = 0;
        while received < 500 {
            match consumer.pop() {
                Some(frame) => {
                    out.write_frame(&frame).expect("write to the vec");
                    received += 1;
                }
                None => thread::yield_now(),
            }
        }
    });

    let bytes = out.into_inner();
    println!("500 frames drained into {} framed bytes", bytes.len());
}
