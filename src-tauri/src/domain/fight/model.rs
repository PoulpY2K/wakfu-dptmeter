use serde::Serialize;

/// Which team a character belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Side {
    Player,
    Enemy,
}

/// Whether an [`FightEvent::ActionRecorded`] amount was damage or healing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ActionKind {
    Damage,
    Heal,
}

/// Domain events emitted by [`super::FightTracker`] and forwarded to the
/// webview frontend as `fight-event` payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FightEvent {
    FightStarted {
        fight_id: u64,
    },
    CharacterIdentified {
        fight_id: u64,
        name: String,
        entity_id: i64,
        side: Side,
    },
    TurnStarted {
        fight_id: u64,
        name: String,
        entity_id: i64,
        side: Side,
        turn_number: u32,
    },
    ActionRecorded {
        fight_id: u64,
        source: String,
        target: String,
        amount: i32,
        kind: ActionKind,
        element: Option<String>,
        spell_name: Option<String>,
        is_critical: bool,
        turn_number: u32,
    },
    FightEnded {
        fight_id: u64,
    },
}

/// A fighter or summon currently tracked in the active fight.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Character {
    pub(super) name: String,
    pub(super) entity_id: i64,
    pub(super) side: Side,
    // Some(owner_entity_id) for summons, None for real fighters.
    pub(super) owner_entity_id: Option<i64>,
}

/// The spell cast most recently attributed to a caster, staged until the
/// next HP change resolves who it hit.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CurrentCast {
    pub(super) caster_entity_id: i64,
    pub(super) spell_name: String,
    pub(super) is_critical: bool,
}
