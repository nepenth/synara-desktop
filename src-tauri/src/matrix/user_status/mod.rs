//! Desktop re-export of Core MSC4426 user status / in-call.
//!
//! Own writes are `set_status` / `clear_status` only. This module never
//! calls `set_call` and never enables `enable_automatic_call_status`.

pub use synara_core::app::user_status::*;
