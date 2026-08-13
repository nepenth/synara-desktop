//! Pure space hierarchy graph over product [`SpaceSummary`] DTOs (P4.5).
//!
//! No SDK SpaceService types cross this boundary. Live hierarchy sync residual.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::dto::{RoomId, RoomSummary, SpaceChild, SpaceSummary};

use super::error::SpaceError;

/// Indexed space hierarchy for filter / parent queries.
#[derive(Debug, Clone, Default)]
pub struct SpaceHierarchy {
    spaces: HashMap<RoomId, SpaceSummary>,
}

impl SpaceHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install or replace the full space catalog (snapshot).
    pub fn replace_all(&mut self, spaces: Vec<SpaceSummary>) -> Result<(), SpaceError> {
        let mut map = HashMap::new();
        for s in spaces {
            if s.room_id.trim().is_empty() || !s.room_id.starts_with('!') {
                return Err(SpaceError::Invalid {
                    diagnostic_id: "p4.5-invalid-space-id",
                });
            }
            map.insert(s.room_id.clone(), s);
        }
        // Cycle check on parent edges among known spaces.
        if has_parent_cycle(&map) {
            return Err(SpaceError::Cycle {
                diagnostic_id: "p4.5-space-parent-cycle",
            });
        }
        self.spaces = map;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.spaces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spaces.is_empty()
    }

    pub fn get(&self, space_id: &str) -> Option<&SpaceSummary> {
        self.spaces.get(space_id)
    }

    pub fn all_spaces(&self) -> Vec<&SpaceSummary> {
        let mut v: Vec<_> = self.spaces.values().collect();
        v.sort_by(|a, b| a.room_id.cmp(&b.room_id));
        v
    }

    /// Direct child room ids of a space (ordered by optional `order` then room_id).
    pub fn direct_child_ids(&self, space_id: &str) -> Result<Vec<RoomId>, SpaceError> {
        let space = self.spaces.get(space_id).ok_or(SpaceError::NotFound {
            diagnostic_id: "p4.5-space-not-found",
        })?;
        let mut children = space.children.clone();
        children.sort_by(|a, b| match (&a.order, &b.order) {
            (Some(oa), Some(ob)) => oa.cmp(ob).then_with(|| a.room_id.cmp(&b.room_id)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.room_id.cmp(&b.room_id),
        });
        Ok(children.into_iter().map(|c| c.room_id).collect())
    }

    /// All descendant room ids (BFS), including nested spaces' children.
    pub fn descendant_room_ids(&self, space_id: &str) -> Result<Vec<RoomId>, SpaceError> {
        if !self.spaces.contains_key(space_id) {
            return Err(SpaceError::NotFound {
                diagnostic_id: "p4.5-space-not-found",
            });
        }
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut q = VecDeque::new();
        q.push_back(space_id.to_owned());
        seen.insert(space_id.to_owned());
        while let Some(id) = q.pop_front() {
            if let Some(space) = self.spaces.get(&id) {
                for child in &space.children {
                    if seen.insert(child.room_id.clone()) {
                        out.push(child.room_id.clone());
                        if self.spaces.contains_key(&child.room_id) {
                            q.push_back(child.room_id.clone());
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// Root spaces: no parent listed, or parents not present in catalog.
    pub fn root_spaces(&self) -> Vec<&SpaceSummary> {
        let mut roots: Vec<_> = self
            .spaces
            .values()
            .filter(|s| match &s.parent_room_ids {
                None => true,
                Some(parents) if parents.is_empty() => true,
                Some(parents) => parents.iter().all(|p| !self.spaces.contains_key(p)),
            })
            .collect();
        roots.sort_by(|a, b| a.room_id.cmp(&b.room_id));
        roots
    }

    /// Filter room list to rooms that are descendants of `space_id` (not the space itself).
    pub fn filter_rooms_in_space(
        &self,
        space_id: &str,
        rooms: &[RoomSummary],
    ) -> Result<Vec<RoomSummary>, SpaceError> {
        let descendants: HashSet<_> = self.descendant_room_ids(space_id)?.into_iter().collect();
        Ok(rooms
            .iter()
            .filter(|r| descendants.contains(&r.room_id))
            .cloned()
            .collect())
    }
}

fn has_parent_cycle(spaces: &HashMap<RoomId, SpaceSummary>) -> bool {
    // Detect cycles only among space nodes using parent_room_ids edges.
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    fn dfs(
        id: &str,
        spaces: &HashMap<RoomId, SpaceSummary>,
        visiting: &mut HashSet<RoomId>,
        visited: &mut HashSet<RoomId>,
    ) -> bool {
        if visited.contains(id) {
            return false;
        }
        if !visiting.insert(id.to_owned()) {
            return true;
        }
        if let Some(space) = spaces.get(id) {
            if let Some(parents) = &space.parent_room_ids {
                for p in parents {
                    if spaces.contains_key(p) && dfs(p, spaces, visiting, visited) {
                        return true;
                    }
                }
            }
        }
        visiting.remove(id);
        visited.insert(id.to_owned());
        false
    }

    for id in spaces.keys() {
        if dfs(id, spaces, &mut visiting, &mut visited) {
            return true;
        }
    }
    false
}

/// Helper to build a child edge for harness tests.
pub fn space_child(room_id: impl Into<String>, order: Option<&str>) -> SpaceChild {
    SpaceChild {
        room_id: room_id.into(),
        order: order.map(|s| s.to_owned()),
        suggested: None,
    }
}
