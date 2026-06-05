// src/player/mod.rs
// Player models, combat state machine, and navigation.

pub mod ai;
pub mod nav;
pub mod physics;

use crate::config::{get_class_stats, RESPAWN_COOLDOWN};

#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub id: u32,
    pub team: String,
    pub class_type: String,
    pub pos: [f32; 3],
    pub hp: f32,
    pub max_hp: f32,
    pub is_alive: bool,
    pub is_shielding: bool,
    pub has_flag: bool,
}

#[derive(Debug, Clone)]
pub enum PlayerAction {
    ShootDisc {
        owner_id: u32,
        team: String,
        pos: [f32; 3],
        vel: [f32; 3],
        damage: i32,
        range: f32,
    },
    MeleeStrike {
        owner_id: u32,
        target_id: u32,
        damage: f32,
    },
    HealAlly {
        owner_id: u32,
        target_id: u32,
        amount: f32,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub team: String,
    pub class_type: String,
    
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub hp: f32,
    pub max_hp: f32,
    pub shield: f32,
    pub max_shield: f32,
    pub is_alive: bool,
    pub has_flag: bool,
    pub state: String,
    pub kills: u32,
    pub deaths: u32,
    pub captures: u32,
    pub damage_dealt: i32,
    pub healing_done: i32,
    pub is_healing: bool,
    pub is_dashing: bool,
    pub is_shielding: bool,
    pub healing_target_id: Option<u32>,
    pub is_taking_cover: bool,
    pub overcharge_timer: f32,

    #[serde(skip_serializing)]
    pub spawn_pos: [f32; 3],
    #[serde(skip_serializing)]
    pub strategy: String,
    #[serde(skip_serializing)]
    pub target_pos: [f32; 3],
    #[serde(skip_serializing)]
    pub cover_target_pos: Option<[f32; 3]>,
    #[serde(skip_serializing)]
    pub ability_cooldown: f32,
    #[serde(skip_serializing)]
    pub ability_active_timer: f32,
    #[serde(skip_serializing)]
    pub disc_cooldown: f32,
    #[serde(skip_serializing)]
    pub last_damaged_time: f32,
    #[serde(skip_serializing)]
    pub last_stalling_alert_time: f32,
    #[serde(skip_serializing)]
    pub respawn_timer: f32,
    #[serde(skip_serializing)]
    pub stuck_frames: u32,
    #[serde(skip_serializing)]
    pub patrol_target_unset: bool,
    
    // Stats loaded from config
    #[serde(skip_serializing)]
    pub base_speed: f32,
    #[serde(skip_serializing)]
    pub melee_damage: i32,
    #[serde(skip_serializing)]
    pub disc_damage: i32,
    #[serde(skip_serializing)]
    pub disc_cooldown_max: f32,
    #[serde(skip_serializing)]
    pub disc_speed: f32,
    #[serde(skip_serializing)]
    pub disc_range: f32,
    #[serde(skip_serializing)]
    pub shield_regen_rate: f32,
}

impl Player {
    pub fn new(player_id: u32, name: String, team: String, class_type: String, spawn_pos: [f32; 3]) -> Self {
        let stats = get_class_stats(&class_type);
        Self {
            id: player_id,
            name,
            team,
            class_type,
            pos: spawn_pos,
            vel: [0.0, 0.0, 0.0],
            hp: stats.max_hp,
            max_hp: stats.max_hp,
            shield: stats.max_shield,
            max_shield: stats.max_shield,
            is_alive: true,
            has_flag: false,
            state: "PATROL".to_string(),
            kills: 0,
            deaths: 0,
            captures: 0,
            damage_dealt: 0,
            healing_done: 0,
            is_healing: false,
            is_dashing: false,
            is_shielding: false,
            healing_target_id: None,
            is_taking_cover: false,
            overcharge_timer: 0.0,
            
            spawn_pos,
            strategy: "SPLIT".to_string(),
            target_pos: spawn_pos,
            cover_target_pos: None,
            ability_cooldown: 0.0,
            ability_active_timer: 0.0,
            disc_cooldown: 0.0,
            last_damaged_time: -99.0,
            last_stalling_alert_time: -99.0,
            respawn_timer: 0.0,
            stuck_frames: 0,
            patrol_target_unset: true,
            
            base_speed: stats.speed,
            melee_damage: stats.melee_damage,
            disc_damage: stats.disc_damage,
            disc_cooldown_max: stats.disc_cooldown,
            disc_speed: stats.disc_speed,
            disc_range: stats.disc_range,
            shield_regen_rate: stats.shield_regen_rate,
        }
    }

    pub fn take_damage(&mut self, mut amount: f32, time_now: f32) -> bool {
        if !self.is_alive {
            return false;
        }
        self.last_damaged_time = time_now;
        if self.shield > 0.0 {
            if self.shield >= amount {
                self.shield -= amount;
                amount = 0.0;
            } else {
                amount -= self.shield;
                self.shield = 0.0;
            }
        }
        if amount > 0.0 {
            self.hp -= amount;
            if self.hp <= 0.0 {
                self.hp = 0.0;
                self.de_rez(time_now);
                return true;
            }
        }
        false
    }

    pub fn de_rez(&mut self, _time_now: f32) {
        self.is_alive = false;
        self.hp = 0.0;
        self.shield = 0.0;
        self.has_flag = false;
        self.respawn_timer = RESPAWN_COOLDOWN;
        self.vel = [0.0, 0.0, 0.0];
        self.is_shielding = false;
        self.is_dashing = false;
        self.is_healing = false;
        self.healing_target_id = None;
        self.is_taking_cover = false;
    }

