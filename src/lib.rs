//! # ABUT
//! Structural IPC orchestration and boundary-aware Unix Domain Sockets.
//! 
//! To "abut" is to share a common boundary. This crate provides the 
//! primitives for two isolated processes to lean against one another 
//! via a UDS interface without compromising their individual security scopes.
//!
//! ## Core Security Principles
//! * **Adjacency:** Manages the physical and logical "touch points" between 
//!   the sidecar and the host application.
//! * **Boundary Integrity:** Ensures that while processes may abut, their 
//!   memory allotments and `classified` contents never intermingle.
//! * **Deterministic Junctions:** Uses `scope` to ensure that the IPC 
//!   junction is severed immediately upon task completion.
//! 


pub mod error;
pub mod frame;
pub mod traits;
pub mod types;

pub use error::*;
pub use traits::*;
pub use types::*;