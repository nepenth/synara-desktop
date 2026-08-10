//! PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
//! matrix_sdk type leaked into domain DTO wire module.

use matrix_sdk::ruma::OwnedRoomId;

pub struct LeakyRoom {
    pub id: OwnedRoomId,
}
