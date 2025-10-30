use crate::model::fighter::Fighter;
use chrono::DateTime;

#[derive(Debug)]
pub struct Fight {
    pub fighters: Vec<Fighter>,
    pub start_date: DateTime<chrono::Utc>,
    pub finish_date: DateTime<chrono::Utc>,
}

impl Fight {
    pub fn start_fight(&mut self, fighters: Vec<Fighter>) {
        let now = chrono::Utc::now();

        log::info!("Fight started at {}", now);
        self.start_date = now;
        self.fighters = fighters;
    }
}
