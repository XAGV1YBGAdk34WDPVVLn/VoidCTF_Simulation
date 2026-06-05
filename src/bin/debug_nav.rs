use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Building {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub d: f32,
    pub z: f32,
    pub h: f32,
}

#[derive(Clone, Debug)]
pub struct Platform {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub d: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct Ramp {
    pub id: String,
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub z1: f32,
    pub z2: f32,
}

#[derive(Clone, Debug)]
pub struct BaseInfo {
    pub pos: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct MapLayout {
    pub name: String,
    pub style: i32,
    pub platforms: Vec<Platform>,
    pub buildings: Vec<Building>,
    pub bases: HashMap<String, BaseInfo>,
    pub ramps: Vec<Ramp>,
}

mod math {
    pub fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
    }
    pub fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }
    pub fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
        [a[0] * s, a[1] * s, a[2] * s]
    }
    pub fn length_sq(a: [f32; 3]) -> f32 {
        a[0] * a[0] + a[1] * a[1] + a[2] * a[2]
    }
    pub fn length(a: [f32; 3]) -> f32 {
        length_sq(a).sqrt()
    }
    pub fn normalize(a: [f32; 3]) -> [f32; 3] {
        let len = length(a);
        if len > 0.00001 {
            scale(a, 1.0 / len)
        } else {
            [0.0, 0.0, 0.0]
        }
    }
    pub fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        length(sub(a, b))
    }
}

pub fn check_line_of_sight(
    p1: [f32; 3],
    p2: [f32; 3],
    buildings: &[Building],
    platforms: &[Platform],
    radius: f32,
) -> bool {
    for b in buildings {
        let bx1 = b.x - radius;
        let by1 = b.y - radius;
        let bx2 = b.x + b.w + radius;
        let by2 = b.y + b.d + radius;
        let bz1 = b.z;
        let bz2 = b.z + b.h;

        let mut tmin = 0.0f32;
        let mut tmax = 1.0f32;
        let mut blocked = true;

        for i in 0..2 {
            let orig = p1[i];
            let dir_v = p2[i] - p1[i];
            let bmin = if i == 0 { bx1 } else { by1 };
            let bmax = if i == 0 { bx2 } else { by2 };

            if dir_v.abs() < 1e-6 {
                if orig < bmin || orig > bmax {
                    blocked = false;
                    break;
                }
            } else {
                let mut t0 = (bmin - orig) / dir_v;
                let mut t1 = (bmax - orig) / dir_v;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                tmin = tmin.max(t0);
                tmax = tmax.min(t1);
                if tmin > tmax {
                    blocked = false;
                    break;
                }
            }
        }

        if blocked && tmin <= tmax && tmin < 1.0 && tmax > 0.0 {
            let t_enter = tmin.max(0.0);
            let t_exit = tmax.min(1.0);
            let z_enter = p1[2] + t_enter * (p2[2] - p1[2]);
            let z_exit = p1[2] + t_exit * (p2[2] - p1[2]);

            let z_ray_min = z_enter.min(z_exit);
            let z_ray_max = z_enter.max(z_exit);

            if z_ray_max >= bz1 && z_ray_min <= bz2 {
                return false;
            }
        }
    }
    true
}

