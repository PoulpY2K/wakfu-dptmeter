use std::collections::HashMap;

use super::model::{Character, Side};

/// Tracks every character seen so far in the active fight and resolves a
/// summon's actions back to whichever real fighter owns it.
#[derive(Debug, Default)]
pub(super) struct ParticipantRegistry {
    by_entity_id: HashMap<i64, Character>,
    // Most recently joined/invoked entity for a given display name. The
    // Wakfu log only ever refers to actors by name in spell-cast and PV
    // lines (never by entity id), so when two owners have a same-named
    // summon alive at once, the log itself gives no way to tell which
    // instance a later line refers to. Resolving to the most recently
    // invoked entity mirrors the one thing we do know for certain: at the
    // moment of invocation, a summon belongs to whoever just invoked it.
    entity_id_by_name: HashMap<String, i64>,
    // Owner entity id staged by a summon invocation, consumed by the
    // FighterJoined that immediately follows it for the same summon name.
    pending_summon_owners: HashMap<String, i64>,
}

impl ParticipantRegistry {
    pub(super) fn clear(&mut self) {
        self.by_entity_id.clear();
        self.entity_id_by_name.clear();
        self.pending_summon_owners.clear();
    }

    pub(super) fn get(&self, entity_id: i64) -> Option<&Character> {
        self.by_entity_id.get(&entity_id)
    }

    /// Resolves `name` to the entity id that should be credited for its
    /// actions: itself for a real fighter, or its owner for a summon.
    pub(super) fn resolve_owner_entity_id(&self, name: &str) -> Option<i64> {
        let entity_id = *self.entity_id_by_name.get(name)?;
        let character = self.by_entity_id.get(&entity_id)?;
        Some(character.owner_entity_id.unwrap_or(character.entity_id))
    }

    /// Stages `owner_name` as the owner of a summon named `summon_name`, to
    /// be picked up by [`Self::register`] once that summon joins the fight.
    pub(super) fn stage_summon_owner(&mut self, owner_name: &str, summon_name: String) {
        if let Some(owner_entity_id) = self.resolve_owner_entity_id(owner_name) {
            self.pending_summon_owners
                .insert(summon_name, owner_entity_id);
        }
    }

    /// Records a character joining the fight, resolving it against any
    /// summon invocation staged for the same name. Returns the summon's
    /// owner entity id, or `None` if `name` is a real fighter.
    pub(super) fn register(&mut self, name: String, entity_id: i64, side: Side) -> Option<i64> {
        let owner_entity_id = self.pending_summon_owners.remove(&name);
        self.by_entity_id.insert(
            entity_id,
            Character {
                name: name.clone(),
                entity_id,
                side,
                owner_entity_id,
            },
        );
        self.entity_id_by_name.insert(name, entity_id);
        owner_entity_id
    }
}

#[cfg(test)]
mod tests {
    // Entity ids below are transcribed verbatim from real Wakfu log lines;
    // adding digit separators would misrepresent the source data.
    #![expect(clippy::unreadable_literal)]

    use super::*;

    #[test]
    fn a_real_fighter_resolves_to_itself() {
        let mut registry = ParticipantRegistry::default();
        registry.register("Blampy".to_string(), 5547447, Side::Player);

        assert_eq!(registry.resolve_owner_entity_id("Blampy"), Some(5547447));
    }

    #[test]
    fn an_unknown_name_resolves_to_nothing() {
        let registry = ParticipantRegistry::default();

        assert_eq!(registry.resolve_owner_entity_id("Blampy"), None);
    }

    #[test]
    fn a_summon_resolves_to_its_staged_owner() {
        let mut registry = ParticipantRegistry::default();
        registry.register("Blampy".to_string(), 5547447, Side::Player);
        registry.stage_summon_owner("Blampy", "Bombe Aveuglante".to_string());

        let owner_entity_id = registry.register("Bombe Aveuglante".to_string(), -1, Side::Enemy);

        assert_eq!(owner_entity_id, Some(5547447));
        assert_eq!(
            registry.resolve_owner_entity_id("Bombe Aveuglante"),
            Some(5547447)
        );
    }

    #[test]
    fn a_same_named_summon_reattributes_to_its_most_recent_owner() {
        let mut registry = ParticipantRegistry::default();
        registry.register("Blampy".to_string(), 5547447, Side::Player);
        registry.register("Distipy".to_string(), 11370102, Side::Player);

        registry.stage_summon_owner("Blampy", "Bombe Aveuglante".to_string());
        registry.register("Bombe Aveuglante".to_string(), -1, Side::Enemy);
        assert_eq!(
            registry.resolve_owner_entity_id("Bombe Aveuglante"),
            Some(5547447)
        );

        registry.stage_summon_owner("Distipy", "Bombe Aveuglante".to_string());
        registry.register("Bombe Aveuglante".to_string(), -2, Side::Enemy);
        assert_eq!(
            registry.resolve_owner_entity_id("Bombe Aveuglante"),
            Some(11370102)
        );
    }

    #[test]
    fn clear_removes_all_registered_state() {
        let mut registry = ParticipantRegistry::default();
        registry.register("Blampy".to_string(), 5547447, Side::Player);

        registry.clear();

        assert_eq!(registry.resolve_owner_entity_id("Blampy"), None);
        assert_eq!(registry.get(5547447), None);
    }
}
