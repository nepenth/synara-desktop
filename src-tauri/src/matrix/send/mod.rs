//! P6.1 / P7.4 send-queue harness re-export.
#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::send::*;

#[cfg(test)]
mod live_synapse_proof;
