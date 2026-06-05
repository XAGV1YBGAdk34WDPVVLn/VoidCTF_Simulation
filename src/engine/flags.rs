use crate::engine::GameEngine;

impl GameEngine {
    pub fn update_flags(&mut self) {
        let blue_base_pos = self.map_layout.bases["blue"].pos;
        let orange_base_pos = self.map_layout.bases["orange"].pos;

        let mut events_to_log = Vec::new();
        let mut action_occurred = false;

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
                action_occurred = true;
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
                        action_occurred = true;
                    } else if touching_player_team == *team && !flag_at_base {
                        let flag = self.flags.get_mut(team).unwrap();
                        flag.pos = if team == "blue" { blue_base_pos } else { orange_base_pos };
                        flag.at_base = true;
                        
                        events_to_log.push(format!("{} returned the {} Flag to base!", touching_player_name, team.to_uppercase()));
                        action_occurred = true;
                    }
                }
            }
        }

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
            action_occurred = true;
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
            action_occurred = true;
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
        if action_occurred || has_events {
            self.last_action_time = self.sim_time;
        }
    }
}
