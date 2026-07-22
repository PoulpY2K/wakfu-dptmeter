//! Compiled regex patterns for wakfu.log line parsing.

use std::sync::LazyLock;

use regex::Regex;

pub(super) static FIGHT_CREATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"CREATION DU COMBAT\s*$").unwrap());

pub(super) static FIGHTER_JOINED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[_FL_] fightId=(\d+) (.+?) breed : \d+ \[(-?\d+)] isControlledByAI=(true|false) obstacleId : -?\d+ join the fight at \{Point3 : \([^)]*\)}",
    )
    .unwrap()
});

pub(super) static SUMMON_INVOKED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(combat\)] (.+?): Invoque un\(e\) (.+?)\s*$").unwrap());

pub(super) static SPELL_CAST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)] (.+?) lance le sort (.+?)(\s+\(Critiques\))?\s*$").unwrap()
});

pub(super) static HP_CHANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)] (.+?): ([+-]?[\d\s]+?) PV((?:\s+\([^)]+\))*)\s*$").unwrap()
});

pub(super) static HP_CHANGE_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(([^)]+)\)").unwrap());

pub(super) static FIGHT_ENDED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[FIGHT] End fight with id (\d+)").unwrap());
