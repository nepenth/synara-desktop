//! P8.3 — Verification request inbox + SAS display foundation (harness).

pub use synara_core::app::verification::*;

pub mod live;
pub use live::start as start_verification_owner;
