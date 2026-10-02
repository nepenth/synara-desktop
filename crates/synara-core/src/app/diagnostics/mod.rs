//! Native diagnostic privacy filters used by client configuration.

mod redact;
pub use redact::*;

#[cfg(test)]
mod tests;
