// src/engine/mod.rs
// Core game engine loop, physics ticks, flag scoring, and stats reporting.

pub mod update;
pub mod projectiles;
pub mod flags;

use std::collections::HashMap;
use crate::world::{get_map_layout, MapLayout};
use crate::player::Player;
use crate::config::MATCH_TIME_LIMIT;

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
    pub both_carried_timer: f32,
    pub overcharge_node: serde_json::Value,
    pub tournament: crate::tournament::TournamentState,
    pub last_tactic_change_time: HashMap<String, f32>,
    
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

        let mut last_tactic_change_time = HashMap::new();
        last_tactic_change_time.insert("blue".to_string(), 0.0);
        last_tactic_change_time.insert("orange".to_string(), 0.0);

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
            both_carried_timer: 0.0,
            overcharge_node,
            tournament: crate::tournament::TournamentState::new(),
            last_tactic_change_time,
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
        let active_match = &self.tournament.matches[self.tournament.current_match_index];
        let blue_team = &self.tournament.teams[active_match.blue_team_index];
        let orange_team = &self.tournament.teams[active_match.orange_team_index];
        
        let blue_prefix = blue_team.name.split_whitespace().next().unwrap_or("Blue");
        let orange_prefix = orange_team.name.split_whitespace().next().unwrap_or("Orange");

        let blue_spawns = &self.map_layout.spawns["blue"];
        self.players.insert(0, Player::new(0, format!("{}-Tron", blue_prefix), "blue".to_string(), "Stalker".to_string(), blue_spawns[0]));
        self.players.insert(1, Player::new(1, format!("{}-Block", blue_prefix), "blue".to_string(), "Enforcer".to_string(), blue_spawns[1]));
        self.players.insert(2, Player::new(2, format!("{}-Medic", blue_prefix), "blue".to_string(), "Tactician".to_string(), blue_spawns[2]));

        let orange_spawns = &self.map_layout.spawns["orange"];
        self.players.insert(3, Player::new(3, format!("{}-Vector", orange_prefix), "orange".to_string(), "Stalker".to_string(), orange_spawns[0]));
        self.players.insert(4, Player::new(4, format!("{}-Shield", orange_prefix), "orange".to_string(), "Enforcer".to_string(), orange_spawns[1]));
        self.players.insert(5, Player::new(5, format!("{}-Solder", orange_prefix), "orange".to_string(), "Tactician".to_string(), orange_spawns[2]));
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

    pub fn apply_single_strategy(&mut self, team: &str, strategy_payload: serde_json::Value) {
        self.tactics.insert(team.to_string(), strategy_payload.clone());
        
        let strat_name = strategy_payload.get("strategy").and_then(|v| v.as_str()).unwrap_or("SPLIT").to_string();
        
        for p in self.players.values_mut() {
            if p.team == team {
                p.strategy = strat_name.clone();
            }
        }
        
        self.log_event(&format!("TACTICAL ADJUSTMENT: {} Team pivots to {} strategy!", team.to_uppercase(), strat_name));
    }

    pub fn check_and_cycle_tactics(&mut self) {
        if self.state != "RUNNING" {
            return;
        }

        let teams = vec!["blue".to_string(), "orange".to_string()];
        for team in teams {
            let last_change = *self.last_tactic_change_time.get(&team).unwrap_or(&0.0);
            if self.sim_time - last_change >= 50.0 {
                // Check if in comeback mode
                let current_rationale = self.tactics[&team].get("rationale").and_then(|v| v.as_str()).unwrap_or("");
                if current_rationale.contains("deficit") {
                    continue; // Skip cycling during comeback
                }

                // Generate a new strategy
                let mut new_tactic = crate::ai_tactics::get_pregame_tactics(&team);
                let current_strat = self.tactics[&team].get("strategy").and_then(|v| v.as_str()).unwrap_or("");
                
                // If it chose the same strategy, try choosing a different one
                let mut attempts = 0;
                while new_tactic.get("strategy").and_then(|v| v.as_str()).unwrap_or("") == current_strat && attempts < 5 {
                    new_tactic = crate::ai_tactics::get_pregame_tactics(&team);
                    attempts += 1;
                }

                // Update source to indicate it's a mid-match adjustment
                if let Some(obj) = new_tactic.as_object_mut() {
                    obj.insert("source".to_string(), serde_json::json!("Dynamic Tactic Assessor"));
                }

                self.apply_single_strategy(&team, new_tactic);
                self.last_tactic_change_time.insert(team, self.sim_time);
            }
        }
    }

    pub fn break_stalemate(&mut self) {
        self.last_action_time = self.sim_time;
        self.both_carried_timer = 0.0;
        
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
        self.both_carried_timer = 0.0;
        self.last_tactic_change_time.insert("blue".to_string(), 0.0);
        self.last_tactic_change_time.insert("orange".to_string(), 0.0);

        self.map_layout = crate::world::generate_random_map();

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

        let active_match = &self.tournament.matches[self.tournament.current_match_index];
        let blue_team = &self.tournament.teams[active_match.blue_team_index];
        let orange_team = &self.tournament.teams[active_match.orange_team_index];
        
        let blue_prefix = blue_team.name.split_whitespace().next().unwrap_or("Blue");
        let orange_prefix = orange_team.name.split_whitespace().next().unwrap_or("Orange");

        if let Some(p) = self.players.get_mut(&0) { p.name = format!("{}-Tron", blue_prefix); p.team = "blue".to_string(); }
        if let Some(p) = self.players.get_mut(&1) { p.name = format!("{}-Block", blue_prefix); p.team = "blue".to_string(); }
        if let Some(p) = self.players.get_mut(&2) { p.name = format!("{}-Medic", blue_prefix); p.team = "blue".to_string(); }
        if let Some(p) = self.players.get_mut(&3) { p.name = format!("{}-Vector", orange_prefix); p.team = "orange".to_string(); }
        if let Some(p) = self.players.get_mut(&4) { p.name = format!("{}-Shield", orange_prefix); p.team = "orange".to_string(); }
        if let Some(p) = self.players.get_mut(&5) { p.name = format!("{}-Solder", orange_prefix); p.team = "orange".to_string(); }

        let blue_base = self.map_layout.bases["blue"].pos;
        let orange_base = self.map_layout.bases["orange"].pos;
        self.flags.get_mut("blue").unwrap().carrier_id = None;
        self.flags.get_mut("blue").unwrap().pos = blue_base;
        self.flags.get_mut("blue").unwrap().at_base = true;

        self.flags.get_mut("orange").unwrap().carrier_id = None;
        self.flags.get_mut("orange").unwrap().pos = orange_base;
        self.flags.get_mut("orange").unwrap().at_base = true;

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
        self.end_time = rand::random::<f32>();

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
            "tournament": self.tournament,
            "logs": self.match_log
        })
    }
}