pub fn get_navigation_target(
    p_pos: [f32; 3],
    target_pos: [f32; 3],
    buildings: &[Building],
    platforms: Option<&[Platform]>,
    ramps: Option<&[Ramp]>,
) -> [f32; 3] {
    let mut all_obstacles = buildings.to_vec();
    if let Some(plats) = platforms {
        for p in plats {
            let target_on_plat = target_pos[0] >= p.x
                && target_pos[0] <= p.x + p.w
                && target_pos[1] >= p.y
                && target_pos[1] <= p.y + p.d;

            if p.z > p_pos[2] + 2.5 && !target_on_plat {
                all_obstacles.push(Building {
                    id: format!("plat_obs_{}", p.id),
                    x: p.x,
                    y: p.y,
                    w: p.w,
                    d: p.d,
                    z: 0.0,
                    h: p.z,
                });
            }
        }
    }

    if let Some(rmps) = ramps {
        for r in rmps {
            let r_max_z = r.z1.max(r.z2);
            let target_on_ramp = target_pos[0] >= r.x1
                && target_pos[0] <= r.x2
                && target_pos[1] >= r.y1
                && target_pos[1] <= r.y2;

            if r_max_z > p_pos[2] + 2.5 && !target_on_ramp {
                all_obstacles.push(Building {
                    id: format!("ramp_side_bot_{}", r.id),
                    x: r.x1,
                    y: r.y1 - 0.2,
                    w: r.x2 - r.x1,
                    d: 0.2,
                    z: 0.0,
                    h: r_max_z,
                });
                all_obstacles.push(Building {
                    id: format!("ramp_side_top_{}", r.id),
                    x: r.x1,
                    y: r.y2,
                    w: r.x2 - r.x1,
                    d: 0.2,
                    z: 0.0,
                    h: r_max_z,
                });
            }
        }
    }

    if check_line_of_sight(p_pos, target_pos, &all_obstacles, &[], 1.5) {
        return target_pos;
    }

    let mut waypoints = Vec::new();
    let padding = 7.0;
    for b in &all_obstacles {
        let bx1 = b.x;
        let by1 = b.y;
        let bx2 = b.x + b.w;
        let by2 = b.y + b.d;
        let bz = p_pos[2];

        let mut wps = vec![
            [bx1 - padding, by1 - padding, bz],
            [bx2 + padding, by1 - padding, bz],
            [bx1 - padding, by2 + padding, bz],
            [bx2 + padding, by2 + padding, bz],
        ];

        for wp in &mut wps {
            wp[0] = wp[0].clamp(-95.0, 95.0);
            wp[1] = wp[1].clamp(-95.0, 95.0);
        }
        
        for wp in wps {
            let mut inside_any = false;
            for b2 in &all_obstacles {
                let bx1_b2 = b2.x;
                let by1_b2 = b2.y;
                let bx2_b2 = b2.x + b2.w;
                let by2_b2 = b2.y + b2.d;
                let bz1_b2 = b2.z;
                let bz2_b2 = b2.z + b2.h;
                
                let radius = 2.2;
                if bz1_b2 <= wp[2] && wp[2] < bz2_b2 {
                    if bx1_b2 - radius <= wp[0] && wp[0] <= bx2_b2 + radius
                       && by1_b2 - radius <= wp[1] && wp[1] <= by2_b2 + radius {
                        inside_any = true;
                        break;
                    }
                }
            }
            if !inside_any {
                waypoints.push(wp);
            }
        }
    }

    let mut nodes = vec![p_pos, target_pos];
    nodes.extend(waypoints.clone());
    
    let n = nodes.len();
    let mut dist = vec![f32::INFINITY; n];
    let mut prev = vec![None; n];
    let mut visited = vec![false; n];
    
    dist[0] = 0.0;
    
    for _ in 0..n {
        let mut u = None;
        let mut min_d = f32::INFINITY;
        for i in 0..n {
            if !visited[i] && dist[i] < min_d {
                min_d = dist[i];
                u = Some(i);
            }
        }
        
        let u = match u {
            Some(idx) => idx,
            None => break,
        };
        
        if u == 1 {
            break;
        }
        
        visited[u] = true;
        
        for v in 0..n {
            if visited[v] {
                continue;
            }
            if check_line_of_sight(nodes[u], nodes[v], &all_obstacles, &[], 1.5) {
                let d = math::distance(nodes[u], nodes[v]);
                let alt = dist[u] + d;
                if alt < dist[v] {
                    dist[v] = alt;
                    prev[v] = Some(u);
                }
            }
        }
    }

    if dist[1] < f32::INFINITY {
        let mut curr = 1;
        let mut path = Vec::new();
        while let Some(p) = prev[curr] {
            path.push(curr);
            curr = p;
        }
        for &node_idx in &path {
            if check_line_of_sight(p_pos, nodes[node_idx], &all_obstacles, &[], 1.5) {
                return nodes[node_idx];
            }
        }
        if let Some(&first_wp_idx) = path.last() {
            return nodes[first_wp_idx];
        }
    }

    let mut valid_fallback_wps = Vec::new();
    for wp in nodes.iter().skip(2) {
        if check_line_of_sight(p_pos, *wp, &all_obstacles, &[], 1.5) {
            valid_fallback_wps.push(*wp);
        }
    }
    if !valid_fallback_wps.is_empty() {
        valid_fallback_wps.sort_by(|a, b| math::distance(*a, target_pos).partial_cmp(&math::distance(*b, target_pos)).unwrap());
        return *valid_fallback_wps.first().unwrap();
    }

    target_pos
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub team: String,
    pub class_type: String,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub state: String,
    pub target_pos: [f32; 3],
    pub base_speed: f32,
    pub stuck_frames: u32,
}

