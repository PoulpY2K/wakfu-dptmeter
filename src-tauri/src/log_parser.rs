use regex::Regex;
use std::sync::LazyLock;

static FIGHT_CREATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"CREATION DU COMBAT\s*$").unwrap());

static FIGHTER_JOINED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[_FL_\] fightId=(\d+) (.+?) breed : \d+ \[(-?\d+)\] isControlledByAI=(true|false) obstacleId : -?\d+ join the fight at \{Point3 : \([^)]*\)\}",
    )
    .unwrap()
});

static SUMMON_INVOKED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)\] (.+?): Invoque un\(e\) (.+?)\s*$").unwrap()
});

static SPELL_CAST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(combat\)\] (.+?) lance le sort ").unwrap());

static HP_CHANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)\] (.+?): (-?[\d ]+?) PV(?:\s+\(([^)]+)\))?( \(Parade !\))?\s*$").unwrap()
});

#[derive(Debug, Clone, PartialEq)]
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
    },
    HpChange {
        name: String,
        amount: i32,
        element: Option<String>,
        is_parried: bool,
    },
    Unrecognized,
}

pub fn parse_line(line: &str) -> LogEvent {
    if FIGHT_CREATION_RE.is_match(line) {
        return LogEvent::FightCreationDetected;
    }

    if let Some(caps) = FIGHTER_JOINED_RE.captures(line) {
        return LogEvent::FighterJoined {
            fight_id: caps[1].parse().expect("fight_id should be a valid u64"),
            name: caps[2].to_string(),
            entity_id: caps[3].parse().expect("entity_id should be a valid i64"),
            is_controlled_by_ai: &caps[4] == "true",
        };
    }

    if let Some(caps) = SUMMON_INVOKED_RE.captures(line) {
        return LogEvent::SummonInvoked {
            owner_name: caps[1].to_string(),
            summon_name: caps[2].to_string(),
        };
    }

    if let Some(caps) = SPELL_CAST_RE.captures(line) {
        return LogEvent::SpellCast {
            actor_name: caps[1].to_string(),
        };
    }

    if let Some(caps) = HP_CHANGE_RE.captures(line) {
        let amount: i32 = caps[2]
            .replace(' ', "")
            .parse()
            .expect("HP change amount should be a valid integer");
        return LogEvent::HpChange {
            name: caps[1].to_string(),
            amount,
            element: caps.get(3).map(|m| m.as_str().to_string()),
            is_parried: caps.get(4).is_some(),
        };
    }

    LogEvent::Unrecognized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unrelated_noise_line_as_unrecognized() {
        let line = " INFO 12:49:43,502 [main] (aVt:410) - Chargement de la configuration depuis le fichier : 'C:\\Users\\poulpyy\\AppData\\Roaming\\zaap\\gamesLogs\\wakfu/config'";
        assert_eq!(parse_line(line), LogEvent::Unrecognized);
    }

    #[test]
    fn parses_fight_creation_line() {
        let line = " INFO 12:50:14,591 [AWT-EventQueue-0] (aXI:47) - CREATION DU COMBAT";
        assert_eq!(parse_line(line), LogEvent::FightCreationDetected);
    }

    #[test]
    fn parses_fighter_joined_for_ai_controlled_entity() {
        let line = " INFO 12:50:14,595 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1568151141 Soeur Zerker breed : 4214 [-1724034221200073] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (-1, 3, 0)}";
        assert_eq!(
            parse_line(line),
            LogEvent::FighterJoined {
                fight_id: 1568151141,
                name: "Soeur Zerker".to_string(),
                entity_id: -1724034221200073,
                is_controlled_by_ai: true,
            }
        );
    }

    #[test]
    fn parses_fighter_joined_for_player_controlled_entity() {
        let line = " INFO 12:50:14,604 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1568151141 Blampy breed : 13 [5547447] isControlledByAI=false obstacleId : -1 join the fight at {Point3 : (-1, 0, 0)}";
        assert_eq!(
            parse_line(line),
            LogEvent::FighterJoined {
                fight_id: 1568151141,
                name: "Blampy".to_string(),
                entity_id: 5547447,
                is_controlled_by_ai: false,
            }
        );
    }

    #[test]
    fn parses_summon_invoked_line() {
        // Note: the real log line has a trailing space after the summon name.
        let line = " INFO 12:50:43,661 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Blampy: Invoque un(e) Bombe Aveuglante ";
        assert_eq!(
            parse_line(line),
            LogEvent::SummonInvoked {
                owner_name: "Blampy".to_string(),
                summon_name: "Bombe Aveuglante".to_string(),
            }
        );
    }

    #[test]
    fn parses_spell_cast_line() {
        let line = " INFO 12:50:19,275 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Soeur Zerker lance le sort Transposition";
        assert_eq!(
            parse_line(line),
            LogEvent::SpellCast {
                actor_name: "Soeur Zerker".to_string(),
            }
        );
    }

    #[test]
    fn parses_simple_hp_change_line() {
        let line = " INFO 12:50:20,635 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Distipy: -892 PV (Air)";
        assert_eq!(
            parse_line(line),
            LogEvent::HpChange {
                name: "Distipy".to_string(),
                amount: -892,
                element: Some("Air".to_string()),
                is_parried: false,
            }
        );
    }

    #[test]
    fn parses_hp_change_with_thousands_separator_and_double_space() {
        let line = " INFO 12:50:23,547 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Blampy: -1 757 PV  (Feu)";
        assert_eq!(
            parse_line(line),
            LogEvent::HpChange {
                name: "Blampy".to_string(),
                amount: -1757,
                element: Some("Feu".to_string()),
                is_parried: false,
            }
        );
    }

    #[test]
    fn parses_parried_hp_change_line() {
        let line = " INFO 12:50:28,480 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Soeur Zerker: -1 975 PV  (Feu) (Parade !)";
        assert_eq!(
            parse_line(line),
            LogEvent::HpChange {
                name: "Soeur Zerker".to_string(),
                amount: -1975,
                element: Some("Feu".to_string()),
                is_parried: true,
            }
        );
    }

    #[test]
    fn does_not_misfire_on_non_pv_status_lines() {
        let pm_line = " INFO 12:50:32,397 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Distipy: -2 PM max (Parti pris)";
        let pw_line = " INFO 12:50:19,244 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Marylpy: 0 PW (Pioche mélangée)";
        let pa_line = " INFO 12:50:32,396 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Distipy: 2 PA (Parti pris)";

        assert_eq!(parse_line(pm_line), LogEvent::Unrecognized);
        assert_eq!(parse_line(pw_line), LogEvent::Unrecognized);
        assert_eq!(parse_line(pa_line), LogEvent::Unrecognized);
    }
}
