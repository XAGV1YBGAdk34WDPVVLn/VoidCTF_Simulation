use crate::math;
use crate::world::{Building, Platform, Ramp};

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

    for p in platforms {
        let px1 = p.x - radius;
        let py1 = p.y - radius;
        let px2 = p.x + p.w + radius;
        let py2 = p.y + p.d + radius;
        let pz1 = p.z - 0.8;
        let pz2 = p.z;

        let mut tmin = 0.0f32;
        let mut tmax = 1.0f32;
        let mut blocked = true;

        for i in 0..2 {
            let orig = p1[i];
            let dir_v = p2[i] - p1[i];
            let bmin = if i == 0 { px1 } else { py1 };
            let bmax = if i == 0 { px2 } else { py2 };

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

            if z_ray_max > pz1 + 0.01 && z_ray_min < pz2 - 0.01 {
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
            // If the platform is higher than the player by more than 2.5 units, treat it as an obstacle
            if p.z > p_pos[2] + 2.5 {
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
            // If the ramp is higher than the player by more than 2.5 units, treat its sides as obstacles
            if r_max_z > p_pos[2] + 2.5 {
                // Bottom wall along Y = r.y1
                all_obstacles.push(Building {
                    id: format!("ramp_side_bot_{}", r.id),
                    x: r.x1,
                    y: r.y1 - 1.0,
                    w: r.x2 - r.x1,
                    d: 1.0,
                    z: 0.0,
                    h: r_max_z,
                });
                // Top wall along Y = r.y2
                all_obstacles.push(Building {
                    id: format!("ramp_side_top_{}", r.id),
                    x: r.x1,
                    y: r.y2,
                    w: r.x2 - r.x1,
                    d: 1.0,
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
    nodes.extend(waypoints);
    
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

pub fn find_cover_position(
    player_pos: [f32; 3],
    enemy_pos: [f32; 3],
    buildings: &[Building],
    platforms: &[Platform],
    player_radius: f32,
) -> Option<[f32; 3]> {
    let mut best_cover_pos = None;
    let mut best_dist = f32::INFINITY;

    for b in buildings {
        let bx = b.x + b.w / 2.0;
        let by = b.y + b.d / 2.0;
        let bz = b.z;
        let bh = b.h;

        if bz + bh < player_pos[2] + 1.0 {
            continue;
        }

        let b_center = [bx, by, player_pos[2]];
        let dist_to_b = math::distance(player_pos, b_center);
        if dist_to_b > 45.0 {
            continue;
        }

        let mut dir_enemy_to_b = math::sub(b_center, enemy_pos);
        dir_enemy_to_b[2] = 0.0;
        let dist_enemy_to_b = math::length(dir_enemy_to_b);
        if dist_enemy_to_b == 0.0 {
            continue;
        }

        let dir_enemy_to_b_norm = math::scale(dir_enemy_to_b, 1.0 / dist_enemy_to_b);
        let b_radius = ((b.w / 2.0).powi(2) + (b.d / 2.0).powi(2)).sqrt();
        let cover_point = math::add(b_center, math::scale(dir_enemy_to_b_norm, b_radius + player_radius + 2.0));
        
        if cover_point[0].abs() > 95.0 || cover_point[1].abs() > 95.0 {
            continue;
        }

        let mut inside_any = false;
        for b2 in buildings {
            let bx1 = b2.x;
            let by1 = b2.y;
            let bx2 = b2.x + b2.w;
            let by2 = b2.y + b2.d;
            let bz1 = b2.z;
            let bz2 = b2.z + b2.h;
            
            let radius = player_radius + 0.5;
            if bz1 <= cover_point[2] && cover_point[2] < bz2 {
                if bx1 - radius <= cover_point[0] && cover_point[0] <= bx2 + radius 
                   && by1 - radius <= cover_point[1] && cover_point[1] <= by2 + radius {
                    inside_any = true;
                    break;
                }
            }
        }
        if inside_any {
            continue;
        }

        if !check_line_of_sight(cover_point, enemy_pos, buildings, platforms, 0.0) {
            let dist_to_cover = math::distance(player_pos, cover_point);
            if dist_to_cover < best_dist {
                best_dist = dist_to_cover;
                best_cover_pos = Some(cover_point);
            }
        }
    }

    best_cover_pos
}