impl Player {
    pub fn update_physics(
        &mut self,
        dt: f32,
        current_speed: f32,
        to_target: [f32; 3],
        distance: f32,
        map_layout: &MapLayout,
    ) {
        let dir_vec = math::normalize(to_target);
        let mut avoidance_force = [0.0, 0.0, 0.0];
        for b in &map_layout.buildings {
            let b_center = [b.x + b.w / 2.0, b.y + b.d / 2.0, self.pos[2]];
            let to_building = math::sub(self.pos, b_center);
            let b_dist = math::length(to_building);
            let safe_dist = (b.w + b.d) / 3.0 + 3.0;
            if b_dist < safe_dist {
                if self.pos[2] >= b.z && self.pos[2] <= b.z + b.h {
                    avoidance_force = math::add(avoidance_force, math::scale(math::normalize(to_building), 5.0 / (b_dist + 0.01)));
                }
            }
        }

        let noise_time = (self.id as f32 * 12.34) + (self.pos[0] * 0.1) + (self.pos[1] * 0.1);
        let wander_strength = 1.2;
        let wander_force = [
            noise_time.cos() * wander_strength,
            (noise_time * 1.5).sin() * wander_strength,
            0.0
        ];

        let desired_vel = math::add(
            math::add(math::scale(dir_vec, current_speed), avoidance_force),
            wander_force
        );
        self.vel = math::add(math::scale(self.vel, 0.7), math::scale(desired_vel, 0.3));

        let old_pos = self.pos;
        let mut new_pos = math::add(self.pos, math::scale(self.vel, dt));

        for b in &map_layout.buildings {
            let bx1 = b.x;
            let by1 = b.y;
            let bx2 = b.x + b.w;
            let by2 = b.y + b.d;
            let bz1 = b.z;
            let bz2 = b.z + b.h;

            if (bz1 <= new_pos[2] && new_pos[2] < bz2 - 0.5) || (bz1 <= self.pos[2] && self.pos[2] < bz2 - 0.5) {
                let radius = 2.2;
                if bx1 - radius <= new_pos[0] && new_pos[0] <= bx2 + radius && by1 - radius <= new_pos[1] && new_pos[1] <= by2 + radius {
                    let mut candidates = Vec::new();
                    if self.pos[0] <= bx1 - radius + 0.05 {
                        candidates.push(("left", (new_pos[0] - (bx1 - radius)).abs()));
                    }
                    if self.pos[0] >= bx2 + radius - 0.05 {
                        candidates.push(("right", (new_pos[0] - (bx2 + radius)).abs()));
                    }
                    if self.pos[1] <= by1 - radius + 0.05 {
                        candidates.push(("bottom", (new_pos[1] - (by1 - radius)).abs()));
                    }
                    if self.pos[1] >= by2 + radius - 0.05 {
                        candidates.push(("top", (new_pos[1] - (by2 + radius)).abs()));
                    }

                    let face = if !candidates.is_empty() {
                        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                        candidates.first().unwrap().0
                    } else {
                        let l_dist = (new_pos[0] - (bx1 - radius)).abs();
                        let r_dist = (new_pos[0] - (bx2 + radius)).abs();
                        let t_dist = (new_pos[1] - (by1 - radius)).abs();
                        let b_dist = (new_pos[1] - (by2 + radius)).abs();
                        let min_dist = l_dist.min(r_dist).min(t_dist).min(b_dist);
                        if min_dist == l_dist {
                            "left"
                        } else if min_dist == r_dist {
                            "right"
                        } else if min_dist == t_dist {
                            "bottom"
                        } else {
                            "top"
                        }
                    };

                    match face {
                        "left" => {
                            new_pos[0] = bx1 - radius;
                            self.vel[0] = self.vel[0].min(0.0);
                        }
                        "right" => {
                            new_pos[0] = bx2 + radius;
                            self.vel[0] = self.vel[0].max(0.0);
                        }
                        "bottom" => {
                            new_pos[1] = by1 - radius;
                            self.vel[1] = self.vel[1].min(0.0);
                        }
                        _ => {
                            new_pos[1] = by2 + radius;
                            self.vel[1] = self.vel[1].max(0.0);
                        }
                    }
                }
            }
        }

        new_pos[0] = new_pos[0].clamp(-98.0, 98.0);
        new_pos[1] = new_pos[1].clamp(-98.0, 98.0);

        let mut target_z = 0.0;
        let mut on_ramp = false;

        let mut natural_floor_z = 0.0;
        for platform in &map_layout.platforms {
            if platform.x <= self.pos[0] && self.pos[0] <= platform.x + platform.w 
               && platform.y <= self.pos[1] && self.pos[1] <= platform.y + platform.d {
                natural_floor_z = platform.z;
                break;
            }
        }
        if natural_floor_z == 0.0 {
            for base in map_layout.bases.values() {
                let dist_2d = math::distance(
                    [self.pos[0], self.pos[1], 0.0],
                    [base.pos[0], base.pos[1], 0.0],
                );
                if dist_2d <= 6.0 {
                    natural_floor_z = 1.5;
                    break;
                }
            }
        }

        let on_floor = (self.pos[2] - natural_floor_z).abs() < 2.0;
        let target_on_same_level = (self.target_pos[2] - natural_floor_z).abs() < 2.0;
        let bypass_ramp = on_floor && target_on_same_level;

        if !bypass_ramp {
            for ramp in &map_layout.ramps {
                if ramp.x1 - 0.2 <= new_pos[0] && new_pos[0] <= ramp.x2 + 0.2 && ramp.y1 - 0.2 <= new_pos[1] && new_pos[1] <= ramp.y2 + 0.2 {
                    let x_span = ramp.x2 - ramp.x1;
                    if x_span > 0.0 {
                        let ratio = ((new_pos[0] - ramp.x1) / x_span).clamp(0.0, 1.0);
                        let r_z = ramp.z1 + ratio * (ramp.z2 - ramp.z1);
                        if (new_pos[2] - r_z).abs() < 1.8 {
                            target_z = r_z;
                            on_ramp = true;
                            break;
                        }
                    }
                }
            }
        }

        if !on_ramp {
            for platform in &map_layout.platforms {
                if platform.x <= new_pos[0] && new_pos[0] <= platform.x + platform.w && platform.y <= new_pos[1] && new_pos[1] <= platform.y + platform.d {
                    if (new_pos[2] - platform.z).abs() < 3.0 || (self.pos[2] >= platform.z - 1.0 && self.vel[2] >= -1.0) {
                        target_z = platform.z;
                        break;
                    }
                }
            }
        }

        if !on_ramp && target_z == 0.0 {
            for base in map_layout.bases.values() {
                let dist_2d = math::distance(
                    [new_pos[0], new_pos[1], 0.0],
                    [base.pos[0], base.pos[1], 0.0],
                );
                if dist_2d <= 6.0 {
                    target_z = 1.5;
                    break;
                }
            }
        }

        if on_ramp {
            new_pos[2] = target_z;
        } else {
            new_pos[2] = 0.8 * self.pos[2] + 0.2 * target_z;
        }
        self.pos = new_pos;

        let actual_dist = math::distance(self.pos, old_pos);
        let expected_dist = current_speed * dt;
        if expected_dist > 0.1 && actual_dist < expected_dist * 0.05 && distance > 0.15 {
            self.stuck_frames += 1;
        } else {
            self.stuck_frames = 0;
        }
    }
}

