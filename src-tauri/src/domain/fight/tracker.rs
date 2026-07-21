use std::collections::HashMap;

use super::model::{ActionKind, Combatant, CurrentCast, FightEvent, Side};
use crate::domain::log_parsing::LogEvent;

#[derive(Debug, Default)]
pub struct FightTracker {
    fight_id: Option<u64>,
    participants: HashMap<i64, Combatant>,
    // Most recently joined/invoked entity for a given display name. The
    // Wakfu log only ever refers to actors by name in spell-cast and PV
    // lines (never by entity id), so when two owners have a same-named
    // summon alive at once, the log itself gives no way to tell which
    // instance a later line refers to. Resolving to the most recently
    // invoked entity mirrors the one thing we do know for certain: at the
    // moment of invocation, a summon belongs to whoever just invoked it.
    entity_id_by_name: HashMap<String, i64>,
    // Owner entity id staged by SummonInvoked, consumed by the FighterJoined
    // that immediately follows it for the same summon name.
    pending_summon_owners: HashMap<String, i64>,
    current_cast: Option<CurrentCast>,
}

impl FightTracker {
    pub fn new() -> Self {
        Self::default()
    }

    fn reset(&mut self) {
        self.fight_id = None;
        self.participants.clear();
        self.entity_id_by_name.clear();
        self.pending_summon_owners.clear();
        self.current_cast = None;
    }

    fn resolve_owner_entity_id(&self, name: &str) -> Option<i64> {
        let entity_id = *self.entity_id_by_name.get(name)?;
        let combatant = self.participants.get(&entity_id)?;
        Some(combatant.owner_entity_id.unwrap_or(combatant.entity_id))
    }

    pub fn process(&mut self, event: LogEvent) -> Vec<FightEvent> {
        match event {
            LogEvent::FightCreationDetected => {
                self.reset();
                Vec::new()
            }
            LogEvent::FighterJoined {
                fight_id,
                name,
                entity_id,
                is_controlled_by_ai,
            } => self.handle_fighter_joined(fight_id, name, entity_id, is_controlled_by_ai),
            LogEvent::SummonInvoked {
                owner_name,
                summon_name,
            } => {
                self.handle_summon_invoked(&owner_name, summon_name);
                Vec::new()
            }
            LogEvent::SpellCast {
                actor_name,
                spell_name,
                is_critical,
            } => {
                self.handle_spell_cast(&actor_name, spell_name, is_critical);
                Vec::new()
            }
            LogEvent::HpChange {
                name,
                amount,
                element,
                ..
            } => self.handle_hp_change(name, amount, element),
            LogEvent::FightEnded { fight_id } => self.handle_fight_ended(fight_id),
            LogEvent::Unrecognized => Vec::new(),
        }
    }

    fn handle_fighter_joined(
        &mut self,
        fight_id: u64,
        name: String,
        entity_id: i64,
        is_controlled_by_ai: bool,
    ) -> Vec<FightEvent> {
        let mut events = Vec::new();
        if self.fight_id.is_none() {
            self.fight_id = Some(fight_id);
            events.push(FightEvent::FightStarted { fight_id });
        }

        let owner_entity_id = self.pending_summon_owners.remove(&name);
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
                owner_entity_id,
            },
        );
        self.entity_id_by_name.insert(name.clone(), entity_id);

        if owner_entity_id.is_some() {
            return events;
        }

        events.push(FightEvent::CombatantIdentified {
            fight_id,
            name,
            entity_id,
            side,
        });
        events
    }

    fn handle_summon_invoked(&mut self, owner_name: &str, summon_name: String) {
        if let Some(owner_entity_id) = self.resolve_owner_entity_id(owner_name) {
            self.pending_summon_owners
                .insert(summon_name, owner_entity_id);
        }
    }

    fn handle_spell_cast(&mut self, actor_name: &str, spell_name: String, is_critical: bool) {
        self.current_cast = self
            .resolve_owner_entity_id(actor_name)
            .map(|caster_entity_id| CurrentCast {
                caster_entity_id,
                spell_name,
                is_critical,
            });
    }

    fn handle_hp_change(
        &self,
        name: String,
        amount: i32,
        element: Option<String>,
    ) -> Vec<FightEvent> {
        let (Some(fight_id), Some(current_cast)) = (self.fight_id, self.current_cast.as_ref())
        else {
            return Vec::new();
        };
        let Some(source) = self
            .participants
            .get(&current_cast.caster_entity_id)
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
            spell_name: Some(current_cast.spell_name.clone()),
            is_critical: current_cast.is_critical,
        }]
    }

    fn handle_fight_ended(&mut self, fight_id: u64) -> Vec<FightEvent> {
        let resolved_fight_id = self.fight_id.unwrap_or(fight_id);
        self.reset();
        vec![FightEvent::FightEnded {
            fight_id: resolved_fight_id,
        }]
    }
}

