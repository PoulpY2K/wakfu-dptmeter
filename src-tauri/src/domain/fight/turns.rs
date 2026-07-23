use std::collections::HashMap;

/// Tracks whose turn it currently is and how many turns each character has
/// taken so far. The Wakfu log never states turn boundaries directly: a
/// spell cast resolving to a different owner than the current one is the
/// only signal that a new turn has started.
#[derive(Debug, Default)]
pub(super) struct TurnTracker {
    current_owner_entity_id: Option<i64>,
    counts_by_entity_id: HashMap<i64, u32>,
}

impl TurnTracker {
    pub(super) fn clear(&mut self) {
        self.current_owner_entity_id = None;
        self.counts_by_entity_id.clear();
    }

    pub(super) fn turn_number(&self, entity_id: i64) -> u32 {
        self.counts_by_entity_id
            .get(&entity_id)
            .copied()
            .unwrap_or_default()
    }

    /// Records a spell cast by `caster_entity_id`. Returns the new turn
    /// number if this starts a new turn, or `None` if the caster is
    /// continuing their current turn.
    pub(super) fn advance(&mut self, caster_entity_id: i64) -> Option<u32> {
        if self.current_owner_entity_id == Some(caster_entity_id) {
            return None;
        }
        self.current_owner_entity_id = Some(caster_entity_id);

        let turn_number = self
            .counts_by_entity_id
            .entry(caster_entity_id)
            .or_insert(0);
        *turn_number += 1;
        Some(*turn_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_cast_by_a_character_starts_their_first_turn() {
        let mut turns = TurnTracker::default();

        assert_eq!(turns.advance(1), Some(1));
        assert_eq!(turns.turn_number(1), 1);
    }

    #[test]
    fn repeated_casts_by_the_same_owner_stay_in_the_same_turn() {
        let mut turns = TurnTracker::default();
        turns.advance(1);

        assert_eq!(turns.advance(1), None);
        assert_eq!(turns.turn_number(1), 1);
    }

    #[test]
    fn a_cast_by_a_different_owner_starts_a_new_turn() {
        let mut turns = TurnTracker::default();
        turns.advance(1);

        assert_eq!(turns.advance(2), Some(1));
    }

    #[test]
    fn returning_to_a_previous_owner_increments_their_turn_count() {
        let mut turns = TurnTracker::default();
        turns.advance(1);
        turns.advance(2);

        assert_eq!(turns.advance(1), Some(2));
    }

    #[test]
    fn clear_resets_the_current_owner_and_all_turn_counts() {
        let mut turns = TurnTracker::default();
        turns.advance(1);

        turns.clear();

        assert_eq!(turns.turn_number(1), 0);
        assert_eq!(turns.advance(1), Some(1));
    }
}
