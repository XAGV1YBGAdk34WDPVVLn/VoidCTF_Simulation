use crate::engine::GameEngine;

impl GameEngine {
    pub fn update_projectiles(&mut self, dt: f32, time_now: f32) {
        let mut active_projectiles = Vec::new();
        let mut events_to_log = Vec::new();
        let mut action_occurred = false;

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
                            action_occurred = true;
                            target_name = p.name.clone();
                            target_class = p.class_type.clone();

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
                let mut map = proj.as_object_mut().unwrap().clone();
                map.insert("pos".to_string(), serde_json::json!(pos));
                map.insert("vel".to_string(), serde_json::json!(vel));
                map.insert("range_left".to_string(), serde_json::json!(range_left));
                map.insert("bounces".to_string(), serde_json::json!(bounces));
                active_projectiles.push(serde_json::Value::Object(map));
            }
        }

        self.projectiles = active_projectiles;

        let has_events = !events_to_log.is_empty();
        for msg in events_to_log {
            self.log_event(&msg);
        }
        if action_occurred || has_events {
            self.last_action_time = self.sim_time;
        }
    }
}
