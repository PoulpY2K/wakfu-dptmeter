#[derive(Debug)]
pub struct Fighter {
    pub id: u64,
    pub name: String,
    pub total_damage: u64,
    pub is_controlled_by_ai: bool,
}