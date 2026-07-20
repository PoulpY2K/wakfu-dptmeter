#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    Unrecognized,
}

pub fn parse_line(_line: &str) -> LogEvent {
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
}
