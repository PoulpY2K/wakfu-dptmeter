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

#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    FightCreationDetected,
    FighterJoined {
        fight_id: u64,
        name: String,
        entity_id: i64,
        is_controlled_by_ai: bool,
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
}