    pub fn respawn(&mut self) {
        self.is_alive = true;
        self.pos = self.spawn_pos;
        self.vel = [0.0, 0.0, 0.0];
        self.hp = self.max_hp;
        self.shield = self.max_shield;
        self.has_flag = false;
        self.state = "PATROL".to_string();
        self.ability_cooldown = 0.0;
        self.ability_active_timer = 0.0;
        self.disc_cooldown = 0.0;
        self.last_damaged_time = -99.0;
        self.last_stalling_alert_time = -99.0;
        self.respawn_timer = 0.0;
        self.stuck_frames = 0;
        self.patrol_target_unset = true;
    }

    pub fn heal(&mut self, amount: f32) {
        if !self.is_alive {
            return;
        }
        self.hp = (self.hp + amount).min(self.max_hp);
    }

    pub fn update_timers(&mut self, dt: f32, time_now: f32) {
        if !self.is_alive {
            if self.respawn_timer > 0.0 {
                self.respawn_timer -= dt;
                if self.respawn_timer <= 0.0 {
                    self.respawn();
                }
            }
            return;
        }

        if self.ability_cooldown > 0.0 {
            self.ability_cooldown -= dt;
        }
        if self.disc_cooldown > 0.0 {
            self.disc_cooldown -= dt;
        }
        if self.overcharge_timer > 0.0 {
            self.overcharge_timer -= dt;
        }

        if self.ability_active_timer > 0.0 {
            self.ability_active_timer -= dt;
            if self.ability_active_timer <= 0.0 {
                self.is_shielding = false;
                self.is_dashing = false;
            }
        }

        if time_now - self.last_damaged_time > 4.0 {
            if self.shield < self.max_shield {
                self.shield = (self.shield + self.shield_regen_rate * dt).min(self.max_shield);
            } else if self.hp < self.max_hp {
                let hp_regen_rate = self.max_hp * 0.05;
                self.hp = (self.hp + hp_regen_rate * dt).min(self.max_hp);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Building, Platform, Ramp, BaseInfo, MapLayout};
    use std::collections::HashMap;

    fn setup_mock_layout() -> MapLayout {
        let mut platforms = Vec::new();
        platforms.push(Platform {
            id: "p_blue".to_string(),
            x: -70.0,
            y: -35.0,
            w: 30.0,
            d: 70.0,
            z: 7.0,
        });

        let mut ramps = Vec::new();
        ramps.push(Ramp {
            id: "r_blue".to_string(),
            x1: -85.0,
            x2: -70.0,
            y1: 8.0,
            y2: 24.0,
            z1: 0.0,
            z2: 7.0,
        });

        let mut bases = HashMap::new();
        bases.insert("blue".to_string(), BaseInfo { pos: [-80.0, 0.0, 0.0] });

        MapLayout {
            name: "Test Arena".to_string(),
            style: 0,
            platforms,
            buildings: vec![Building {
                id: "b_pillar".to_string(),
                x: -4.0,
                y: 22.0,
                w: 8.0,
                d: 8.0,
                z: 0.0,
                h: 30.0,
            }],
            spawns: HashMap::new(),
            bases,
            ramps,
        }
    }

    #[test]
    fn test_line_of_sight_blocked_by_building() {
        let layout = setup_mock_layout();
        // Target is straight through the pillar at (-4 to 4, 22 to 30)
        let p1 = [-10.0, 26.0, 0.0];
        let p2 = [10.0, 26.0, 0.0];
        assert!(!nav::check_line_of_sight(p1, p2, &layout.buildings, &[], 0.0));
    }

    #[test]
    fn test_line_of_sight_clear() {
        let layout = setup_mock_layout();
        let p1 = [-10.0, 10.0, 0.0];
        let p2 = [10.0, 10.0, 0.0];
        assert!(nav::check_line_of_sight(p1, p2, &layout.buildings, &[], 0.0));
    }

    #[test]
    fn test_choose_action_ramp_navigation_ascending() {
        let layout = setup_mock_layout();
        let mut player = Player::new(1, "TestBot".to_string(), "blue".to_string(), "Stalker".to_string(), [-90.0, 16.0, 0.0]);
        // Set target on the elevated platform
        player.target_pos = [-60.0, 16.0, 7.0];
        
        let mut flags = HashMap::new();
        flags.insert("blue".to_string(), serde_json::json!({
            "at_base": true,
            "pos": [-80.0, 0.0, 0.0],
            "carrier_id": null
        }));
        flags.insert("orange".to_string(), serde_json::json!({
            "at_base": true,
            "pos": [80.0, 0.0, 7.0],
            "carrier_id": null
        }));

        let strategy = serde_json::json!({
            "SPLIT": {
                "offensive_focus": 1.0,
                "defensive_focus": 1.0,
                "retreat_threshold": 0.35,
            }
        });
        let overcharge = serde_json::json!({"active": false});

        let player_infos = vec![PlayerInfo {
            id: player.id,
            team: player.team.clone(),
            class_type: player.class_type.clone(),
            pos: player.pos,
            hp: player.hp,
            max_hp: player.max_hp,
            is_alive: player.is_alive,
            is_shielding: player.is_shielding,
            has_flag: player.has_flag,
        }];

        // Tick 1: Player is on the ground, far from the ramp bottom entry point (-85.0).
        // Target is elevated, needs height change. Best ramp is r_blue.
        // Since player is at x = -90.0 (dist to lower is 5.0), they should navigate to lower_end first.
        let _actions = player.choose_action(&player_infos, &flags, &layout, &strategy, &overcharge, 0.016, 0.0);
        assert!(player.target_pos[2] > 0.0); // elevated target
    }
}
