// src/config.rs
// Game settings and stats configuration.



pub const MATCH_TIME_LIMIT: f32 = 300.0; // 5 minutes in seconds
pub const WINNING_CAPTURES: u32 = 3;
pub const RESPAWN_COOLDOWN: f32 = 6.0;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ClassStats {
    pub max_hp: f32,
    pub max_shield: f32,
    pub speed: f32,
    pub melee_damage: i32,
    pub disc_damage: i32,
    pub disc_cooldown: f32,
    pub disc_speed: f32,
    pub disc_range: f32,
    pub shield_regen_rate: f32,
    pub shield_regen_delay: f32,
}

pub fn get_class_stats(class_type: &str) -> ClassStats {
    match class_type {
        "Stalker" => ClassStats {
            max_hp: 80.0,
            max_shield: 40.0,
            speed: 13.0,
            melee_damage: 12,
            disc_damage: 15,
            disc_cooldown: 0.8,
            disc_speed: 35.0,
            disc_range: 70.0,
            shield_regen_rate: 6.0,
            shield_regen_delay: 4.0,
        },
        "Enforcer" => ClassStats {
            max_hp: 150.0,
            max_shield: 80.0,
            speed: 8.0,
            melee_damage: 25,
            disc_damage: 30,
            disc_cooldown: 2.0,
            disc_speed: 25.0,
            disc_range: 55.0,
            shield_regen_rate: 10.0,
            shield_regen_delay: 5.0,
        },
        _ => ClassStats { // Tactician
            max_hp: 100.0,
            max_shield: 50.0,
            speed: 10.5,
            melee_damage: 15,
            disc_damage: 18,
            disc_cooldown: 1.2,
            disc_speed: 30.0,
            disc_range: 65.0,
            shield_regen_rate: 8.0,
            shield_regen_delay: 4.5,
        },
    }
}
