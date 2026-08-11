//! Wire-safe counter bounds for Matrix IPC (R0.3 / REV-004).
//!
//! Session generations and sequence numbers are JSON numbers on the wire.
//! JavaScript `number` is IEEE-754 double and only preserves integers up to
//! `Number.MAX_SAFE_INTEGER` (2^53 − 1). Rust therefore freezes the same upper
//! bound for all envelope/stream counters and uses checked increments so
//! `last + 1` never overflows or silently wraps.

use serde::{Deserialize, Deserializer, Serializer};

/// Maximum inclusive wire counter (`Number.MAX_SAFE_INTEGER` / 2^53 − 1).
pub const MAX_WIRE_COUNTER: u64 = 9_007_199_254_740_991;

/// True when `n` is a non-negative integer representable exactly in JS `number`.
#[inline]
pub fn is_valid_wire_counter(n: u64) -> bool {
    n <= MAX_WIRE_COUNTER
}

/// Checked successor of a wire counter, or `None` if it would leave the safe range.
#[inline]
pub fn checked_next_wire_counter(last: u64) -> Option<u64> {
    last.checked_add(1).filter(|&n| n <= MAX_WIRE_COUNTER)
}

/// Serde: reject JSON numbers outside the wire-safe range (and non-integers).
pub fn deserialize_wire_counter<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let n = u64::deserialize(deserializer)?;
    if is_valid_wire_counter(n) {
        Ok(n)
    } else {
        Err(serde::de::Error::custom(format!(
            "wire counter {n} exceeds MAX_WIRE_COUNTER ({MAX_WIRE_COUNTER})"
        )))
    }
}

/// Serde for optional wire counters.
pub fn deserialize_optional_wire_counter<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<u64>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(n) if is_valid_wire_counter(n) => Ok(Some(n)),
        Some(n) => Err(serde::de::Error::custom(format!(
            "wire counter {n} exceeds MAX_WIRE_COUNTER ({MAX_WIRE_COUNTER})"
        ))),
    }
}

/// Serialize a wire counter (always a JSON number).
pub fn serialize_wire_counter<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if !is_valid_wire_counter(*value) {
        return Err(serde::ser::Error::custom(format!(
            "wire counter {value} exceeds MAX_WIRE_COUNTER ({MAX_WIRE_COUNTER})"
        )));
    }
    serializer.serialize_u64(*value)
}

#[cfg(test)]
mod wire_counter_unit_tests {
    use super::*;

    #[test]
    fn max_matches_js_safe_integer() {
        assert_eq!(MAX_WIRE_COUNTER, (1u64 << 53) - 1);
        assert!(is_valid_wire_counter(0));
        assert!(is_valid_wire_counter(MAX_WIRE_COUNTER));
        assert!(!is_valid_wire_counter(MAX_WIRE_COUNTER + 1));
    }

    #[test]
    fn checked_next_at_boundary() {
        assert_eq!(checked_next_wire_counter(0), Some(1));
        assert_eq!(
            checked_next_wire_counter(MAX_WIRE_COUNTER - 1),
            Some(MAX_WIRE_COUNTER)
        );
        assert_eq!(checked_next_wire_counter(MAX_WIRE_COUNTER), None);
    }
}
