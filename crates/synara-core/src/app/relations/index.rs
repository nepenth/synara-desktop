//! Relation index — reactions / replaces / references (P5.6 harness foundation).
//!
//! Pure projection over Synara [`RelationRef`] DTOs. No SDK send, no dual-backend.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::dto::{
    EventId, RelationRef, RoomId, TimelineReactionSummaryItem, UserId, REL_TYPE_ANNOTATION,
    REL_TYPE_REFERENCE, REL_TYPE_REPLACE, REL_TYPE_THREAD,
};

use super::error::RelationError;

/// One reaction annotation by a single sender on a target event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AnnotationKey {
    room_id: RoomId,
    target_event_id: EventId,
    key: String,
    sender: UserId,
}

/// Session-generation-stamped relation index.
#[derive(Debug, Default)]
pub struct RelationIndex {
    session_generation: u64,
    /// annotation keys currently present (idempotent set).
    annotations: HashMap<AnnotationKey, RelationRef>,
    /// Latest m.replace relation per (room, target).
    replaces: HashMap<(RoomId, EventId), RelationRef>,
    /// m.reference edges: (room, target) → set of relation event ids (from RelationRef.event_id as target).
    references: HashMap<(RoomId, EventId), BTreeSet<EventId>>,
    /// Thread membership: (room, root) → reply event ids.
    threads: HashMap<(RoomId, EventId), BTreeSet<EventId>>,
}

impl RelationIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            annotations: HashMap::new(),
            replaces: HashMap::new(),
            references: HashMap::new(),
            threads: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
            && self.replaces.is_empty()
            && self.references.is_empty()
            && self.threads.is_empty()
    }

    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }

    fn validate_rel(rel: &RelationRef) -> Result<(), RelationError> {
        if rel.event_id.is_empty() || !rel.event_id.starts_with('$') {
            return Err(RelationError::Invalid {
                diagnostic_id: "p5.6-invalid-target-event-id",
            });
        }
        if rel.rel_type.is_empty() {
            return Err(RelationError::Invalid {
                diagnostic_id: "p5.6-empty-rel-type",
            });
        }
        if let Some(room) = &rel.room_id {
            if room.is_empty() || !room.starts_with('!') {
                return Err(RelationError::Invalid {
                    diagnostic_id: "p5.6-invalid-room-id",
                });
            }
        }
        if let Some(sender) = &rel.sender {
            if sender.is_empty() || !sender.starts_with('@') {
                return Err(RelationError::Invalid {
                    diagnostic_id: "p5.6-invalid-sender",
                });
            }
        }
        Ok(())
    }

    fn room_of(rel: &RelationRef) -> Result<RoomId, RelationError> {
        rel.room_id.clone().ok_or(RelationError::Invalid {
            diagnostic_id: "p5.6-missing-room-id",
        })
    }

    /// Apply one relation. For annotations, requires `sender` + `key`.
    pub fn apply(&mut self, rel: RelationRef) -> Result<(), RelationError> {
        Self::validate_rel(&rel)?;
        let room = Self::room_of(&rel)?;
        match rel.rel_type.as_str() {
            REL_TYPE_ANNOTATION | "m.annotation" => {
                let sender = rel.sender.clone().ok_or(RelationError::Invalid {
                    diagnostic_id: "p5.6-annotation-missing-sender",
                })?;
                let key =
                    rel.key
                        .clone()
                        .filter(|k| !k.is_empty())
                        .ok_or(RelationError::Invalid {
                            diagnostic_id: "p5.6-annotation-missing-key",
                        })?;
                let ak = AnnotationKey {
                    room_id: room,
                    target_event_id: rel.event_id.clone(),
                    key,
                    sender,
                };
                self.annotations.insert(ak, rel);
            }
            REL_TYPE_REPLACE => {
                // Latest write wins for replace of a target event.
                self.replaces.insert((room, rel.event_id.clone()), rel);
            }
            REL_TYPE_REFERENCE => {
                // Treat RelationRef.event_id as target; optional key holds source event id when present.
                let source = rel
                    .key
                    .clone()
                    .filter(|k| k.starts_with('$'))
                    .unwrap_or_else(|| rel.event_id.clone());
                self.references
                    .entry((room, rel.event_id.clone()))
                    .or_default()
                    .insert(source);
            }
            REL_TYPE_THREAD => {
                // event_id = root; key optional reply event id.
                let reply = rel
                    .key
                    .clone()
                    .filter(|k| k.starts_with('$'))
                    .unwrap_or_else(|| rel.event_id.clone());
                self.threads
                    .entry((room, rel.event_id.clone()))
                    .or_default()
                    .insert(reply);
            }
            _ => {
                return Err(RelationError::Invalid {
                    diagnostic_id: "p5.6-unsupported-rel-type",
                });
            }
        }
        Ok(())
    }

    /// Remove one annotation (reaction toggle off).
    pub fn remove_annotation(
        &mut self,
        room_id: &str,
        target_event_id: &str,
        key: &str,
        sender: &str,
    ) -> bool {
        self.annotations
            .remove(&AnnotationKey {
                room_id: room_id.to_owned(),
                target_event_id: target_event_id.to_owned(),
                key: key.to_owned(),
                sender: sender.to_owned(),
            })
            .is_some()
    }

    /// Reaction summaries for one target event, sorted by key.
    pub fn reaction_summaries(
        &self,
        room_id: &str,
        target_event_id: &str,
        local_user: Option<&str>,
    ) -> Vec<TimelineReactionSummaryItem> {
        let mut by_key: BTreeMap<String, (u32, bool)> = BTreeMap::new();
        for ak in self.annotations.keys() {
            if ak.room_id == room_id && ak.target_event_id == target_event_id {
                let entry = by_key.entry(ak.key.clone()).or_insert((0, false));
                entry.0 = entry.0.saturating_add(1);
                if local_user.is_some_and(|u| u == ak.sender) {
                    entry.1 = true;
                }
            }
        }
        by_key
            .into_iter()
            .map(|(key, (count, me))| TimelineReactionSummaryItem {
                item_id: format!("rxn:{target_event_id}:{key}"),
                event_id: target_event_id.to_owned(),
                room_id: room_id.to_owned(),
                key,
                count,
                me: Some(me),
            })
            .collect()
    }

    /// Latest replace relation for a target event, if any.
    pub fn latest_replace(&self, room_id: &str, target_event_id: &str) -> Option<&RelationRef> {
        self.replaces
            .get(&(room_id.to_owned(), target_event_id.to_owned()))
    }

    pub fn reference_count(&self, room_id: &str, target_event_id: &str) -> usize {
        self.references
            .get(&(room_id.to_owned(), target_event_id.to_owned()))
            .map(|s| s.len())
            .unwrap_or(0)
    }

    pub fn thread_reply_count(&self, room_id: &str, root_event_id: &str) -> usize {
        self.threads
            .get(&(room_id.to_owned(), root_event_id.to_owned()))
            .map(|s| s.len())
            .unwrap_or(0)
    }

    pub fn clear_room(&mut self, room_id: &str) {
        self.annotations.retain(|k, _| k.room_id != room_id);
        self.replaces.retain(|(r, _), _| r != room_id);
        self.references.retain(|(r, _), _| r != room_id);
        self.threads.retain(|(r, _), _| r != room_id);
    }

    pub fn clear(&mut self) {
        self.annotations.clear();
        self.replaces.clear();
        self.references.clear();
        self.threads.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}
