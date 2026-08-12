//! Pure cold-start room activity recovery gate.
//!
//! This scalar policy deliberately receives no room, timeline, or SDK value.
//! The iOS shell retains the observations and execution details.

/// Closed classification of the prior activity observation.
///
/// `Missing` includes the shell's absent and sentinel-distant-past states;
/// `Known` means the shell observed actual prior activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomActivityPreviousState {
    Missing,
    Known,
}

/// Whether cold-start activity recovery is required.
///
/// A recovery runs only when the newest activity needs recovery and no prior
/// activity is known. This total calculation has no state, I/O, or side effect.
pub fn room_activity_recovery_required(
    latest_requires_recovery: bool,
    previous_state: RoomActivityPreviousState,
) -> bool {
    latest_requires_recovery && previous_state == RoomActivityPreviousState::Missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_start_recovery_truth_table_is_exhaustive() {
        for (latest_requires_recovery, previous_state, expected) in [
            (false, RoomActivityPreviousState::Missing, false),
            (false, RoomActivityPreviousState::Known, false),
            (true, RoomActivityPreviousState::Missing, true),
            (true, RoomActivityPreviousState::Known, false),
        ] {
            assert_eq!(
                room_activity_recovery_required(latest_requires_recovery, previous_state),
                expected
            );
        }
    }
}
