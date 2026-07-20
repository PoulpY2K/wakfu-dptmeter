use regex::Regex;
use std::sync::LazyLock;

static FIGHT_CREATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"CREATION DU COMBAT\s*$").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    FightCreationDetected,
    Unrecognized,
}

pub fn parse_line(line: &str) -> LogEvent {
    if FIGHT_CREATION_RE.is_match(line) {
        return LogEvent::FightCreationDetected;
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
}