fn main() {
    let fort_w = 28.525967;
    let fort_d = 64.38451;
    let center_size = 33.730186;
    let side_z = 7.0;
    let center_z = 12.0;

    let platforms = vec![
        Platform {
            id: "p_blue_fort".to_string(),
            x: -fort_w - 30.0,
            y: -fort_d / 2.0,
            w: fort_w,
            d: fort_d,
            z: side_z,
        },
        Platform {
            id: "p_orange_fort".to_string(),
            x: 30.0,
            y: -fort_d / 2.0,
            w: fort_w,
            d: fort_d,
            z: side_z,
        },
        Platform {
            id: "p_mid_high".to_string(),
            x: -center_size / 2.0,
            y: -center_size / 2.0,
            w: center_size,
            d: center_size,
            z: center_z,
        }
    ];

    let ramps = vec![
        Ramp {
            id: "r_fort_blue_g".to_string(),
            x1: -fort_w - 45.0,
            x2: -fort_w - 30.0,
            y1: 8.0,
            y2: 24.0,
            z1: 0.0,
            z2: side_z,
        },
        Ramp {
            id: "r_fort_orange_g".to_string(),
            x1: fort_w + 30.0,
            x2: fort_w + 45.0,
            y1: 8.0,
            y2: 24.0,
            z1: side_z,
            z2: 0.0,
        },
        Ramp {
            id: "r_fort_blue_c".to_string(),
            x1: -30.0,
            x2: -center_size / 2.0,
            y1: -8.0,
            y2: 8.0,
            z1: side_z,
            z2: center_z,
        },
        Ramp {
            id: "r_fort_orange_c".to_string(),
            x1: center_size / 2.0,
            x2: 30.0,
            y1: -8.0,
            y2: 8.0,
            z1: center_z,
            z2: side_z,
        }
    ];

    let buildings = vec![
        Building { id: "b_blue_fort_wall".to_string(), x: -fort_w - 20.0, y: fort_d / 2.0 - 15.0, w: 6.0, d: 12.0, z: 0.0, h: 25.0 },
        Building { id: "b_blue_fort_wall_2".to_string(), x: -fort_w - 20.0, y: -fort_d / 2.0 + 5.0, w: 6.0, d: 12.0, z: 0.0, h: 25.0 },
        Building { id: "b_orange_fort_wall".to_string(), x: fort_w + 14.0, y: fort_d / 2.0 - 15.0, w: 6.0, d: 12.0, z: 0.0, h: 25.0 },
        Building { id: "b_orange_fort_wall_2".to_string(), x: fort_w + 14.0, y: -fort_d / 2.0 + 5.0, w: 6.0, d: 12.0, z: 0.0, h: 25.0 },
        Building { id: "b_mid_pillar_n".to_string(), x: -4.0, y: 22.0, w: 8.0, d: 8.0, z: 0.0, h: 30.0 },
        Building { id: "b_mid_pillar_s".to_string(), x: -4.0, y: -30.0, w: 8.0, d: 8.0, z: 0.0, h: 30.0 }
    ];

    let mut bases = HashMap::new();
    bases.insert("blue".to_string(), BaseInfo { pos: [-80.0, 0.0, 0.0] });

    let map_layout = MapLayout {
        name: "Fortress Gates".to_string(),
        style: 2,
        platforms,
        buildings,
        bases,
        ramps,
    };

    // Initialize player at tick 282 state:
    let mut player = Player {
        id: 1,
        name: "Aero-Medic".to_string(),
        team: "blue".to_string(),
        class_type: "Tactician".to_string(),
        pos: [-80.48557, 0.00898466, 1.4999998],
        vel: [2.9715548, 0.698963, -1e-45],
        state: "PATROL".to_string(),
        target_pos: [-75.1383, 8.571438, 0.0],
        base_speed: 10.5,
        stuck_frames: 0,
    };

    let dt = 0.0333333; // 30Hz physics tick

    println!("Starting 60-tick simulation with ALL obstacles...");
    for tick in 1..=60 {
        let mut routing_target = player.target_pos;
        
        let spread_angle = player.id as f32 * 2.0;
        let spread_dist = 3.5;
        routing_target[0] += spread_angle.cos() * spread_dist;
        routing_target[1] += spread_angle.sin() * spread_dist;

        let nav_target = get_navigation_target(player.pos, routing_target, &map_layout.buildings, Some(&map_layout.platforms), Some(&map_layout.ramps));
        let to_target = math::sub(nav_target, player.pos);
        let distance = math::length(to_target);

        let target_distance = math::distance(routing_target, player.pos);
        let speed_factor = if target_distance < 2.0 {
            (target_distance / 2.0).clamp(0.15, 1.0)
        } else {
            1.0
        };
        let current_speed = player.base_speed * speed_factor;

        player.update_physics(dt, current_speed, to_target, distance, &map_layout);

        println!(
            "Tick {:02}: pos={:?} vel={:?} target_dist={:.3} nav_target={:?} spread_angle={:.3}",
            tick, player.pos, player.vel, target_distance, nav_target, spread_angle
        );
    }
}
