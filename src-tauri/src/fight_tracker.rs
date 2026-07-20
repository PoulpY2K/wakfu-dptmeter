use crate::log_parser::LogEvent;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Side {
    Player,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ActionKind {
    Damage,
    Heal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum FightEvent {
    FightStarted {
        fight_id: u64,
    },
    CombatantIdentified {
        fight_id: u64,
        name: String,
        entity_id: i64,
        side: Side,
    },
    ActionRecorded {
        fight_id: u64,
        source: String,
        target: String,
        amount: i32,
        kind: ActionKind,
        element: Option<String>,
    },
    FightEnded {
        fight_id: u64,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Combatant {
    name: String,
    entity_id: i64,
    side: Side,
}

#[derive(Debug, Default)]
pub struct FightTracker {
    fight_id: Option<u64>,
    participants: HashMap<i64, Combatant>,
    summon_owner: HashMap<String, i64>,
    current_caster: Option<i64>,
}

impl FightTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, event: LogEvent) -> Vec<FightEvent> {
        match event {
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_parser::LogEvent;

    #[test]
    fn process_returns_no_events_for_unrecognized_line() {
        let mut tracker = FightTracker::new();
        let events = tracker.process(LogEvent::Unrecognized);
        assert_eq!(events, Vec::new());
    }
}
