use crate::engine::GameEngine;
use crate::player::{PlayerInfo, PlayerAction};
use crate::config::WINNING_CAPTURES;

impl GameEngine {
    pub fn update(&mut self, dt: f32, time_now: f32) {
        if self.is_paused {
            return;
        }
        self.sim_time += dt;

        if self.state == "CHAMPION_CELEBRATION" {
            if let Some(champ_idx) = self.tournament.champion_index {
                let finals = &self.tournament.matches[2];
                let is_blue = finals.blue_team_index == champ_idx;
                
                let champ_player_ids = if is_blue { vec![0, 1, 2] } else { vec![3, 4, 5] };
                let losing_player_ids = if is_blue { vec![3, 4, 5] } else { vec![0, 1, 2] };

                for id in &champ_player_ids {
                    if let Some(p) = self.players.get_mut(id) {
                        p.is_alive = true;
                        p.hp = p.max_hp;
                        p.shield = p.max_shield;
                        p.vel = [0.0, 0.0, 0.0];
                        if p.class_type == "Enforcer" {
                            p.pos = [0.0, 0.0, 1.5];
                            p.is_shielding = true;
                        } else if p.class_type == "Stalker" {
                            p.pos = [-5.0, -1.0, 2.0];
                            p.is_shielding = false;
                        } else {
                            p.pos = [5.0, -1.0, 2.0];
                            p.is_shielding = false;
                        }
                    }
                }

                for id in &losing_player_ids {
                    if let Some(p) = self.players.get_mut(id) {
                        p.is_alive = false;
                        p.pos = [0.0, -100.0, 0.0];
                        p.vel = [0.0, 0.0, 0.0];
                    }
                }
            }
            return;
        }

        if self.state == "PREGAME" {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.state = "RUNNING".to_string();
                self.last_action_time = self.sim_time;
                self.log_event("Match started! Grid is ACTIVE.");
            }
        } else if self.state == "RUNNING" {
            self.match_time -= dt;
            self.check_and_cycle_tactics();

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
                if pointer_less_than_or_eq(respawn_timer, 0.0) {
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
            
            if (self.sim_time * 30.0) as i32 % 60 == 0 {
                for p in self.players.values() {
                    println!("DEBUG: Player {} ({}) state={} pos={:?} target={:?} vel={:?} hp={}/{} shield={}/{}", p.name, p.team, p.state, p.pos, p.target_pos, p.vel, p.hp, p.max_hp, p.shield, p.max_shield);
                }
            }

            if pointer_less_than_or_eq(self.match_time, 0.0) {
                self.end_match(None);
                return;
            }

            let blue_carried = self.flags.get("blue").unwrap().carrier_id.is_some();
            let orange_carried = self.flags.get("orange").unwrap().carrier_id.is_some();
            if blue_carried && orange_carried {
                self.both_carried_timer += dt;
            } else {
                self.both_carried_timer = 0.0;
            }

            if self.both_carried_timer > 30.0 || self.sim_time - self.last_action_time > 30.0 {
                self.break_stalemate();
            }

            let player_infos: Vec<PlayerInfo> = self.players.values().map(|p| PlayerInfo {
                id: p.id,
                team: p.team.clone(),
                class_type: p.class_type.clone(),
                pos: p.pos,
                hp: p.hp,
                max_hp: p.max_hp,
                is_alive: p.is_alive,
                is_shielding: p.is_shielding,
                has_flag: p.has_flag,
            }).collect();

            let flags_data = self.flags_json();
            let mut pending_actions = Vec::new();

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

            self.update_projectiles(dt, time_now);
            self.update_flags();

            if self.scores["blue"] >= WINNING_CAPTURES {
                self.end_match(Some("blue".to_string()));
            } else if self.scores["orange"] >= WINNING_CAPTURES {
                self.end_match(Some("orange".to_string()));
            }
        }
    }
}

fn pointer_less_than_or_eq(val: f32, limit: f32) -> bool {
    val <= limit
}
