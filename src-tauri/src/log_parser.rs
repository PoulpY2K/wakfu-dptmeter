use regex::Regex;
use std::sync::LazyLock;

static FIGHT_CREATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"CREATION DU COMBAT\s*$").unwrap());

static FIGHTER_JOINED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[_FL_] fightId=(\d+) (.+?) breed : \d+ \[(-?\d+)] isControlledByAI=(true|false) obstacleId : -?\d+ join the fight at \{Point3 : \([^)]*\)}",
    )
    .unwrap()
});

static SUMMON_INVOKED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)] (.+?): Invoque un\(e\) (.+?)\s*$").unwrap()
});

static SPELL_CAST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)] (.+?) lance le sort (.+?)(\s+\(Critiques\))?\s*$").unwrap()
});

static HP_CHANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(combat\)] (.+?): ([+-]?[\d\s]+?) PV((?:\s+\([^)]+\))*)\s*$").unwrap()
});

static HP_CHANGE_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(([^)]+)\)").unwrap());

static FIGHT_ENDED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[FIGHT] End fight with id (\d+)").unwrap());

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

pub fn parse_line(line: &str) -> LogEvent {
    try_fight_creation(line)
        .or_else(|| try_fighter_joined(line))
        .or_else(|| try_summon_invoked(line))
        .or_else(|| try_spell_cast(line))
        .or_else(|| try_hp_change(line))
        .or_else(|| try_fight_ended(line))
        .unwrap_or(LogEvent::Unrecognized)
}

fn try_fight_creation(line: &str) -> Option<LogEvent> {
    FIGHT_CREATION_RE
        .is_match(line)
        .then_some(LogEvent::FightCreationDetected)
}

fn try_fighter_joined(line: &str) -> Option<LogEvent> {
    let caps = FIGHTER_JOINED_RE.captures(line)?;
    let fight_id = caps[1].parse::<u64>().ok()?;
    let entity_id = caps[3].parse::<i64>().ok()?;
    Some(LogEvent::FighterJoined {
        fight_id,
        name: caps[2].to_string(),
        entity_id,
        is_controlled_by_ai: &caps[4] == "true",
    })
}

fn try_summon_invoked(line: &str) -> Option<LogEvent> {
    let caps = SUMMON_INVOKED_RE.captures(line)?;
    Some(LogEvent::SummonInvoked {
        owner_name: caps[1].to_string(),
        summon_name: caps[2].to_string(),
    })
}

fn try_spell_cast(line: &str) -> Option<LogEvent> {
    let caps = SPELL_CAST_RE.captures(line)?;
    Some(LogEvent::SpellCast {
        actor_name: caps[1].to_string(),
        spell_name: caps[2].to_string(),
        is_critical: caps.get(3).is_some(),
    })
}

fn try_hp_change(line: &str) -> Option<LogEvent> {
    let caps = HP_CHANGE_RE.captures(line)?;
    let amount = caps[2]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .parse::<i32>()
        .ok()?;

    let tags: Vec<&str> = HP_CHANGE_TAG_RE
        .captures_iter(&caps[3])
        .map(|tag_caps| tag_caps.get(1).unwrap().as_str())
        .collect();
    let is_parried = tags.iter().any(|t| *t == "Parade !");
    let element = tags
        .iter()
        .find(|t| **t != "Parade !")
        .map(|s| s.to_string());

    Some(LogEvent::HpChange {
        name: caps[1].to_string(),
        amount,
        element,
        is_parried,
    })
}

fn try_fight_ended(line: &str) -> Option<LogEvent> {
    let caps = FIGHT_ENDED_RE.captures(line)?;
    let fight_id = caps[1].parse::<u64>().ok()?;
    Some(LogEvent::FightEnded { fight_id })
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
                spell_name: "Transposition".to_string(),
                is_critical: false,
            }
        );
    }

    #[test]
    fn parses_spell_cast_line_with_critique_suffix() {
        let line = " INFO 12:50:21,535 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Soeur Zerker lance le sort Châtiment (Critiques)";
        assert_eq!(
            parse_line(line),
            LogEvent::SpellCast {
                actor_name: "Soeur Zerker".to_string(),
                spell_name: "Châtiment".to_string(),
                is_critical: true,
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
        let line = " INFO 12:50:23,547 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Blampy: -1\u{202F}757 PV  (Feu)";
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
        let line = " INFO 12:50:28,480 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Soeur Zerker: -1\u{202F}975 PV  (Feu) (Parade !)";
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

    #[test]
    fn parses_fight_ended_line() {
        let line = " INFO 12:50:50,028 [AWT-EventQueue-0] (aWF:91) - [FIGHT] End fight with id 1568151141";
        assert_eq!(
            parse_line(line),
            LogEvent::FightEnded {
                fight_id: 1568151141,
            }
        );
    }

    #[test]
    fn ignores_documented_noise_lines_as_unrecognized() {
        // Cas limites de la spec : ces lignes ne doivent jamais produire d'evenement.
        let not_found_line =
            " WARN 12:50:14,498 [AWT-EventQueue-0] (cky:29) - The fight with the id 1568151141 has not been found";
        let join_procedure_line =
            " INFO 12:50:22,848 [AWT-EventQueue-0] (cko:37) - Starting join procedure for 11049475";

        assert_eq!(parse_line(not_found_line), LogEvent::Unrecognized);
        assert_eq!(parse_line(join_procedure_line), LogEvent::Unrecognized);
    }

    #[test]
    fn parses_heal_line_with_leading_plus_sign() {
        // Ligne 7 de resources/wakfu-with-heal.log
        let line = " INFO 16:21:32,663 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Blampy: +343 PV (Eau)";
        assert_eq!(
            parse_line(line),
            LogEvent::HpChange {
                name: "Blampy".to_string(),
                amount: 343,
                element: Some("Eau".to_string()),
                is_parried: false,
            }
        );
    }

    #[test]
    fn parses_hp_change_line_with_four_trailing_tags() {
        // Ligne 18 de resources/wakfu-with-heal.log
        let line = " INFO 16:21:35,075 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Lumilpy: -817 PV (Lumière) (Feu) (Parade !) (Enflammé)";
        assert_eq!(
            parse_line(line),
            LogEvent::HpChange {
                name: "Lumilpy".to_string(),
                amount: -817,
                element: Some("Lumière".to_string()),
                is_parried: true,
            }
        );
    }

    #[test]
    fn does_not_panic_on_fighter_joined_line_with_out_of_range_fight_id() {
        let line = " INFO 12:50:14,595 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=99999999999999999999999999999999999999 Soeur Zerker breed : 4214 [-1724034221200073] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (-1, 3, 0)}";
        assert_eq!(parse_line(line), LogEvent::Unrecognized);
    }

    #[test]
    fn does_not_panic_on_fight_ended_line_with_out_of_range_fight_id() {
        let line = " INFO 12:50:50,028 [AWT-EventQueue-0] (aWF:91) - [FIGHT] End fight with id 99999999999999999999999999999999999999";
        assert_eq!(parse_line(line), LogEvent::Unrecognized);
    }

    #[test]
    fn does_not_panic_on_hp_change_line_with_out_of_range_amount() {
        let line = " INFO 12:50:20,635 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Distipy: -99999999999999999999999999999999999999 PV (Air)";
        assert_eq!(parse_line(line), LogEvent::Unrecognized);
    }
}
