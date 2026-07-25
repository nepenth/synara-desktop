//! PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
//! ruma type leaked into IPC wire module.

use ruma::OwnedUserId;

pub fn as_user(id: OwnedUserId) -> String {
    id.to_string()
}
