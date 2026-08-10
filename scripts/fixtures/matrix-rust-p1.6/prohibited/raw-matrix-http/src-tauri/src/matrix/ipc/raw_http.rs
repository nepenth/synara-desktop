//! PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
//! Raw /_matrix/ path literal in product Rust wire/runtime module.

pub fn versions_path() -> &'static str {
    "/_matrix/client/versions"
}
