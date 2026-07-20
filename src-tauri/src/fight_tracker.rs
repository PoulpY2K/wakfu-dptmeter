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

    fn resolve_owner_entity_id(&self, name: &str) -> Option<i64> {
        if let Some(&owner_id) = self.summon_owner.get(name) {
            Some(owner_id)
        } else {
            self.participants
                .values()
                .find(|combatant| combatant.name == name)
                .map(|combatant| combatant.entity_id)
        }
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

                if self.summon_owner.contains_key(&name) {
                    return events;
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
            LogEvent::SummonInvoked {
                owner_name,
                summon_name,
            } => {
                if let Some(owner_entity_id) = self.resolve_owner_entity_id(&owner_name) {
                    self.summon_owner.insert(summon_name, owner_entity_id);
                }
                Vec::new()
            }
            LogEvent::SpellCast { actor_name } => {
                self.current_caster = self.resolve_owner_entity_id(&actor_name);
                Vec::new()
            }
            LogEvent::HpChange {
                name,
                amount,
                element,
                ..
            } => {
                let (Some(fight_id), Some(caster_id)) = (self.fight_id, self.current_caster)
                else {
                    return Vec::new();
                };
                let Some(source) = self
                    .participants
                    .get(&caster_id)
                    .map(|combatant| combatant.name.clone())
                else {
                    return Vec::new();
                };

                let kind = if amount < 0 {
                    ActionKind::Damage
                } else {
                    ActionKind::Heal
                };

                vec![FightEvent::ActionRecorded {
                    fight_id,
                    source,
                    target: name,
                    amount,
                    kind,
                    element,
                }]
            }
            LogEvent::FightEnded { fight_id } => {
                let resolved_fight_id = self.fight_id.unwrap_or(fight_id);
                self.fight_id = None;
                self.participants.clear();
                self.summon_owner.clear();
                self.current_caster = None;
                vec![FightEvent::FightEnded {
                    fight_id: resolved_fight_id,
                }]
            }
            LogEvent::Unrecognized => Vec::new(),
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

    #[test]
    fn summon_invocation_excludes_it_from_combatant_identification() {
        let mut tracker = FightTracker::new();
        tracker.process(LogEvent::FightCreationDetected);

        let blampy_events = tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Blampy".to_string(),
            entity_id: 5547447,
            is_controlled_by_ai: false,
        });
        assert_eq!(blampy_events.len(), 2); // FightStarted + CombatantIdentified

        let summon_invoked_events = tracker.process(LogEvent::SummonInvoked {
            owner_name: "Blampy".to_string(),
            summon_name: "Bombe Aveuglante".to_string(),
        });
        assert_eq!(summon_invoked_events, Vec::new());

        let summon_joined_events = tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Bombe Aveuglante".to_string(),
            entity_id: -1724034220279884,
            is_controlled_by_ai: true,
        });
        assert_eq!(summon_joined_events, Vec::new());
    }

    #[test]
    fn spell_cast_then_hp_change_attributes_damage_to_the_caster() {
        let mut tracker = FightTracker::new();
        tracker.process(LogEvent::FightCreationDetected);
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Blampy".to_string(),
            entity_id: 5547447,
            is_controlled_by_ai: false,
        });
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Soeur Zerker".to_string(),
            entity_id: -1724034221200073,
            is_controlled_by_ai: true,
        });

        let spell_cast_events = tracker.process(LogEvent::SpellCast {
            actor_name: "Blampy".to_string(),
        });
        assert_eq!(spell_cast_events, Vec::new());

        let hp_change_events = tracker.process(LogEvent::HpChange {
            name: "Soeur Zerker".to_string(),
            amount: -1500,
            element: Some("Feu".to_string()),
            is_parried: false,
        });
        assert_eq!(
            hp_change_events,
            vec![FightEvent::ActionRecorded {
                fight_id: 1568151141,
                source: "Blampy".to_string(),
                target: "Soeur Zerker".to_string(),
                amount: -1500,
                kind: ActionKind::Damage,
                element: Some("Feu".to_string()),
            }]
        );
    }

    #[test]
    fn spell_cast_by_a_summon_attributes_damage_to_the_summons_owner() {
        let mut tracker = FightTracker::new();
        tracker.process(LogEvent::FightCreationDetected);
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Blampy".to_string(),
            entity_id: 5547447,
            is_controlled_by_ai: false,
        });
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Soeur Zerker".to_string(),
            entity_id: -1724034221200073,
            is_controlled_by_ai: true,
        });
        tracker.process(LogEvent::SummonInvoked {
            owner_name: "Blampy".to_string(),
            summon_name: "Bombe Aveuglante".to_string(),
        });
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Bombe Aveuglante".to_string(),
            entity_id: -1724034220279884,
            is_controlled_by_ai: true,
        });

        // Cas limite de la spec : une invocation qui lance elle-meme un sort
        // doit attribuer les degats a son proprietaire, pas a l'invocation.
        tracker.process(LogEvent::SpellCast {
            actor_name: "Bombe Aveuglante".to_string(),
        });

        let hp_change_events = tracker.process(LogEvent::HpChange {
            name: "Soeur Zerker".to_string(),
            amount: -300,
            element: None,
            is_parried: false,
        });
        assert_eq!(
            hp_change_events,
            vec![FightEvent::ActionRecorded {
                fight_id: 1568151141,
                source: "Blampy".to_string(),
                target: "Soeur Zerker".to_string(),
                amount: -300,
                kind: ActionKind::Damage,
                element: None,
            }]
        );
    }

    #[test]
    fn positive_hp_change_is_recorded_as_heal() {
        let mut tracker = FightTracker::new();
        tracker.process(LogEvent::FightCreationDetected);
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Marylpy".to_string(),
            entity_id: 11370104,
            is_controlled_by_ai: false,
        });
        tracker.process(LogEvent::SpellCast {
            actor_name: "Marylpy".to_string(),
        });

        let hp_change_events = tracker.process(LogEvent::HpChange {
            name: "Marylpy".to_string(),
            amount: 400,
            element: None,
            is_parried: false,
        });
        assert_eq!(
            hp_change_events,
            vec![FightEvent::ActionRecorded {
                fight_id: 1568151141,
                source: "Marylpy".to_string(),
                target: "Marylpy".to_string(),
                amount: 400,
                kind: ActionKind::Heal,
                element: None,
            }]
        );
    }

    #[test]
    fn fight_ended_emits_event_and_resets_state_for_the_next_fight() {
        let mut tracker = FightTracker::new();
        tracker.process(LogEvent::FightCreationDetected);
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Blampy".to_string(),
            entity_id: 5547447,
            is_controlled_by_ai: false,
        });

        let ended_events = tracker.process(LogEvent::FightEnded {
            fight_id: 1568151141,
        });
        assert_eq!(
            ended_events,
            vec![FightEvent::FightEnded {
                fight_id: 1568151141
            }]
        );

        // Un nouveau combat doit a nouveau emettre FightStarted : l'etat a bien ete remis a zero.
        tracker.process(LogEvent::FightCreationDetected);
        let next_fight_events = tracker.process(LogEvent::FighterJoined {
            fight_id: 42,
            name: "Distipy".to_string(),
            entity_id: 11370102,
            is_controlled_by_ai: false,
        });
        assert_eq!(
            next_fight_events,
            vec![
                FightEvent::FightStarted { fight_id: 42 },
                FightEvent::CombatantIdentified {
                    fight_id: 42,
                    name: "Distipy".to_string(),
                    entity_id: 11370102,
                    side: Side::Player,
                },
            ]
        );
    }
}
