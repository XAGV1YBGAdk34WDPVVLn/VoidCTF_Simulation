// src/engine.rs
// Core game engine loop, physics ticks, flag scoring, and stats reporting.

use std::collections::HashMap;
use crate::world::{get_map_layout, MapLayout};
use crate::player::{Player, PlayerInfo, PlayerAction};
use crate::config::{MATCH_TIME_LIMIT, WINNING_CAPTURES};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Flag {
    pub team: String,
    pub pos: [f32; 3],
    pub carrier_id: Option<u32>,
    pub at_base: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GameEngine {
    pub map_layout: MapLayout,
    pub state: String, // "PREGAME", "RUNNING", "POSTGAME", "AUDITING"
    pub timer: f32,
    pub match_time: f32,
    pub scores: HashMap<String, u32>,
    pub players: HashMap<u32, Player>,
    pub flags: HashMap<String, Flag>,
    pub projectiles: Vec<serde_json::Value>,
    pub match_log: Vec<String>,
    pub start_time: f32,
    pub end_time: f32,
    pub tactics: HashMap<String, serde_json::Value>,
    pub audit_report: Option<String>,
    pub audit_loading: bool,
    pub sim_time: f32,
    pub last_action_time: f32,
    pub overcharge_node: serde_json::Value,
    
    #[serde(skip_serializing)]
    pub strategy_templates: serde_json::Value,
    #[serde(skip_serializing)]
    pub summary_stats: serde_json::Value,
}

impl GameEngine {
    pub fn new() -> Self {
        let map_layout = get_map_layout();
        
        let mut scores = HashMap::new();
        scores.insert("blue".to_string(), 0);
        scores.insert("orange".to_string(), 0);

        let mut flags = HashMap::new();
        let blue_base = map_layout.bases["blue"].pos;
        let orange_base = map_layout.bases["orange"].pos;
        flags.insert(
            "blue".to_string(),
            Flag {
                team: "blue".to_string(),
                pos: blue_base,
                carrier_id: None,
                at_base: true,
            },
        );
        flags.insert(
            "orange".to_string(),
            Flag {
                team: "orange".to_string(),
                pos: orange_base,
                carrier_id: None,
                at_base: true,
            },
        );

        let tactics_blue = serde_json::json!({
            "strategy": "SPLIT",
            "rationale": "Initializing grid protocols...",
            "source": "Default"
        });
        let tactics_orange = serde_json::json!({
            "strategy": "SPLIT",
            "rationale": "Initializing grid protocols...",
            "source": "Default"
        });

        let mut tactics = HashMap::new();
        tactics.insert("blue".to_string(), tactics_blue);
        tactics.insert("orange".to_string(), tactics_orange);

        let strategy_templates = serde_json::json!({
            "RUSH": {
                "offensive_focus": 1.5,
                "defensive_focus": 0.5,
                "retreat_threshold": 0.25,
                "description": "All-out blitz targeting the enemy flag. Players play aggressively and push deep."
            },
            "TURTLE": {
                "offensive_focus": 0.4,
                "defensive_focus": 1.8,
                "retreat_threshold": 0.45,
                "description": "Fortify the home base. Heavy stays near the flag, and support prioritizes keeping teammates alive."
            },
            "SPLIT": {
                "offensive_focus": 1.0,
                "defensive_focus": 1.0,
                "retreat_threshold": 0.35,
                "description": "Infiltrator maneuvers stealthily around the flanks, Heavy creates midfield distraction, Support assists dynamically."
            },
            "FLANK": {
                "offensive_focus": 1.3,
                "defensive_focus": 0.7,
                "retreat_threshold": 0.30,
                "description": "Exploit blind spots. Stalker flanks via elevated lanes while Enforcer creates pressure in the center."
            },
            "HARASS": {
                "offensive_focus": 1.2,
                "defensive_focus": 0.8,
                "retreat_threshold": 0.35,
                "description": "Intercept and clash in the midfield. Focus on de-rezzing enemy nodes to prevent them from staging attacks."
            },
            "COUNTER": {
                "offensive_focus": 0.8,
                "defensive_focus": 1.4,
                "retreat_threshold": 0.40,
                "description": "Defensive posture. Intercept enemy flag runners, then transition to a rapid counter-offensive push."
            }
        });

        let (node_x, node_y) = match map_layout.style {
            0 => (0.0, 15.0),
            1 => (0.0, 22.0),
            _ => (0.0, 8.0),
        };
        let mut node_z = 0.0;
        for p in &map_layout.platforms {
            if p.x <= node_x && node_x <= p.x + p.w && p.y <= node_y && node_y <= p.y + p.d {
                node_z = p.z;
                break;
            }
        }
        let overcharge_node = serde_json::json!({
            "pos": [node_x, node_y, node_z],
            "active": true,
            "respawn_timer": 0.0
        });

        let mut engine = Self {
            map_layout: map_layout.clone(),
            state: "PREGAME".to_string(),
            timer: 15.0,
            match_time: MATCH_TIME_LIMIT,
            scores,
            players: HashMap::new(),
            flags,
            projectiles: Vec::new(),
            match_log: Vec::new(),
            start_time: 0.0,
            end_time: 0.0,
            tactics,
            audit_report: None,
            audit_loading: false,
            sim_time: 0.0,
            last_action_time: 0.0,
            overcharge_node,
            strategy_templates,
            summary_stats: serde_json::json!({}),
        };

        println!("=== MAP ENVIRONMENT BUILD ===");
        for p in &map_layout.platforms {
            println!("  Platform: {} at x={}, y={}, w={}, d={}, z={}", p.id, p.x, p.y, p.w, p.d, p.z);
        }
        for b in &map_layout.buildings {
            println!("  Building: {} at x={}, y={}, w={}, d={}, z={}, h={}", b.id, b.x, b.y, b.w, b.d, b.z, b.h);
        }
        println!("=============================");

        engine.init_players();
        engine
    }

    pub fn init_players(&mut self) {
        let blue_spawns = &self.map_layout.spawns["blue"];
        self.players.insert(0, Player::new(0, "A-Tron".to_string(), "blue".to_string(), "Stalker".to_string(), blue_spawns[0]));
        self.players.insert(1, Player::new(1, "B-Block".to_string(), "blue".to_string(), "Enforcer".to_string(), blue_spawns[1]));
        self.players.insert(2, Player::new(2, "C-Medic".to_string(), "blue".to_string(), "Tactician".to_string(), blue_spawns[2]));

        let orange_spawns = &self.map_layout.spawns["orange"];
        self.players.insert(3, Player::new(3, "X-Vector".to_string(), "orange".to_string(), "Stalker".to_string(), orange_spawns[0]));
        self.players.insert(4, Player::new(4, "Y-Shield".to_string(), "orange".to_string(), "Enforcer".to_string(), orange_spawns[1]));
        self.players.insert(5, Player::new(5, "Z-Solder".to_string(), "orange".to_string(), "Tactician".to_string(), orange_spawns[2]));
    }

    pub fn apply_strategies(&mut self, blue_strategy: serde_json::Value, orange_strategy: serde_json::Value) {
        self.tactics.insert("blue".to_string(), blue_strategy.clone());
        self.tactics.insert("orange".to_string(), orange_strategy.clone());

        let blue_strat_name = blue_strategy.get("strategy").and_then(|v| v.as_str()).unwrap_or("SPLIT").to_string();
        let orange_strat_name = orange_strategy.get("strategy").and_then(|v| v.as_str()).unwrap_or("SPLIT").to_string();

        for p in self.players.values_mut() {
            if p.team == "blue" {
                p.strategy = blue_strat_name.clone();
            } else {
                p.strategy = orange_strat_name.clone();
            }
        }

        self.log_event(&format!("Tactic applied: Blue -> {} | Orange -> {}", blue_strat_name, orange_strat_name));
    }

    pub fn break_stalemate(&mut self) {
        self.last_action_time = self.sim_time;
        
        let blue_strat = if rand::random::<bool>() { "RUSH" } else { "SPLIT" };
        let orange_strat = if rand::random::<bool>() { "RUSH" } else { "SPLIT" };
        
        let blue_tactics = serde_json::json!({
            "strategy": blue_strat,
            "rationale": format!("STALEMATE INTERCEPTED: Overriding protocol to {} to force action.", blue_strat),
            "source": "Grid AI Overlord"
        });
        let orange_tactics = serde_json::json!({
            "strategy": orange_strat,
            "rationale": format!("STALEMATE INTERCEPTED: Overriding protocol to {} to force action.", orange_strat),
            "source": "Grid AI Overlord"
        });

        // Resolve double-capture stalemates immediately
        let both_stolen = {
            let blue_carried = self.flags.get("blue").unwrap().carrier_id.is_some();
            let orange_carried = self.flags.get("orange").unwrap().carrier_id.is_some();
            blue_carried && orange_carried
        };

        if both_stolen {
            self.log_event("STALEMATE INTERCEPTED: Returning both flags to bases!");
            let blue_base = self.map_layout.bases["blue"].pos;
            let orange_base = self.map_layout.bases["orange"].pos;
            
            let blue_flag = self.flags.get_mut("blue").unwrap();
            blue_flag.carrier_id = None;
            blue_flag.pos = blue_base;
            blue_flag.at_base = true;

            let orange_flag = self.flags.get_mut("orange").unwrap();
            orange_flag.carrier_id = None;
            orange_flag.pos = orange_base;
            orange_flag.at_base = true;

            for p in self.players.values_mut() {
                p.has_flag = false;
            }
        } else {
            self.log_event("STALEMATE INTERCEPTED (30s inactivity). Overriding strategies to force action!");
        }

        self.apply_strategies(blue_tactics, orange_tactics);
    }

    pub fn log_event(&mut self, msg: &str) {
        let elapsed = MATCH_TIME_LIMIT - self.match_time;
        self.match_log.push(format!("[{:.1}s] {}", elapsed, msg));
        println!("[{:.1}s] {}", elapsed, msg);
    }

    pub fn reset_match(&mut self) {
        self.state = "PREGAME".to_string();
        self.timer = 15.0;
        self.match_time = MATCH_TIME_LIMIT;
        self.scores.insert("blue".to_string(), 0);
        self.scores.insert("orange".to_string(), 0);
        self.projectiles.clear();
        self.match_log.clear();
        self.audit_report = None;
        self.audit_loading = false;
        self.sim_time = 0.0;
        self.last_action_time = 0.0;

        // Generate a new procedural map per round
        self.map_layout = crate::world::generate_random_map();

        // Update player spawn coordinates to match the new layout spawns
        let blue_spawns = &self.map_layout.spawns["blue"];
        let orange_spawns = &self.map_layout.spawns["orange"];
        if let Some(p) = self.players.get_mut(&0) { p.spawn_pos = blue_spawns[0]; }
        if let Some(p) = self.players.get_mut(&1) { p.spawn_pos = blue_spawns[1]; }
        if let Some(p) = self.players.get_mut(&2) { p.spawn_pos = blue_spawns[2]; }
        if let Some(p) = self.players.get_mut(&3) { p.spawn_pos = orange_spawns[0]; }
        if let Some(p) = self.players.get_mut(&4) { p.spawn_pos = orange_spawns[1]; }
        if let Some(p) = self.players.get_mut(&5) { p.spawn_pos = orange_spawns[2]; }

        for p in self.players.values_mut() {
            p.respawn();
            p.kills = 0;
            p.deaths = 0;
            p.captures = 0;
            p.damage_dealt = 0;
            p.healing_done = 0;
        }

        let blue_base = self.map_layout.bases["blue"].pos;
        let orange_base = self.map_layout.bases["orange"].pos;
        self.flags.get_mut("blue").unwrap().carrier_id = None;
        self.flags.get_mut("blue").unwrap().pos = blue_base;
        self.flags.get_mut("blue").unwrap().at_base = true;

        self.flags.get_mut("orange").unwrap().carrier_id = None;
        self.flags.get_mut("orange").unwrap().pos = orange_base;
        self.flags.get_mut("orange").unwrap().at_base = true;

        // Reset midfield overcharge node
        let (node_x, node_y) = match self.map_layout.style {
            0 => (0.0, 15.0),
            1 => (0.0, 22.0),
            _ => (0.0, 8.0),
        };
        let mut node_z = 0.0;
        for p in &self.map_layout.platforms {
            if p.x <= node_x && node_x <= p.x + p.w && p.y <= node_y && node_y <= p.y + p.d {
                node_z = p.z;
                break;
            }
        }
        self.overcharge_node = serde_json::json!({
            "pos": [node_x, node_y, node_z],
            "active": true,
            "respawn_timer": 0.0
        });

        self.log_event("Grid rebooted. Preparing match parameters...");
    }

    pub fn update(&mut self, dt: f32, time_now: f32) {
        self.sim_time += dt;

        if self.state == "PREGAME" {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.state = "RUNNING".to_string();
                self.last_action_time = self.sim_time;
                self.log_event("Match started! Grid is ACTIVE.");
            }
        } else if self.state == "RUNNING" {
            self.match_time -= dt;

            // Update overcharge node logic
            let mut active = self.overcharge_node.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut respawn_timer = self.overcharge_node.get("respawn_timer").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let node_pos_arr = self.overcharge_node.get("pos").unwrap().as_array().unwrap();
            let node_pos = [
                node_pos_arr[0].as_f64().unwrap() as f32,
                node_pos_arr[1].as_f64().unwrap() as f32,
                node_pos_arr[2].as_f64().unwrap() as f32,
            ];

            if !active {
                respawn_timer -= dt;
                if respawn_timer <= 0.0 {
                    active = true;
                    respawn_timer = 0.0;
                    self.log_event("MIDFIELD OVERCHARGE NODE spawned!");
                }
            }

            let mut pickup_player_name = None;
            if active {
                for p in self.players.values_mut() {
                    if p.is_alive {
                        let h_dist = crate::math::distance([p.pos[0], p.pos[1], 0.0], [node_pos[0], node_pos[1], 0.0]);
                        let v_dist = (p.pos[2] - node_pos[2]).abs();
                        if h_dist < 4.0 && v_dist < 4.0 {
                            p.overcharge_timer = 6.0;
                            p.shield = p.max_shield * 1.5;
                            active = false;
                            respawn_timer = 30.0;
                            pickup_player_name = Some(p.name.clone());
                            break;
                        }
                    }
                }
            }

            if let Some(name) = pickup_player_name {
                self.log_event(&format!("{} picked up the MIDFIELD OVERCHARGE!", name));
            }

            self.overcharge_node = serde_json::json!({
                "pos": node_pos,
                "active": active,
                "respawn_timer": respawn_timer
            });
            
            // Debug player positions every 2 seconds (approx 60 ticks)
            if (self.sim_time * 30.0) as i32 % 60 == 0 {
                for p in self.players.values() {
                    println!("DEBUG: Player {} ({}) state={} pos={:?} target={:?} vel={:?} hp={}/{} shield={}/{}", p.name, p.team, p.state, p.pos, p.target_pos, p.vel, p.hp, p.max_hp, p.shield, p.max_shield);
                }
            }

            if self.match_time <= 0.0 {
                self.end_match(None);
                return;
            }

            if self.sim_time - self.last_action_time > 30.0 {
                self.break_stalemate();
            }

            // To avoid double borrows, construct cheap flat PlayerInfo representations
            let player_infos: Vec<PlayerInfo> = self.players.values().map(|p| PlayerInfo {
                id: p.id,
                team: p.team.clone(),
                class_type: p.class_type.clone(),
                pos: p.pos,
                vel: p.vel,
                hp: p.hp,
                max_hp: p.max_hp,
                shield: p.shield,
                max_shield: p.max_shield,
                is_alive: p.is_alive,
                is_shielding: p.is_shielding,
                has_flag: p.has_flag,
            }).collect();

            let flags_data = self.flags_json();

            // Store actions to perform at the end of the tick
            let mut pending_actions = Vec::new();

            // Update timers and run decision logic for players
            let mut alerts = Vec::new();
            for p in self.players.values_mut() {
                p.update_timers(dt, time_now);
                let p_actions = p.choose_action(
                    &player_infos,
                    &flags_data,
                    &self.map_layout,
                    &self.strategy_templates,
                    &self.overcharge_node,
                    dt,
                    time_now,
                );
                pending_actions.extend(p_actions);

                if p.is_alive && (p.state == "RETREAT" || p.is_taking_cover) && (time_now - p.last_damaged_time > 6.0) && (p.hp < p.max_hp || p.shield < p.max_shield) {
                    if time_now - p.last_stalling_alert_time > 8.0 {
                        p.last_stalling_alert_time = time_now;
                        alerts.push(format!(
                            "TACTICAL ALERT: {} ({}) is stalling to heal! Press the fight to prevent node repair!",
                            p.name, p.team
                        ));
                    }
                }
            }

            for alert in alerts {
                self.log_event(&alert);
            }

            // Apply actions to resolve dependencies
            for action in pending_actions {
                match action {
                    PlayerAction::HealAlly { owner_id, target_id, amount } => {
                        let mut healed_amt = 0.0;
                        if let Some(target) = self.players.get_mut(&target_id) {
                            let old_hp_shield = target.hp + target.shield;
                            target.heal(amount);
                            healed_amt = (target.hp + target.shield) - old_hp_shield;
                        }
                        if healed_amt > 0.0 {
                            if let Some(owner) = self.players.get_mut(&owner_id) {
                                owner.healing_done += healed_amt as i32;
                            }
                        }
                    }
                    PlayerAction::MeleeStrike { owner_id, target_id, damage } => {
                        let mut target_name = String::new();
                        let mut target_class = String::new();
                        let mut de_rezzed = false;

                        if let Some(target) = self.players.get_mut(&target_id) {
                            target_name = target.name.clone();
                            target_class = target.class_type.clone();
                            de_rezzed = target.take_damage(damage, time_now);
                            self.last_action_time = self.sim_time;
                        }

                        let mut attacker_info = None;
                        if let Some(attacker) = self.players.get_mut(&owner_id) {
                            attacker.damage_dealt += damage as i32;
                            if de_rezzed {
                                attacker.kills += 1;
                                attacker_info = Some((attacker.name.clone(), attacker.class_type.clone()));
                            }
                        }

                        if let Some((att_name, att_class)) = attacker_info {
                            self.log_event(&format!(
                                "{} ({}) de-rezzed {} ({}) via Melee Strike",
                                att_name, att_class, target_name, target_class
                            ));
                        }
                    }
                    PlayerAction::ShootDisc { owner_id, team, pos, vel, damage, range } => {
                        let mut rng = rand::thread_rng();
                        let disc = serde_json::json!({
                            "id": rand::Rng::gen_range(&mut rng, 10000..99999),
                            "owner_id": owner_id,
                            "team": team,
                            "pos": pos,
                            "vel": vel,
                            "damage": damage,
                            "range_left": range,
                            "z_plane": pos[2]
                        });
                        self.projectiles.push(disc);
                    }
                }
            }

            // Update projectiles
            self.update_projectiles(dt, time_now);

            // Update flags
            self.update_flags();

            // Check score win threshold
            if self.scores["blue"] >= WINNING_CAPTURES {
                self.end_match(Some("blue".to_string()));
            } else if self.scores["orange"] >= WINNING_CAPTURES {
                self.end_match(Some("orange".to_string()));
            }
        }
    }

    pub fn update_projectiles(&mut self, dt: f32, time_now: f32) {
        let mut active_projectiles = Vec::new();
        let mut events_to_log = Vec::new();

        for mut proj in self.projectiles.drain(..) {
            let pos_arr = proj.get("pos").unwrap().as_array().unwrap();
            let mut pos = [
                pos_arr[0].as_f64().unwrap() as f32,
                pos_arr[1].as_f64().unwrap() as f32,
                pos_arr[2].as_f64().unwrap() as f32,
            ];
            let vel_arr = proj.get("vel").unwrap().as_array().unwrap();
            let mut vel = [
                vel_arr[0].as_f64().unwrap() as f32,
                vel_arr[1].as_f64().unwrap() as f32,
                vel_arr[2].as_f64().unwrap() as f32,
            ];

            let delta_move = crate::math::scale(vel, dt);
            let prev_pos = pos;
            pos = crate::math::add(pos, delta_move);

            let mut range_left = proj.get("range_left").unwrap().as_f64().unwrap() as f32;
            range_left -= crate::math::length(delta_move);

            let mut bounces = proj.get("bounces").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let mut hit = false;

            // 1. Boundary check
            if pos[0].abs() > 99.0 {
                if bounces < 3 {
                    vel[0] = -vel[0];
                    pos[0] = if pos[0] > 0.0 { 99.0 } else { -99.0 };
                    bounces += 1;
                } else {
                    hit = true;
                }
            }
            if !hit && pos[1].abs() > 99.0 {
                if bounces < 3 {
                    vel[1] = -vel[1];
                    pos[1] = if pos[1] > 0.0 { 99.0 } else { -99.0 };
                    bounces += 1;
                } else {
                    hit = true;
                }
            }

            // 2. Building check
            if !hit {
                for b in &self.map_layout.buildings {
                    if b.x <= pos[0] && pos[0] <= b.x + b.w && b.y <= pos[1] && pos[1] <= b.y + b.d {
                        if b.z <= pos[2] && pos[2] <= b.z + b.h {
                            if bounces < 3 {
                                let crossed_x = prev_pos[0] < b.x || prev_pos[0] > b.x + b.w;
                                let crossed_y = prev_pos[1] < b.y || prev_pos[1] > b.y + b.d;
                                if crossed_x && !crossed_y {
                                    vel[0] = -vel[0];
                                    pos[0] = if vel[0] > 0.0 { b.x + b.w + 0.05 } else { b.x - 0.05 };
                                } else if crossed_y && !crossed_x {
                                    vel[1] = -vel[1];
                                    pos[1] = if vel[1] > 0.0 { b.y + b.d + 0.05 } else { b.y - 0.05 };
                                } else {
                                    vel[0] = -vel[0];
                                    vel[1] = -vel[1];
                                    pos[0] = if vel[0] > 0.0 { b.x + b.w + 0.05 } else { b.x - 0.05 };
                                    pos[1] = if vel[1] > 0.0 { b.y + b.d + 0.05 } else { b.y - 0.05 };
                                }
                                bounces += 1;
                            } else {
                                hit = true;
                            }
                            break;
                        }
                    }
                }
            }

            // 2.5 Platform check
            if !hit {
                for p in &self.map_layout.platforms {
                    let pz1 = p.z - 0.8;
                    let pz2 = p.z;
                    if p.x <= pos[0] && pos[0] <= p.x + p.w && p.y <= pos[1] && pos[1] <= p.y + p.d {
                        if pz1 <= pos[2] && pos[2] <= pz2 {
                            if bounces < 3 {
                                let crossed_x = prev_pos[0] < p.x || prev_pos[0] > p.x + p.w;
                                let crossed_y = prev_pos[1] < p.y || prev_pos[1] > p.y + p.d;
                                let crossed_z = prev_pos[2] < pz1 || prev_pos[2] > pz2;
                                
                                if crossed_z && !crossed_x && !crossed_y {
                                    // Bounced vertically off floor or ceiling of platform!
                                    vel[2] = -vel[2];
                                    pos[2] = if vel[2] > 0.0 { pz2 + 0.05 } else { pz1 - 0.05 };
                                } else if crossed_x && !crossed_y {
                                    vel[0] = -vel[0];
                                    pos[0] = if vel[0] > 0.0 { p.x + p.w + 0.05 } else { p.x - 0.05 };
                                } else if crossed_y && !crossed_x {
                                    vel[1] = -vel[1];
                                    pos[1] = if vel[1] > 0.0 { p.y + p.d + 0.05 } else { p.y - 0.05 };
                                } else {
                                    vel[0] = -vel[0];
                                    vel[1] = -vel[1];
                                    pos[0] = if vel[0] > 0.0 { p.x + p.w + 0.05 } else { p.x - 0.05 };
                                    pos[1] = if vel[1] > 0.0 { p.y + p.d + 0.05 } else { p.y - 0.05 };
                                }
                                bounces += 1;
                            } else {
                                hit = true;
                            }
                            break;
                        }
                    }
                }
            }

            // 3. Player check
            if !hit {
                let owner_id = proj.get("owner_id").unwrap().as_u64().unwrap() as u32;
                let proj_team = proj.get("team").unwrap().as_str().unwrap();
                let damage = proj.get("damage").unwrap().as_f64().unwrap() as f32;

                let mut de_rezzed = false;
                let mut target_name = String::new();
                let mut target_class = String::new();

                for p in self.players.values_mut() {
                    if p.is_alive && p.team != proj_team {
                        let h_dist = crate::math::distance([p.pos[0], p.pos[1], 0.0], [pos[0], pos[1], 0.0]);
                        let v_dist = (p.pos[2] + 1.0 - pos[2]).abs();
                        if h_dist < 3.0 && v_dist < 4.0 {
                            hit = true;
                            let mut dmg = damage;
                            if p.is_shielding {
                                dmg *= 0.3;
                            }
                            de_rezzed = p.take_damage(dmg, time_now);
                            self.last_action_time = self.sim_time;
                            target_name = p.name.clone();
                            target_class = p.class_type.clone();

                            // Apply stats
                            if let Some(owner) = self.players.get_mut(&owner_id) {
                                owner.damage_dealt += dmg as i32;
                                if de_rezzed {
                                    owner.kills += 1;
                                }
                            }
                            break;
                        }
                    }
                }

                if hit && de_rezzed {
                    if let Some(owner) = self.players.get(&owner_id) {
                        events_to_log.push(format!("{} ({}) de-rezzed {} ({})", owner.name, owner.class_type, target_name, target_class));
                    }
                }
            }

            if !hit && range_left > 0.0 {
                // Update fields
                let mut map = proj.as_object_mut().unwrap().clone();
                map.insert("pos".to_string(), serde_json::json!(pos));
                map.insert("vel".to_string(), serde_json::json!(vel));
                map.insert("range_left".to_string(), serde_json::json!(range_left));
                map.insert("bounces".to_string(), serde_json::json!(bounces));
                active_projectiles.push(serde_json::Value::Object(map));
            }
        }

        self.projectiles = active_projectiles;

        for event in events_to_log {
            self.log_event(&event);
        }
    }

    pub fn update_flags(&mut self) {
        let blue_base_pos = self.map_layout.bases["blue"].pos;
        let orange_base_pos = self.map_layout.bases["orange"].pos;

        let mut events_to_log = Vec::new();

        // 1. Update Carrier Position Tracking
        for team in &["blue".to_string(), "orange".to_string()] {
            let drop_event = {
                let flag = &self.flags[team];
                if let Some(carrier_id) = flag.carrier_id {
                    let carrier = &self.players[&carrier_id];
                    if !carrier.is_alive {
                        Some((carrier.name.clone(), carrier.pos))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((carrier_name, carrier_pos)) = drop_event {
                events_to_log.push(format!("{} Flag dropped by {}!", team.to_uppercase(), carrier_name));
                let flag = self.flags.get_mut(team).unwrap();
                flag.carrier_id = None;
                flag.at_base = false;
                flag.pos = carrier_pos;
            } else {
                let carrier_pos = {
                    let flag = &self.flags[team];
                    flag.carrier_id.map(|cid| self.players[&cid].pos)
                };
                if let Some(pos) = carrier_pos {
                    let flag = self.flags.get_mut(team).unwrap();
                    flag.pos = [pos[0], pos[1], pos[2] + 2.0];
                }
            }
        }

        // 2. Check Interceptions & Grabs
        for team in &["blue".to_string(), "orange".to_string()] {
            let opposing_team = if team == "blue" { "orange" } else { "blue" };
            
            let flag_pos = self.flags[team].pos;
            let flag_carrier_id = self.flags[team].carrier_id;
            let flag_at_base = self.flags[team].at_base;

            if flag_carrier_id.is_none() {
                let mut touching_player_id = None;
                let mut touching_player_team = String::new();
                let mut touching_player_name = String::new();

                for p in self.players.values() {
                    if p.is_alive {
                        let dist = crate::math::distance(p.pos, flag_pos);
                        if dist < 4.5 {
                            touching_player_id = Some(p.id);
                            touching_player_team = p.team.clone();
                            touching_player_name = p.name.clone();
                            break;
                        }
                    }
                }

                if let Some(pid) = touching_player_id {
                    if touching_player_team == opposing_team {
                        self.players.get_mut(&pid).unwrap().has_flag = true;
                        
                        let flag = self.flags.get_mut(team).unwrap();
                        flag.carrier_id = Some(pid);
                        flag.at_base = false;
                        
                        events_to_log.push(format!("{} secured {} Flag!", touching_player_name, team.to_uppercase()));
                    } else if touching_player_team == *team && !flag_at_base {
                        let flag = self.flags.get_mut(team).unwrap();
                        flag.pos = if team == "blue" { blue_base_pos } else { orange_base_pos };
                        flag.at_base = true;
                        
                        events_to_log.push(format!("{} returned the {} Flag to base!", touching_player_name, team.to_uppercase()));
                    }
                }
            }
        }

        // 3. Check Captures / Scoring
        let mut blue_scored = false;
        let mut orange_scored = false;
        let mut blue_carrier_id = None;
        let mut orange_carrier_id = None;

        if let Some(cid) = self.flags["orange"].carrier_id {
            let carrier = &self.players[&cid];
            let dist = crate::math::distance(carrier.pos, blue_base_pos);
            if dist < 5.0 && self.flags["blue"].at_base {
                blue_scored = true;
                orange_carrier_id = Some(cid);
            }
        }

        if let Some(cid) = self.flags["blue"].carrier_id {
            let carrier = &self.players[&cid];
            let dist = crate::math::distance(carrier.pos, orange_base_pos);
            if dist < 5.0 && self.flags["orange"].at_base {
                orange_scored = true;
                blue_carrier_id = Some(cid);
            }
        }

        if blue_scored {
            *self.scores.get_mut("blue").unwrap() += 1;
            let cid = orange_carrier_id.unwrap();
            let carrier = self.players.get_mut(&cid).unwrap();
            carrier.captures += 1;
            carrier.has_flag = false;

            let orange_flag = self.flags.get_mut("orange").unwrap();
            orange_flag.carrier_id = None;
            orange_flag.pos = orange_base_pos;
            orange_flag.at_base = true;
            
            events_to_log.push(format!("SCORE! Blue Team captures Orange Flag! Blue: {} | Orange: {}", self.scores["blue"], self.scores["orange"]));
        }

        if orange_scored {
            *self.scores.get_mut("orange").unwrap() += 1;
            let cid = blue_carrier_id.unwrap();
            let carrier = self.players.get_mut(&cid).unwrap();
            carrier.captures += 1;
            carrier.has_flag = false;

            let blue_flag = self.flags.get_mut("blue").unwrap();
            blue_flag.carrier_id = None;
            blue_flag.pos = blue_base_pos;
            blue_flag.at_base = true;
            
            events_to_log.push(format!("SCORE! Orange Team captures Blue Flag! Blue: {} | Orange: {}", self.scores["blue"], self.scores["orange"]));
        }

        if blue_scored || orange_scored {
            let b_score = self.scores["blue"];
            let o_score = self.scores["orange"];
            if b_score == 2 && o_score == 0 {
                self.trigger_comeback("orange");
            } else if o_score == 2 && b_score == 0 {
                self.trigger_comeback("blue");
            }
        }

        let has_events = !events_to_log.is_empty();
        for msg in events_to_log {
            self.log_event(&msg);
        }
        if has_events {
            self.last_action_time = self.sim_time;
        }
    }

    pub fn trigger_comeback(&mut self, team: &str) {
        self.log_event(&format!("TACTICAL SHIFT: {} Team triggers COMEBACK strategy: RUSH!", team.to_uppercase()));
        for p in self.players.values_mut() {
            if p.team == team {
                p.strategy = "RUSH".to_string();
            }
        }
        if let Some(t) = self.tactics.get_mut(team) {
            if let Some(obj) = t.as_object_mut() {
                obj.insert("strategy".to_string(), serde_json::json!("RUSH"));
                obj.insert("rationale".to_string(), serde_json::json!("Auto-triggered fallback due to 2-0 score deficit."));
                obj.insert("source".to_string(), serde_json::json!("Engine"));
            }
        }
    }

    pub fn end_match(&mut self, mut winner: Option<String>) {
        self.state = "POSTGAME".to_string();
        self.end_time = rand::random::<f32>(); // placeholder or system time

        if winner.is_none() {
            let b_score = self.scores["blue"];
            let o_score = self.scores["orange"];
            if b_score > o_score {
                winner = Some("blue".to_string());
            } else if o_score > b_score {
                winner = Some("orange".to_string());
            } else {
                winner = Some("tie".to_string());
            }
        }

        let winner_name = winner.unwrap();
        self.log_event(&format!("GRID CLOSED. Match Winner: {}!", winner_name.to_uppercase()));

        let mut player_stats = HashMap::new();
        for p in self.players.values() {
            player_stats.insert(
                p.name.clone(),
                serde_json::json!({
                    "team": p.team,
                    "class": p.class_type,
                    "kills": p.kills,
                    "deaths": p.deaths,
                    "captures": p.captures,
                    "damage_dealt": p.damage_dealt,
                    "healing_done": p.healing_done
                }),
            );
        }

        let elapsed = MATCH_TIME_LIMIT - self.match_time;
        self.summary_stats = serde_json::json!({
            "winner": winner_name,
            "duration_seconds": elapsed as i32,
            "blue_captures": self.scores["blue"],
            "orange_captures": self.scores["orange"],
            "blue_strategy": self.tactics["blue"]["strategy"],
            "orange_strategy": self.tactics["orange"]["strategy"],
            "player_performance": player_stats
        });

        self.audit_loading = true;
    }

    fn flags_json(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        for (k, v) in &self.flags {
            map.insert(
                k.clone(),
                serde_json::json!({
                    "team": v.team,
                    "pos": v.pos,
                    "carrier_id": v.carrier_id,
                    "at_base": v.at_base
                }),
            );
        }
        map
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state,
            "timer": (self.timer * 10.0).round() / 10.0,
            "match_time": (self.match_time * 10.0).round() / 10.0,
            "scores": self.scores,
            "players": self.players,
            "flags": self.flags,
            "projectiles": self.projectiles,
            "tactics": self.tactics,
            "audit_report": self.audit_report,
            "audit_loading": self.audit_loading,
            "sim_time": self.sim_time,
            "overcharge_node": self.overcharge_node,
            "logs": self.match_log
        })
    }
}