#[cfg(test)]
mod tests {
    // Fight/entity ids below are transcribed verbatim from real Wakfu log
    // lines; adding digit separators would misrepresent the source data.
    #![expect(clippy::unreadable_literal)]

    use super::*;

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
                FightEvent::FightStarted {
                    fight_id: 1568151141
                },
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
            spell_name: "Ruse".to_string(),
            is_critical: false,
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
                spell_name: Some("Ruse".to_string()),
                is_critical: false,
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
            spell_name: "Explosion".to_string(),
            is_critical: false,
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
                spell_name: Some("Explosion".to_string()),
                is_critical: false,
            }]
        );
    }

    #[test]
    fn two_owners_with_same_named_summons_attribute_casts_to_the_most_recently_invoked_owner() {
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
            name: "Distipy".to_string(),
            entity_id: 11370102,
            is_controlled_by_ai: false,
        });
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Soeur Zerker".to_string(),
            entity_id: -1724034221200073,
            is_controlled_by_ai: true,
        });

        // Blampy invokes a bomb first; while it is the only one alive its
        // casts are correctly attributed to Blampy.
        tracker.process(LogEvent::SummonInvoked {
            owner_name: "Blampy".to_string(),
            summon_name: "Bombe Aveuglante".to_string(),
        });
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Bombe Aveuglante".to_string(),
            entity_id: -1,
            is_controlled_by_ai: true,
        });
        tracker.process(LogEvent::SpellCast {
            actor_name: "Bombe Aveuglante".to_string(),
            spell_name: "Explosion".to_string(),
            is_critical: false,
        });
        let blampy_bomb_events = tracker.process(LogEvent::HpChange {
            name: "Soeur Zerker".to_string(),
            amount: -100,
            element: None,
            is_parried: false,
        });
        assert_eq!(
            blampy_bomb_events[0].clone(),
            FightEvent::ActionRecorded {
                fight_id: 1568151141,
                source: "Blampy".to_string(),
                target: "Soeur Zerker".to_string(),
                amount: -100,
                kind: ActionKind::Damage,
                element: None,
                spell_name: Some("Explosion".to_string()),
                is_critical: false,
            }
        );

        // Distipy then invokes a same-named bomb. Once both are alive, the
        // log gives no way to tell them apart by name alone: casts for that
        // name now resolve to whoever invoked most recently.
        tracker.process(LogEvent::SummonInvoked {
            owner_name: "Distipy".to_string(),
            summon_name: "Bombe Aveuglante".to_string(),
        });
        tracker.process(LogEvent::FighterJoined {
            fight_id: 1568151141,
            name: "Bombe Aveuglante".to_string(),
            entity_id: -2,
            is_controlled_by_ai: true,
        });
        tracker.process(LogEvent::SpellCast {
            actor_name: "Bombe Aveuglante".to_string(),
            spell_name: "Explosion".to_string(),
            is_critical: false,
        });
        let distipy_bomb_events = tracker.process(LogEvent::HpChange {
            name: "Soeur Zerker".to_string(),
            amount: -200,
            element: None,
            is_parried: false,
        });
        assert_eq!(
            distipy_bomb_events[0].clone(),
            FightEvent::ActionRecorded {
                fight_id: 1568151141,
                source: "Distipy".to_string(),
                target: "Soeur Zerker".to_string(),
                amount: -200,
                kind: ActionKind::Damage,
                element: None,
                spell_name: Some("Explosion".to_string()),
                is_critical: false,
            }
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
            spell_name: "Mot de soin".to_string(),
            is_critical: false,
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
                spell_name: Some("Mot de soin".to_string()),
                is_critical: false,
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

    #[test]
    fn replays_full_fight_log_and_produces_expected_event_sequence() {
        let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("resources")
            .join("wakfu-one-fight.log");
        let content = std::fs::read_to_string(&log_path)
            .expect("failed to read resources/wakfu-one-fight.log");

        let mut tracker = FightTracker::new();
        let mut events = Vec::new();
        for line in content.lines() {
            let log_event = crate::domain::log_parsing::parse_line(line);
            events.extend(tracker.process(log_event));
        }

        assert_eq!(
            events,
            vec![
                FightEvent::FightStarted {
                    fight_id: 1568151141
                },
                FightEvent::CombatantIdentified {
                    fight_id: 1568151141,
                    name: "Soeur Zerker".to_string(),
                    entity_id: -1724034221200073,
                    side: Side::Enemy,
                },
                FightEvent::CombatantIdentified {
                    fight_id: 1568151141,
                    name: "Blampy".to_string(),
                    entity_id: 5547447,
                    side: Side::Player,
                },
                FightEvent::CombatantIdentified {
                    fight_id: 1568151141,
                    name: "Distipy".to_string(),
                    entity_id: 11370102,
                    side: Side::Player,
                },
                FightEvent::CombatantIdentified {
                    fight_id: 1568151141,
                    name: "Marylpy".to_string(),
                    entity_id: 11370104,
                    side: Side::Player,
                },
                FightEvent::ActionRecorded {
                    fight_id: 1568151141,
                    source: "Soeur Zerker".to_string(),
                    target: "Distipy".to_string(),
                    amount: -892,
                    kind: ActionKind::Damage,
                    element: Some("Air".to_string()),
                    spell_name: Some("Transposition".to_string()),
                    is_critical: false,
                },
                FightEvent::ActionRecorded {
                    fight_id: 1568151141,
                    source: "Soeur Zerker".to_string(),
                    target: "Blampy".to_string(),
                    amount: -1757,
                    kind: ActionKind::Damage,
                    element: Some("Feu".to_string()),
                    spell_name: Some("Châtiment".to_string()),
                    is_critical: true,
                },
                FightEvent::ActionRecorded {
                    fight_id: 1568151141,
                    source: "Distipy".to_string(),
                    target: "Soeur Zerker".to_string(),
                    amount: -1975,
                    kind: ActionKind::Damage,
                    element: Some("Feu".to_string()),
                    spell_name: Some("Flèche explosive".to_string()),
                    is_critical: false,
                },
                FightEvent::ActionRecorded {
                    fight_id: 1568151141,
                    source: "Distipy".to_string(),
                    target: "Soeur Zerker".to_string(),
                    amount: -5465,
                    kind: ActionKind::Damage,
                    element: Some("Feu".to_string()),
                    spell_name: Some("Flèche explosive".to_string()),
                    is_critical: true,
                },
                FightEvent::ActionRecorded {
                    fight_id: 1568151141,
                    source: "Blampy".to_string(),
                    target: "Soeur Zerker".to_string(),
                    amount: -1433,
                    kind: ActionKind::Damage,
                    element: Some("Terre".to_string()),
                    spell_name: Some("Balle plombante".to_string()),
                    is_critical: true,
                },
                FightEvent::FightEnded {
                    fight_id: 1568151141
                },
            ]
        );
    }
}
