//! Push-rules projection index (P9.2 harness foundation).
//!
//! Tracks underride / override / content / room / sender rules as product
//! projections with string actions only. **No raw rule JSON dumps**, no
//! tokens. Host maps SDK push rules → this shape. No dual-backend.

use std::collections::HashMap;

use super::error::PushRulesError;

/// Soft caps.
pub const MAX_RULES: usize = 512;
pub const MAX_RULE_ID_CHARS: usize = 256;
pub const MAX_PATTERN_CHARS: usize = 512;
pub const MAX_ACTIONS: usize = 8;

/// Rule kind / kind-list (product enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PushRuleKind {
    Override,
    Content,
    Room,
    Sender,
    Underride,
}

impl PushRuleKind {
    pub const ALL: &'static [PushRuleKind] = &[
        Self::Override,
        Self::Content,
        Self::Room,
        Self::Sender,
        Self::Underride,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Override => "override",
            Self::Content => "content",
            Self::Room => "room",
            Self::Sender => "sender",
            Self::Underride => "underride",
        }
    }
}

/// High-level action projection (product; not full Matrix action objects).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PushAction {
    Notify,
    DontNotify,
    Coalesce,
    SetTweakHighlight,
    SetTweakSound,
}

impl PushAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Notify => "notify",
            Self::DontNotify => "dont_notify",
            Self::Coalesce => "coalesce",
            Self::SetTweakHighlight => "set_tweak_highlight",
            Self::SetTweakSound => "set_tweak_sound",
        }
    }
}

/// One push rule projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRule {
    pub rule_id: String,
    pub kind: PushRuleKind,
    pub enabled: bool,
    pub default: bool,
    /// Optional content pattern (glob-like string only).
    pub pattern: Option<String>,
    pub actions: Vec<PushAction>,
}

/// Session-generation-stamped push-rules index.
#[derive(Debug, Default)]
pub struct PushRulesIndex {
    session_generation: u64,
    /// rule_id → rule
    rules: HashMap<String, PushRule>,
    /// global master switch (m.rule.master style).
    global_enabled: bool,
}

impl PushRulesIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            rules: HashMap::new(),
            global_enabled: true,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn global_enabled(&self) -> bool {
        self.global_enabled
    }

    pub fn set_global_enabled(&mut self, enabled: bool) {
        self.global_enabled = enabled;
    }

    pub fn upsert(&mut self, rule: PushRule) -> Result<(), PushRulesError> {
        validate_rule(&rule)?;
        if !self.rules.contains_key(&rule.rule_id) && self.rules.len() >= MAX_RULES {
            return Err(PushRulesError::Invalid {
                diagnostic_id: "p9.2-rule-cap",
            });
        }
        self.rules.insert(rule.rule_id.clone(), rule);
        Ok(())
    }

    pub fn get(&self, rule_id: &str) -> Option<&PushRule> {
        self.rules.get(rule_id)
    }

    pub fn remove(&mut self, rule_id: &str) -> bool {
        self.rules.remove(rule_id).is_some()
    }

    pub fn set_enabled(&mut self, rule_id: &str, enabled: bool) -> Result<(), PushRulesError> {
        let r = self
            .rules
            .get_mut(rule_id)
            .ok_or(PushRulesError::NotFound {
                diagnostic_id: "p9.2-rule-not-found",
            })?;
        r.enabled = enabled;
        Ok(())
    }

    /// Rules of one kind, sorted by rule_id.
    pub fn list_kind(&self, kind: PushRuleKind) -> Vec<&PushRule> {
        let mut v: Vec<_> = self.rules.values().filter(|r| r.kind == kind).collect();
        v.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
        v
    }

    /// Whether any enabled rule would request notify (simplified).
    pub fn any_notify_enabled(&self) -> bool {
        if !self.global_enabled {
            return false;
        }
        self.rules.values().any(|r| {
            r.enabled
                && r.actions
                    .iter()
                    .any(|a| matches!(a, PushAction::Notify | PushAction::Coalesce))
        })
    }

    pub fn clear(&mut self) {
        self.rules.clear();
        self.global_enabled = true;
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}

fn validate_rule(rule: &PushRule) -> Result<(), PushRulesError> {
    if rule.rule_id.is_empty() || rule.rule_id.chars().count() > MAX_RULE_ID_CHARS {
        return Err(PushRulesError::Invalid {
            diagnostic_id: "p9.2-invalid-rule-id",
        });
    }
    let lower = rule.rule_id.to_ascii_lowercase();
    if lower.contains("access_token") || lower.contains("refresh_token") {
        return Err(PushRulesError::Invalid {
            diagnostic_id: "p9.2-forbidden-rule-id",
        });
    }
    if let Some(ref p) = rule.pattern {
        if p.chars().count() > MAX_PATTERN_CHARS {
            return Err(PushRulesError::Invalid {
                diagnostic_id: "p9.2-pattern-cap",
            });
        }
        let pl = p.to_ascii_lowercase();
        if pl.contains("access_token") {
            return Err(PushRulesError::Invalid {
                diagnostic_id: "p9.2-forbidden-pattern",
            });
        }
    }
    if rule.actions.len() > MAX_ACTIONS {
        return Err(PushRulesError::Invalid {
            diagnostic_id: "p9.2-actions-cap",
        });
    }
    Ok(())
}
