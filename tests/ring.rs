//! The bounded frame ring: capacity, limits, ordering, and a threaded handoff.

#![cfg(feature = "ring-heapless")]

use abut::ring::{HeaplessRing, RingError};
use abut::FrameSink;
use std::thread;

#[test]
fn frames_come_out_in_the_order_they_went_in() {
    let mut ring: HeaplessRing<32, 4> = HeaplessRing::new();
    let (mut p, mut c) = ring.split();
    assert!(c.pop().is_none());
    p.send_frame(b"first").unwrap();
    p.send_frame(b"").unwrap();
    p.send_frame(b"third").unwrap();
    assert_eq!(c.pop().unwrap().as_slice(), b"first");
    assert_eq!(c.pop().unwrap().len(), 0);
    assert_eq!(c.pop().unwrap().as_slice(), b"third");
    assert!(c.pop().is_none());
}

#[test]
fn capacity_is_depth_minus_one_and_full_is_an_error_not_a_loss() {
    assert_eq!(HeaplessRing::<8, 4>::capacity(), 3);
    assert_eq!(HeaplessRing::<8, 4>::frame_max(), 8);
    let mut ring: HeaplessRing<8, 4> = HeaplessRing::new();
    let (mut p, mut c) = ring.split();
    for i in 0..3u8 {
        assert!(p.ready());
        p.send_frame(&[i]).unwrap();
    }
    assert!(!p.ready());
    assert_eq!(p.send_frame(&[9]), Err(RingError::Full));
    assert_eq!(c.pop().unwrap().as_slice(), &[0]);
    p.send_frame(&[9]).unwrap();
    let rest: Vec<u8> = std::iter::from_fn(|| c.pop()).map(|f| f[0]).collect();
    assert_eq!(rest, [1, 2, 9]);
}

#[test]
fn a_frame_longer_than_frame_max_is_refused() {
    let mut ring: HeaplessRing<4, 2> = HeaplessRing::new();
    let (mut p, mut c) = ring.split();
    assert_eq!(p.send_frame(&[1, 2, 3, 4, 5]), Err(RingError::FrameTooLarge));
    assert!(c.pop().is_none(), "nothing was enqueued");
    p.send_frame(&[1, 2, 3, 4]).unwrap();
    assert_eq!(c.pop().unwrap().as_slice(), &[1, 2, 3, 4]);
}

#[test]
fn the_ring_wraps_around_cleanly_and_frames_do_not_bleed() {
    let mut ring: HeaplessRing<8, 4> = HeaplessRing::new();
    let (mut p, mut c) = ring.split();
    for i in 0..100u8 {
        p.send_frame(&[i; 8]).unwrap();
        assert_eq!(c.pop().unwrap().as_slice(), &[i; 8]);
        p.send_frame(&[i]).unwrap();
        let short = c.pop().unwrap();
        assert_eq!(short.as_slice(), &[i], "no bytes from the previous full frame");
    }
}

#[test]
fn errors_render_with_their_codes() {
    assert_eq!(RingError::Full.to_string(), "[RING0002] Ring full");
    assert_eq!(RingError::FrameTooLarge.to_string(), "[RING0001] Frame too large for this ring");
}

#[test]
fn a_producer_thread_and_a_consumer_thread_hand_off_every_frame() {
    let mut ring: HeaplessRing<64, 16> = HeaplessRing::new();
    let (mut p, mut c) = ring.split();
    thread::scope(|s| {
        s.spawn(move || {
            for i in 0..2000u32 {
                let frame = format!("frame-{i}");
                while let Err(RingError::Full) = p.send_frame(frame.as_bytes()) {
                    thread::yield_now();
                }
            }
        });
        s.spawn(move || {
            let mut next = 0u32;
            while next < 2000 {
                match c.pop() {
                    Some(frame) => {
                        assert_eq!(frame.as_slice(), format!("frame-{next}").as_bytes());
                        next += 1;
                    }
                    None => thread::yield_now(),
                }
            }
        });
    });
}
