//! The key's total order, its stored forms, and the peer exchange decisions:
//! devices that each created a key must end up holding the same one.

use std::time::Duration;

use matrix_sdk::test_utils::mocks::MatrixMockServer;

use super::*;

fn held(generation: u32) -> HeldKey {
    HeldKey {
        key: DraftsKey::generate(),
        generation,
    }
}

#[test]
fn the_higher_generation_wins_then_the_lower_key_id() {
    let low = KeyRank {
        generation: 0,
        key_id: "AAAA".into(),
    };
    let high = KeyRank {
        generation: 0,
        key_id: "BBBB".into(),
    };
    let newer = KeyRank {
        generation: 1,
        key_id: "ZZZZ".into(),
    };
    assert!(low.beats(&high));
    assert!(!high.beats(&low));
    assert!(newer.beats(&low));
    assert!(!low.beats(&newer));
    assert!(!low.beats(&low));
}

#[test]
fn preferred_is_the_same_key_whichever_side_asks() {
    for _ in 0..32 {
        let (a, b) = (held(0), held(0));
        assert_eq!(
            a.clone().preferred(b.clone()),
            b.clone().preferred(a.clone())
        );
        let reset = held(1);
        assert_eq!(a.clone().preferred(reset.clone()), reset);
        assert_eq!(reset.clone().preferred(a), reset);
    }
}

#[test]
fn stored_forms_round_trip_and_generation_zero_keeps_the_old_form() {
    let first = held(0);
    assert_eq!(first.to_secret().as_str(), first.key.to_base64().as_str());
    assert_eq!(first.to_cache_bytes().len(), DRAFTS_KEY_BYTES);
    assert_eq!(HeldKey::from_secret(&first.to_secret()).unwrap(), first);
    assert_eq!(
        HeldKey::from_cache_bytes(&first.to_cache_bytes()).unwrap(),
        first
    );

    let later = held(3);
    assert!(later.to_secret().ends_with(".3"));
    assert_eq!(HeldKey::from_secret(&later.to_secret()).unwrap(), later);
    assert_eq!(
        HeldKey::from_cache_bytes(&later.to_cache_bytes()).unwrap(),
        later
    );

    assert!(HeldKey::from_secret("not a key").is_err());
    assert!(HeldKey::from_secret(&format!("{}.x", first.key.to_base64().as_str())).is_err());
    assert!(HeldKey::from_cache_bytes(&[0u8; 7]).is_none());
}

#[test]
fn a_request_is_answered_only_by_a_key_that_wins() {
    let (a, b) = (held(0).rank(), held(0).rank());
    let (winner, loser) = if a.beats(&b) { (a, b) } else { (b, a) };

    assert_eq!(decide_request(None, None), RequestDecision::Ignore);
    assert_eq!(decide_request(None, Some(&winner)), RequestDecision::Ignore);
    assert_eq!(decide_request(Some(&loser), None), RequestDecision::Answer);
    assert_eq!(
        decide_request(Some(&winner), Some(&loser)),
        RequestDecision::Answer
    );
    assert_eq!(
        decide_request(Some(&loser), Some(&winner)),
        RequestDecision::AskBack
    );
    assert_eq!(
        decide_request(Some(&winner), Some(&winner)),
        RequestDecision::Ignore
    );
}

#[test]
fn a_share_is_adopted_only_when_it_wins_and_is_not_retired() {
    let (a, b) = (held(0).rank(), held(0).rank());
    let (winner, loser) = if a.beats(&b) { (a, b) } else { (b, a) };

    assert_eq!(decide_share(None, &loser, 0), ShareDecision::Adopt);
    assert_eq!(decide_share(Some(&loser), &winner, 0), ShareDecision::Adopt);
    assert_eq!(
        decide_share(Some(&winner), &loser, 0),
        ShareDecision::AskBack
    );
    assert_eq!(
        decide_share(Some(&winner), &winner, 0),
        ShareDecision::Ignore
    );

    // After an identity reset on this device (floor 1) the old key is never
    // taken back, with or without a new key of our own.
    assert_eq!(decide_share(None, &winner, 1), ShareDecision::Ignore);
    let ours = held(1).rank();
    assert_eq!(
        decide_share(Some(&ours), &winner, 1),
        ShareDecision::AskBack
    );
}

