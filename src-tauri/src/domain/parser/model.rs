/// A single meaningful line parsed out of the Wakfu client log.
///
/// [`super::parse_line`] returns [`LogEvent::Unrecognized`] for any line
/// that doesn't match one of the known patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    FightCreationDetected,
    FighterJoined {
        fight_id: u64,
        name: String,
        entity_id: i64,
        is_controlled_by_ai: bool,
    },
    SummonInvoked {
        owner_name: String,
        summon_name: String,
    },
    SpellCast {
        actor_name: String,
        spell_name: String,
        is_critical: bool,
    },
    HpChange {
        name: String,
        amount: i32,
        element: Option<String>,
        is_parried: bool,
    },
    FightEnded {
        fight_id: u64,
    },
    Unrecognized,
}
