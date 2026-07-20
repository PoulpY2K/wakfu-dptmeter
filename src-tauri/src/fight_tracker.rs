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
            LogEvent::FightCreationDetected => {
                self.fight_id = None;
                self.participants.clear();
                self.summon_owner.clear();
                self.current_caster = None;
                Vec::new()
            }
            LogEvent::FighterJoined {
                fight_id,
                name,
                entity_id,
                is_controlled_by_ai,
            } => {
                let mut events = Vec::new();
                if self.fight_id.is_none() {
                    self.fight_id = Some(fight_id);
                    events.push(FightEvent::FightStarted { fight_id });
                }

                let side = if is_controlled_by_ai {
                    Side::Enemy
                } else {
                    Side::Player
                };
                self.participants.insert(
                    entity_id,
                    Combatant {
                        name: name.clone(),
                        entity_id,
                        side,
                    },
                );
                events.push(FightEvent::CombatantIdentified {
                    fight_id,
                    name,
                    entity_id,
                    side,
                });
                events
            }
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

    #[test]
    fn fight_creation_then_two_fighters_join_emits_start_and_identification_events() {
        let mut tracker = FightTracker::new();

        let creation_events = tracker.process(LogEvent::FightCreationDetected);
        assert_eq!(creation_events, Vec::new());

        let enemy_events = tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Soeur Zerker".to_string(),
            entity_id: -1724034221200073,
            is_controlled_by_ai: true,
        });
        assert_eq!(
            enemy_events,
            vec![
                FightEvent::FightStarted { fight_id: 1568151141 },
                FightEvent::CombatantIdentified {
                    fight_id: 1568151141,
                    name: "Soeur Zerker".to_string(),
                    entity_id: -1724034221200073,
                    side: Side::Enemy,
                },
            ]
        );

        let player_events = tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Blampy".to_string(),
            entity_id: 5547447,
            is_controlled_by_ai: false,
        });
        assert_eq!(
            player_events,
            vec![FightEvent::CombatantIdentified {
                fight_id: 1568151141,
                name: "Blampy".to_string(),
                entity_id: 5547447,
                side: Side::Player,
            }]
        );
    }
}