/// Every device announces its key to every other until nothing changes. The
/// exchange uses only the two decision functions, as the live handlers do.
fn settle(devices: &mut [Option<KeyRank>]) {
    for _ in 0..devices.len() + 1 {
        let mut changed = false;
        for asker in 0..devices.len() {
            for peer in 0..devices.len() {
                if asker == peer {
                    continue;
                }
                let (mine, theirs) = (devices[peer].clone(), devices[asker].clone());
                match decide_request(mine.as_ref(), theirs.as_ref()) {
                    RequestDecision::Answer => {
                        let shared = mine.expect("an answering device holds a key");
                        if decide_share(theirs.as_ref(), &shared, 0) == ShareDecision::Adopt {
                            devices[asker] = Some(shared);
                            changed = true;
                        }
                    }
                    RequestDecision::AskBack => {
                        // The peer asks back and the asker answers.
                        let shared = theirs.expect("only a held key wins");
                        if decide_share(mine.as_ref(), &shared, 0) == ShareDecision::Adopt {
                            devices[peer] = Some(shared);
                            changed = true;
                        }
                    }
                    RequestDecision::Ignore => {}
                }
            }
        }
        if !changed {
            return;
        }
    }
    panic!("devices did not settle");
}

#[test]
fn devices_that_each_created_a_key_settle_on_one() {
    for _ in 0..16 {
        let mut devices = vec![
            Some(held(0).rank()),
            Some(held(0).rank()),
            None,
            Some(held(0).rank()),
            None,
        ];
        settle(&mut devices);
        let first = devices[0].clone().expect("a key");
        assert!(devices.iter().all(|device| device.as_ref() == Some(&first)));
    }
}

#[test]
fn a_key_created_after_an_identity_reset_replaces_the_old_one_everywhere() {
    let mut devices = vec![
        Some(held(0).rank()),
        Some(held(1).rank()),
        Some(held(0).rank()),
    ];
    let reset = devices[1].clone();
    settle(&mut devices);
    assert!(devices.iter().all(|device| *device == reset));
}

#[test]
fn requests_repeat_only_when_the_key_changed_or_the_interval_passed() {
    let state = PeerShareState::default();
    let first = held(0).rank();
    let second = held(0).rank();

    assert!(state.begin_request(None, Some(Duration::from_secs(60))));
    assert!(!state.begin_request(None, Some(Duration::from_secs(60))));
    assert!(state.begin_request(None, Some(Duration::ZERO)));
    // Holding a key now: announced once, then never again for the same key.
    assert!(state.begin_request(Some(&first), None));
    assert!(!state.begin_request(Some(&first), None));
    assert!(state.begin_request(Some(&second), None));
    state.forget_request();
    assert!(state.begin_request(Some(&second), None));
}

#[test]
fn asking_back_is_immediate_for_a_new_winning_key_and_spaced_for_the_same_one() {
    let state = PeerShareState::default();
    let (first, second) = (held(0).rank(), held(0).rank());
    let minute = Duration::from_secs(60);

    assert!(state.begin_ask_back(&first, minute));
    assert!(!state.begin_ask_back(&first, minute));
    // Another key announced right after is asked for at once.
    assert!(state.begin_ask_back(&second, minute));
    assert!(state.begin_ask_back(&second, Duration::ZERO));
}

#[test]
fn several_peers_may_answer_one_request() {
    let state = PeerShareState::default();
    state.remember("abc");
    assert!(state.asked("abc"));
    assert!(state.asked("abc"));
    assert!(!state.asked("other"));
}

#[tokio::test]
async fn an_identity_reset_forgets_the_key_and_raises_the_floor() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    assert_eq!(load_floor(&client).await, 0);

    let first = held(0);
    store_held_key(&client, &first).await.unwrap();
    assert_eq!(load_held_key(&client).await, Some(first));

    on_identity_reset(&client).await;
    assert_eq!(load_held_key(&client).await, None);
    assert_eq!(load_floor(&client).await, 1);

    // A second reset without a key in between keeps the floor.
    on_identity_reset(&client).await;
    assert_eq!(load_floor(&client).await, 1);

    let later = held(1);
    store_held_key(&client, &later).await.unwrap();
    on_identity_reset(&client).await;
    assert_eq!(load_floor(&client).await, 2);
}

#[tokio::test]
async fn an_unverified_device_does_not_create_the_key() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    assert!(!own_device_verified(&client).await);
    assert!(create_key_if_verified(&client).await.is_none());
    assert!(load_held_key(&client).await.is_none());
}
