import math
import random
import numpy as np
from simulation.config import CLASS_STATS, RESPAWN_COOLDOWN

def check_line_of_sight(p1, p2, buildings, radius=0.0) -> bool:
    """Checks if a 3D line segment between p1 and p2 intersects any solid AABB building (inflated by radius)."""
    for b in buildings:
        bx1, by1 = b["x"] - radius, b["y"] - radius
        bx2, by2 = b["x"] + b["w"] + radius, b["y"] + b["d"] + radius
        bz1, bz2 = b["z"], b["z"] + b["h"]
        
        # Ray-Box AABB slab intersection check in 2D (X/Y projection)
        tmin = 0.0
        tmax = 1.0
        blocked = True
        
        for i in range(2): # 0 for X, 1 for Y
            orig = p1[i]
            dir_v = p2[i] - p1[i]
            bmin = bx1 if i == 0 else by1
            bmax = bx2 if i == 0 else by2
            
            if abs(dir_v) < 1e-6:
                if orig < bmin or orig > bmax:
                    blocked = False
                    break
            else:
                t0 = (bmin - orig) / dir_v
                t1 = (bmax - orig) / dir_v
                if t0 > t1:
                    t0, t1 = t1, t0
                tmin = max(tmin, t0)
                tmax = min(tmax, t1)
                if tmin > tmax:
                    blocked = False
                    break
        
        if blocked and tmin <= tmax and tmin < 1.0 and tmax > 0.0:
            # The line segment's 2D projection intersects the building footprint.
            # Now verify if the ray segment actually intersects the building vertically during this overlap.
            t_enter = max(0.0, tmin)
            t_exit = min(1.0, tmax)
            z_enter = p1[2] + t_enter * (p2[2] - p1[2])
            z_exit = p1[2] + t_exit * (p2[2] - p1[2])
            
            z_ray_min = min(z_enter, z_exit)
            z_ray_max = max(z_enter, z_exit)
            
            # If the ray's Z range overlaps with the building's solid vertical boundaries, it is blocked
            if z_ray_max >= bz1 and z_ray_min <= bz2:
                return False
            
    return True # Clear path

def get_navigation_target(p_pos, target_pos, buildings, platforms=None) -> np.ndarray:
    """If path is blocked by a building, route towards the best visible corner waypoint (clamped to platform footprints)."""
    # Use C-space inflated checking for movement pathfinding
    if check_line_of_sight(p_pos, target_pos, buildings, radius=2.2):
        return target_pos
        
    # Find if player is currently on an elevated platform
    current_platform = None
    if platforms:
        for plat in platforms:
            px, py, pw, pd, pz = plat["x"], plat["y"], plat["w"], plat["d"], plat["z"]
            if abs(p_pos[2] - pz) < 1.0 and px <= p_pos[0] <= px + pw and py <= p_pos[1] <= py + pd:
                current_platform = plat
                break

    # Generate waypoints around building corners dynamically
    waypoints = []
    padding = 7.0 # outer margin for corner walking
    for b in buildings:
        bx1, by1 = b["x"], b["y"]
        bx2, by2 = b["x"] + b["w"], b["y"] + b["d"]
        bz = p_pos[2] # match player height plane
        
        # 4 Corners
        wps = [
            np.array([bx1 - padding, by1 - padding, bz]),
            np.array([bx2 + padding, by1 - padding, bz]),
            np.array([bx1 - padding, by2 + padding, bz]),
            np.array([bx2 + padding, by2 + padding, bz])
        ]
        
        # If player is on a platform, restrict waypoints to stay on the platform
        if current_platform:
            px, py, pw, pd = current_platform["x"], current_platform["y"], current_platform["w"], current_platform["d"]
            for wp in wps:
                # Clamp with a 1.0 margin to prevent sliding off the edges
                wp[0] = np.clip(wp[0], px + 1.0, px + pw - 1.0)
                wp[1] = np.clip(wp[1], py + 1.0, py + pd - 1.0)
                
        waypoints.extend(wps)
        
    # Select waypoints that the player has immediate line of sight to (using inflated check)
    valid_wps = []
    for wp in waypoints:
        if abs(wp[0]) > 98 or abs(wp[1]) > 98:
            continue
        if check_line_of_sight(p_pos, wp, buildings, radius=2.2):
            valid_wps.append(wp)
            
    if not valid_wps:
        return target_pos # Stuck fallback
        
    # Choose the visible waypoint that gets closest to the ultimate destination
    best_wp = min(valid_wps, key=lambda wp: np.linalg.norm(wp - target_pos))
    return best_wp

def find_cover_position(player_pos, enemy_pos, buildings, player_radius=2.2) -> np.ndarray:
    """Finds a cover position on the opposite side of a nearby building relative to the enemy."""
    best_cover_pos = None
    best_dist = float('inf')
    
    for b in buildings:
        bx = b["x"] + b["w"] / 2.0
        by = b["y"] + b["d"] / 2.0
        bz = b["z"]
        bh = b["h"]
        
        # Verify if building height is high enough to cover the player
        if bz + bh < player_pos[2] + 1.0:
            continue # Building is too short to provide cover
            
        b_center = np.array([bx, by, player_pos[2]])
        
        # Distance from player to building
        dist_to_b = np.linalg.norm(player_pos - b_center)
        
        # If building is too far away, ignore it (cover must be local)
        if dist_to_b > 45.0:
            continue
            
        # Direction from enemy to building center
        dir_enemy_to_b = b_center - enemy_pos
        dir_enemy_to_b[2] = 0.0 # 2D math
        dist_enemy_to_b = np.linalg.norm(dir_enemy_to_b)
        
        if dist_enemy_to_b == 0:
            continue
            
        dir_enemy_to_b_norm = dir_enemy_to_b / dist_enemy_to_b
        
        # Building radius approximation (half diagonal of the footprint)
        b_radius = np.sqrt((b["w"]/2.0)**2 + (b["d"]/2.0)**2)
        
        # Cover point is on the opposite side of the building from the enemy
        cover_point = b_center + dir_enemy_to_b_norm * (b_radius + player_radius + 2.0)
        cover_point[2] = player_pos[2]
        
        # Check if the cover point is inside map bounds
        if abs(cover_point[0]) > 95 or abs(cover_point[1]) > 95:
            continue
            
        # Check if this cover point actually breaks line of sight to the enemy
        if not check_line_of_sight(cover_point, enemy_pos, buildings):
            dist_to_cover = np.linalg.norm(player_pos - cover_point)
            if dist_to_cover < best_dist:
                best_dist = dist_to_cover
                best_cover_pos = cover_point
                
    return best_cover_pos

class Player:
    def __init__(self, player_id: int, name: str, team: str, class_type: str, spawn_pos: list):
        self.id = player_id
        self.name = name
        self.team = team  # "blue" or "orange"
        self.class_type = class_type  # "Stalker", "Enforcer", "Tactician"
        
        # Load stats based on class
        stats = CLASS_STATS[class_type]
        self.max_hp = stats["max_hp"]
        self.max_shield = stats["max_shield"]
        self.base_speed = stats["speed"]
        self.melee_damage = stats["melee_damage"]
        self.disc_damage = stats["disc_damage"]
        self.disc_cooldown_max = stats["disc_cooldown"]
        self.disc_speed = stats["disc_speed"]
        self.disc_range = stats["disc_range"]
        self.shield_regen_rate = stats["shield_regen_rate"]
        self.shield_regen_delay = stats["shield_regen_delay"]
        
        # Game State Variables
        self.spawn_pos = np.array(spawn_pos, dtype=float)
        self.pos = np.copy(self.spawn_pos)
        self.vel = np.array([0.0, 0.0, 0.0])
        self.hp = self.max_hp
        self.shield = self.max_shield
        self.is_alive = True
        self.has_flag = False
        
        # Cooldowns and Timers
        self.disc_cooldown = 0.0
        self.last_damaged_time = 0.0
        self.respawn_timer = 0.0
        self.ability_cooldown = 0.0
        self.ability_active_timer = 0.0
        self.overcharge_timer = 0.0
        
        # AI Control States
        self.state = "SPAWNING"  # SPAWNING, PATROL, INFILTRATE, ESCORT, RETREAT, HEAL_ALLIED
        self.target_pos = np.copy(self.pos)
        self.target_player_id = None
        self.path = []
        self.strategy = "SPLIT"  # Default
        
        # Statistics
        self.kills = 0
        self.deaths = 0
        self.captures = 0
        self.damage_dealt = 0
        self.healing_done = 0
        
        # Combat/visual state indicators sent to front-end
        self.is_healing = False
        self.is_dashing = False
        self.is_shielding = False
        self.healing_target_id = None
        self.is_taking_cover = False

    def take_damage(self, amount: float, time_now: float) -> bool:
        """Applies damage to shield then HP. Returns True if player de-rezzed."""
        if not self.is_alive:
            return False
            
        self.last_damaged_time = time_now
        
        # Shield absorption
        if self.shield > 0:
            if amount <= self.shield:
                self.shield -= amount
                amount = 0
            else:
                amount -= self.shield
                self.shield = 0
                
        # HP damage
        if amount > 0:
            self.hp = max(0.0, self.hp - amount)
            
        if self.hp <= 0:
            self.de_rez(time_now)
            return True
        return False

    def de_rez(self, time_now: float):
        """Kills the player and triggers respawn cooldown."""
        self.is_alive = False
        self.hp = 0
        self.shield = 0
        self.deaths += 1
        self.respawn_timer = RESPAWN_COOLDOWN
        self.has_flag = False
        self.is_healing = False
        self.is_dashing = False
        self.is_shielding = False
        self.healing_target_id = None
        self.vel = np.array([0.0, 0.0, 0.0])

    def respawn(self):
        """Respawns the player at spawn coordinates."""
        self.pos = np.copy(self.spawn_pos)
        self.vel = np.array([0.0, 0.0, 0.0])
        self.hp = self.max_hp
        self.shield = self.max_shield
        self.is_alive = True
        self.state = "PATROL"
        self.disc_cooldown = 0.0
        self.ability_cooldown = 0.0
        self.overcharge_timer = 0.0
        self.has_flag = False

    def heal(self, amount: float):
        """Heals HP first, then shield."""
        if not self.is_alive:
            return
        if self.hp < self.max_hp:
            self.hp = min(self.max_hp, self.hp + amount)
        elif self.shield < self.max_shield:
            self.shield = min(self.max_shield, self.shield + amount)

    def update_timers(self, dt: float, time_now: float):
        """Updates cooldowns, shields, and passive heals."""
        if not self.is_alive:
            self.respawn_timer -= dt
            if self.respawn_timer <= 0:
                self.respawn()
            return

        # Shield passive regeneration
        if time_now - self.last_damaged_time > self.shield_regen_delay:
            if self.shield < self.max_shield:
                self.shield = min(self.max_shield, self.shield + self.shield_regen_rate * dt)
                
        # Cooldown reductions
        if self.disc_cooldown > 0:
            self.disc_cooldown = max(0.0, self.disc_cooldown - dt)
        if self.ability_cooldown > 0:
            self.ability_cooldown = max(0.0, self.ability_cooldown - dt)
        if self.overcharge_timer > 0.0:
            self.overcharge_timer = max(0.0, self.overcharge_timer - dt)
            
        # Ability active timers
        if self.ability_active_timer > 0:
            self.ability_active_timer -= dt
            if self.ability_active_timer <= 0:
                self.is_dashing = False
                self.is_shielding = False

    def choose_action(self, game_state, dt: float, time_now: float):
        """Core AI logic: Updates state machine and moves the player."""
        if not self.is_alive:
            return
            
        # Check surrounding threats/targets
        enemies = [p for p in game_state.players.values() if p.team != self.team and p.is_alive]
        allies = [p for p in game_state.players.values() if p.team == self.team and p.id != self.id and p.is_alive]
        
        ally_flag = game_state.flags[self.team]
        enemy_team = "orange" if self.team == "blue" else "blue"
        enemy_flag = game_state.flags[enemy_team]
        
        # Calculate AI priorities based on Strategy Template modifier
        from simulation.config import STRATEGY_TEMPLATES
        strat_mods = STRATEGY_TEMPLATES.get(self.strategy, STRATEGY_TEMPLATES["SPLIT"])
        
        offense_mod = strat_mods["offensive_focus"]
        defense_mod = strat_mods["defensive_focus"]
        retreat_thresh = strat_mods["retreat_threshold"]
        
        # Check for survival first
        health_ratio = (self.hp + self.shield) / (self.max_hp + self.max_shield)
        was_retreating = self.state == "RETREAT"
        ally_has_flag = any(a.has_flag for a in allies)
        
        # 1. State Transitions
        if self.has_flag:
            self.state = "RUN_FLAG"
        elif was_retreating and health_ratio < 0.90:
            # Hysteresis: Stay in RETREAT until mostly healed (90% HP + Shield)
            self.state = "RETREAT"
        elif not ally_flag["at_base"]:
            # Stalker and Enforcer should fight to the death to recover the flag (no retreat).
            # Tacticians can still retreat to preserve healing potential.
            if health_ratio < retreat_thresh and self.class_type == "Tactician":
                self.state = "RETREAT"
            else:
                self.state = "RECOVER_FLAG"
        elif health_ratio < retreat_thresh:
            self.state = "RETREAT"
        elif self.class_type == "Tactician" and any((a.hp / a.max_hp) < 0.6 for a in allies):
            # Tacticians focus on low-health allies
            self.state = "HEAL_ALLIED"
        elif ally_has_flag:
            # If an ally has the enemy flag, join the push to defend them
            self.state = "INFILTRATE"
        elif self.class_type == "Enforcer" and defense_mod > 1.2:
            # Turtle Heavy stays home
            self.state = "PATROL"
        elif self.class_type == "Stalker" or offense_mod > 1.2:
            # Offensive push
            self.state = "INFILTRATE"
        else:
            self.state = "PATROL"

        # 2. State Actions (Find target position & shoot logic)
        self.is_healing = False
        self.healing_target_id = None
        
        if self.state == "RETREAT":
            # Run back to base spawn point or toward Tactician
            tactician_ally = next((a for a in allies if a.class_type == "Tactician"), None)
            if tactician_ally:
                self.target_pos = np.copy(tactician_ally.pos)
            else:
                self.target_pos = np.copy(self.spawn_pos)
            
            # Defensive skills
            if self.class_type == "Enforcer" and self.ability_cooldown <= 0:
                # Turn on front shield
                self.is_shielding = True
                self.ability_active_timer = 2.5
                self.ability_cooldown = 10.0
            elif self.class_type == "Stalker" and self.ability_cooldown <= 0:
                # Dash away
                self.is_dashing = True
                self.ability_active_timer = 0.5
                self.ability_cooldown = 6.0
                # Instant speed burst
                dir_away = self.pos - self.spawn_pos
                dist = np.linalg.norm(dir_away)
                if dist > 0:
                    self.vel -= (dir_away / dist) * 15.0 # dash back
                    
        elif self.state == "RUN_FLAG":
            # Bring flag home
            self.target_pos = np.copy(self.spawn_pos)
            # Use dash if stalker
            if self.class_type == "Stalker" and self.ability_cooldown <= 0:
                self.is_dashing = True
                self.ability_active_timer = 0.5
                self.ability_cooldown = 6.0
                
        elif self.state == "RECOVER_FLAG":
            # Find the enemy carrier
            carrier = next((p for p in enemies if p.has_flag), None)
            if carrier:
                self.target_pos = np.copy(carrier.pos)
            else:
                # If dropped, go to dropped flag pos
                self.target_pos = np.array(ally_flag["pos"])
                
        elif self.state == "HEAL_ALLIED":
            # Find closest damaged ally
            damaged_allies = sorted(allies, key=lambda a: a.hp / a.max_hp)
            if damaged_allies:
                target_ally = damaged_allies[0]
                self.target_pos = np.copy(target_ally.pos)
                
                # Active Healing beam if within range
                dist = np.linalg.norm(self.pos - target_ally.pos)
                if dist < CLASS_STATS["Tactician"]["heal_range"]:
                    self.is_healing = True
                    self.healing_target_id = target_ally.id
                    heal_amt = CLASS_STATS["Tactician"]["heal_rate"] * dt
                    target_ally.heal(heal_amt)
                    self.healing_done += heal_amt
                    
        elif self.state == "INFILTRATE":
            # Rush enemy flag
            if enemy_flag["carrier_id"] is None:
                self.target_pos = np.array(enemy_flag["pos"])
            else:
                # Someone else got it, protect them or path back
                carrier = next((p for p in allies if p.has_flag), None)
                if carrier:
                    self.target_pos = np.copy(carrier.pos)
                else:
                    self.target_pos = np.copy(self.spawn_pos)
                    
        else: # PATROL
            # Patrol base or midfield center - only choose a new target when we reach the current one
            dist_to_patrol = np.linalg.norm(self.pos - self.target_pos)
            if dist_to_patrol < 4.0 or getattr(self, "patrol_target_unset", True):
                self.patrol_target_unset = False
                if self.class_type == "Enforcer":
                    # Guard base
                    target_x = np.clip(self.spawn_pos[0] + float(random.randint(-15, 15)), -95.0, 95.0)
                    target_y = np.clip(self.spawn_pos[1] + float(random.randint(-15, 15)), -95.0, 95.0)
                    self.target_pos = np.array([target_x, target_y, 0.0])
                elif self.class_type == "Tactician":
                    # Stay near base to heal returning allies
                    target_x = np.clip(self.spawn_pos[0] + float(random.randint(-20, 20)), -95.0, 95.0)
                    target_y = np.clip(self.spawn_pos[1] + float(random.randint(-20, 20)), -95.0, 95.0)
                    self.target_pos = np.array([target_x, target_y, 0.0])
                else:
                    # Patrol mid heights
                    self.target_pos = np.array([
                        0.0, 
                        float(random.randint(-40, 40)), 
                        10.0
                    ])

        # 2.5 Combat Cover Override
        # If the player is in an active state (not retreating or running a flag) and has low shields,
        # find the nearest building to take cover and stay there until shields are fully recharged.
        if self.state in ["INFILTRATE", "PATROL", "HEAL_ALLIED"] and not self.has_flag:
            was_taking_cover = getattr(self, "is_taking_cover", False)
            
            # We need cover if shields are under 25%, or we are already taking cover and shields aren't at 75% yet
            needs_cover = (self.shield < self.max_shield * 0.25) or (was_taking_cover and self.shield < self.max_shield * 0.75)
            
            if needs_cover:
                # If we have visible threats, find cover relative to the closest one
                visible_enemies = []
                for e in enemies:
                    dist = np.linalg.norm(self.pos - e.pos)
                    if dist <= self.disc_range + 5.0 and check_line_of_sight(self.pos, e.pos, game_state.map_layout["buildings"]):
                        visible_enemies.append((dist, e))
                
                cover_found = False
                if visible_enemies:
                    visible_enemies.sort(key=lambda x: x[0])
                    closest_visible_enemy = visible_enemies[0][1]
                    cover_pos = find_cover_position(self.pos, closest_visible_enemy.pos, game_state.map_layout["buildings"])
                    if cover_pos is not None:
                        self.cover_target_pos = cover_pos
                        self.is_taking_cover = True
                        cover_found = True
                elif was_taking_cover:
                    # Keep taking cover to heal if we were already doing so
                    cover_found = True
                
                if not cover_found:
                    self.is_taking_cover = False
                
                # If we have a cached cover position, make sure we steer towards it
                if getattr(self, "is_taking_cover", False) and hasattr(self, "cover_target_pos"):
                    self.target_pos = self.cover_target_pos
            else:
                self.is_taking_cover = False
        else:
            self.is_taking_cover = False

        # 3. Pathing and Physics Movement
        # Tron map navigation: we calculate the vector to the target position, 
        # but modify it based on walls / obstacles and ramps.
        
        # Height Level Navigation helper:
        # If the target is at a higher height than us, route through the ramps.
        routing_target = self.target_pos
        player_z = self.pos[2]
        target_z = self.target_pos[2]
        
        if target_z > player_z + 3.0:
            if self.pos[0] < 0: # Blue Side
                if player_z < 3.0:
                    # Target ground ramp entrance
                    entrance = np.array([-74.0, 0.0, 0.0])
                    if np.linalg.norm(self.pos[:2] - entrance[:2]) > 5.0:
                        routing_target = np.array([-74.0, 0.0, player_z])
                    else:
                        routing_target = np.array([-62.0, 0.0, 7.0])
                elif player_z < 9.0:
                    # Target center ramp entrance
                    entrance = np.array([-38.0, 0.0, 7.0])
                    if np.linalg.norm(self.pos[:2] - entrance[:2]) > 5.0:
                        routing_target = np.array([-38.0, 0.0, player_z])
                    else:
                        routing_target = np.array([-31.0, 0.0, 12.0])
            else: # Orange Side
                if player_z < 3.0:
                    entrance = np.array([74.0, 0.0, 0.0])
                    if np.linalg.norm(self.pos[:2] - entrance[:2]) > 5.0:
                        routing_target = np.array([74.0, 0.0, player_z])
                    else:
                        routing_target = np.array([62.0, 0.0, 7.0])
                elif player_z < 9.0:
                    entrance = np.array([38.0, 0.0, 7.0])
                    if np.linalg.norm(self.pos[:2] - entrance[:2]) > 5.0:
                        routing_target = np.array([38.0, 0.0, player_z])
                    else:
                        routing_target = np.array([31.0, 0.0, 12.0])

        # Add dynamic, ID-based spread offsets to keep teammates from mimicking each other or stacking
        spread_angle = (self.id * 1.5) + (self.pos[0] * 0.05) + (self.pos[1] * 0.05)
        spread_dist = 1.5 if self.state == "RUN_FLAG" else 3.5
        routing_target[0] += np.cos(spread_angle) * spread_dist
        routing_target[1] += np.sin(spread_angle) * spread_dist
        routing_target[0] = np.clip(routing_target[0], -95.0, 95.0)
        routing_target[1] = np.clip(routing_target[1], -95.0, 95.0)

        # Navigation waypoint routing around buildings
        nav_target = get_navigation_target(self.pos, routing_target, game_state.map_layout["buildings"], game_state.map_layout.get("platforms"))
        to_target = nav_target - self.pos
        distance = np.linalg.norm(to_target)
        
        # Calculate speed modifier
        speed_mult = 1.0
        if self.is_dashing:
            speed_mult = 2.0
        elif self.is_shielding:
            speed_mult = 0.5
            
        overcharge_mult = 1.3 if self.overcharge_timer > 0.0 else 1.0
        current_speed = self.base_speed * speed_mult * overcharge_mult
        
        # Determine if we should slow down (only do this near the ultimate destination, not at intermediate corners)
        target_distance = np.linalg.norm(self.target_pos - self.pos)
        should_slow_down = (distance <= 1.0) and (target_distance < 2.0)
        
        if not should_slow_down:
            dir_vec = to_target / (distance + 1e-5)
            
            # Simple obstacle avoidance: check if there's a building in front
            # Buildings are defined as bounding boxes in world.py. We avoid colliding with them.
            avoidance_force = np.array([0.0, 0.0, 0.0])
            for b in game_state.map_layout["buildings"]:
                # Simple cylinder approximation for avoidance
                b_center = np.array([b["x"] + b["w"]/2, b["y"] + b["d"]/2, self.pos[2]])
                to_building = self.pos - b_center
                b_dist = np.linalg.norm(to_building)
                # If too close to building center, apply push force outwards
                safe_dist = (b["w"] + b["d"]) / 3.0 + 3.0
                if b_dist < safe_dist:
                    # Check Z height overlap
                    if self.pos[2] >= b["z"] and self.pos[2] <= b["z"] + b["h"]:
                        avoidance_force += (to_building / (b_dist + 0.01)) * 5.0

            # Evade enemies when carrying the flag or retreating by applying a lateral avoidance push
            enemy_avoidance = np.array([0.0, 0.0, 0.0])
            if self.has_flag or self.state == "RETREAT":
                for e in enemies:
                    to_enemy = self.pos - e.pos
                    e_dist = np.linalg.norm(to_enemy)
                    if e_dist < 28.0: # Evade range
                        dir_away = np.array([to_enemy[0], to_enemy[1], 0.0])
                        dir_len = np.linalg.norm(dir_away)
                        if dir_len > 0.001:
                            dir_away = dir_away / dir_len
                            # Scale push inversely proportional to distance, maxing out at 15.0
                            force_scale = min(15.0, 12.0 / (e_dist + 0.1))
                            enemy_avoidance += dir_away * force_scale

            # Wander noise to simulate human-like micro-corrections and break absolute symmetry
            noise_time = (self.id * 12.34) + (self.pos[0] * 0.1) + (self.pos[1] * 0.1)
            wander_strength = 1.2 # slight sway
            wander_force = np.array([
                np.cos(noise_time) * wander_strength,
                np.sin(noise_time * 1.5) * wander_strength,
                0.0
            ])

            # Combined steering
            desired_vel = dir_vec * current_speed + avoidance_force + enemy_avoidance + wander_force
            # Apply interpolation to velocity
            self.vel = 0.7 * self.vel + 0.3 * desired_vel
        else:
            # Smooth deceleration at destination
            self.vel = 0.5 * self.vel
            
        # Record old position for diagnostic stuck detection
        old_pos = np.copy(self.pos)
        collided_building = None
        
        # Update Position
        new_pos = self.pos + self.vel * dt
        
        # Hard collision box resolution with buildings/obstacles
        for b in game_state.map_layout["buildings"]:
            bx1, by1 = b["x"], b["y"]
            bx2, by2 = b["x"] + b["w"], b["y"] + b["d"]
            bz1, bz2 = b["z"], b["z"] + b["h"]
            
            # Check vertical Z overlap: we only collide if player is vertically within building walls
            # (i.e. not standing on top of it)
            if bz1 <= new_pos[2] < bz2 - 0.5 or bz1 <= self.pos[2] < bz2 - 0.5:
                # Add bounding padding for player radius (2.2 units)
                radius = 2.2
                if bx1 - radius <= new_pos[0] <= bx2 + radius and by1 - radius <= new_pos[1] <= by2 + radius:
                    collided_building = b["id"]
                    
                    # Determine valid faces for snapping based on previous position (to prevent clipping through)
                    candidates = []
                    
                    # Left face (snaps to bx1 - radius)
                    if self.pos[0] <= bx1 - radius + 0.05:
                        candidates.append(("left", abs(new_pos[0] - (bx1 - radius))))
                    # Right face
                    if self.pos[0] >= bx2 + radius - 0.05:
                        candidates.append(("right", abs(new_pos[0] - (bx2 + radius))))
                    # Bottom face (snaps to by1 - radius)
                    if self.pos[1] <= by1 - radius + 0.05:
                        candidates.append(("bottom", abs(new_pos[1] - (by1 - radius))))
                    # Top face
                    if self.pos[1] >= by2 + radius - 0.05:
                        candidates.append(("top", abs(new_pos[1] - (by2 + radius))))
                        
                    if candidates:
                        # Choose the valid candidate with the minimum snap distance
                        best_candidate = min(candidates, key=lambda c: c[1])
                        face = best_candidate[0]
                    else:
                        # Fallback to nearest face if somehow already fully inside
                        l_dist = abs(new_pos[0] - (bx1 - radius))
                        r_dist = abs(new_pos[0] - (bx2 + radius))
                        t_dist = abs(new_pos[1] - (by1 - radius))
                        b_dist = abs(new_pos[1] - (by2 + radius))
                        min_dist = min(l_dist, r_dist, t_dist, b_dist)
                        if min_dist == l_dist: face = "left"
                        elif min_dist == r_dist: face = "right"
                        elif min_dist == t_dist: face = "bottom"
                        else: face = "top"
                        
                    # Apply snapping and clip velocity
                    if face == "left":
                        new_pos[0] = bx1 - radius
                        self.vel[0] = min(0.0, self.vel[0])
                    elif face == "right":
                        new_pos[0] = bx2 + radius
                        self.vel[0] = max(0.0, self.vel[0])
                    elif face == "bottom":
                        new_pos[1] = by1 - radius
                        self.vel[1] = min(0.0, self.vel[1])
                    else:
                        new_pos[1] = by2 + radius
                        self.vel[1] = max(0.0, self.vel[1])
                        
        # Restrict to map bounds
        new_pos[0] = np.clip(new_pos[0], -98, 98)
        new_pos[1] = np.clip(new_pos[1], -98, 98)
        
        # Height coordination: platforms and ramps
        # We query the map manager for the correct Z position (snapping to platforms or climbing ramps)
        target_z = 0.0
        on_ramp = False
        
        # Check ramps first
        if "ramps" in game_state.map_layout:
            for ramp in game_state.map_layout["ramps"]:
                rx1, rx2 = ramp["x1"], ramp["x2"]
                ry1, ry2 = ramp["y1"], ramp["y2"]
                rz1, rz2 = ramp["z1"], ramp["z2"]
                
                # Check 2D horizontal footprint overlap
                if rx1 <= new_pos[0] <= rx2 and ry1 <= new_pos[1] <= ry2:
                    # Ramps rise/fall along the X axis in our level layout
                    x_span = rx2 - rx1
                    if x_span > 0:
                        ratio = (new_pos[0] - rx1) / x_span
                        target_z = rz1 + ratio * (rz2 - rz1)
                        on_ramp = True
                        break
                        
        if not on_ramp:
            # Check if player is on any building/platform top
            for platform in game_state.map_layout["platforms"]:
                px, py, pw, pd, pz = platform["x"], platform["y"], platform["w"], platform["d"], platform["z"]
                if px <= new_pos[0] <= px + pw and py <= new_pos[1] <= py + pd:
                    # If player is near the platform height, snap to it (climbing/walking on it)
                    if abs(new_pos[2] - pz) < 6.0 or (self.pos[2] >= pz - 1.0 and self.vel[2] >= -1.0):
                        target_z = pz
                        break
        
        # Simple vertical smoothing (climbing ramps or stepping onto platforms)
        new_pos[2] = 0.8 * self.pos[2] + 0.2 * target_z
        self.pos = new_pos

        # Diagnostic Stuck Detection Logging & Active Unstuck Nudge
        actual_dist = np.linalg.norm(self.pos - old_pos)
        expected_dist = current_speed * dt
        target_dist = np.linalg.norm(self.target_pos - self.pos)
        if expected_dist > 0.1 and actual_dist < expected_dist * 0.05 and distance > 3.0:
            self.stuck_frames = getattr(self, "stuck_frames", 0) + 1
            if self.stuck_frames >= 20: # Stuck for ~0.66 seconds
                import logging
                logger = logging.getLogger("simulation.player")
                logger.warning(
                    f"DIAGNOSTIC: Player '{self.name}' ({self.class_type}) is STUCK at pos {self.pos.round(1)}! "
                    f"Target={self.target_pos.round(1)}, State={self.state}, Collided with building={collided_building}. Applying unstuck nudge."
                )
                # Apply unstuck nudge perpendicular to the target vector to slide around the obstacle
                nudge_dir = self.target_pos - self.pos
                nudge_dir[2] = 0.0 # 2D horizontal plane
                nudge_dist = np.linalg.norm(nudge_dir)
                if nudge_dist > 0.01:
                    # Move perpendicular and slightly forward
                    perp = np.array([-nudge_dir[1], nudge_dir[0], 0.0]) / nudge_dist
                    self.pos += perp * 3.5 + (nudge_dir / nudge_dist) * 1.5
                    self.vel = (perp * 3.5 + (nudge_dir / nudge_dist) * 1.5) / dt
                else:
                    self.pos += np.array([random.choice([-3.0, 3.0]), random.choice([-3.0, 3.0]), 0.0])
                self.stuck_frames = 0
        else:
            self.stuck_frames = 0

        # 4. Combat Logic: Throw Discs & Melee Batons
        if self.disc_cooldown <= 0 and enemies:
            # Find closest enemy
            closest_enemy = min(enemies, key=lambda e: np.linalg.norm(self.pos - e.pos))
            dist_to_enemy = np.linalg.norm(self.pos - closest_enemy.pos)
            
            # Fire projectile (disc) if in range and we have clear line of sight
            if dist_to_enemy <= self.disc_range and check_line_of_sight(self.pos, closest_enemy.pos, game_state.map_layout["buildings"]):
                self.disc_cooldown = self.disc_cooldown_max
                
                # Check for melee batons (if extremely close)
                if dist_to_enemy < 6.0:
                    # Melee Strike!
                    dmg = self.melee_damage
                    # Reduce damage if target is shielding
                    if closest_enemy.is_shielding:
                        dmg *= 0.3
                    killed = closest_enemy.take_damage(dmg, time_now)
                    self.damage_dealt += dmg
                    if killed:
                        self.kills += 1
                else:
                    # Spawn light disc projectile
                    direction = (closest_enemy.pos - self.pos) / dist_to_enemy
                    # Launch slightly above ground (height offset)
                    launch_pos = self.pos + np.array([0.0, 0.0, 1.5])
                    disc = {
                        "id": random.randint(10000, 99999),
                        "owner_id": self.id,
                        "team": self.team,
                        "pos": launch_pos.tolist(),
                        "vel": (direction * self.disc_speed).tolist(),
                        "damage": self.disc_damage,
                        "range_left": self.disc_range,
                        "z_plane": launch_pos[2]
                    }
                    game_state.projectiles.append(disc)

    def to_dict(self) -> dict:
        """Returns JSON-serializable representation of player state."""
        return {
            "id": self.id,
            "name": self.name,
            "team": self.team,
            "class_type": self.class_type,
            "pos": self.pos.tolist(),
            "vel": self.vel.tolist(),
            "hp": round(self.hp, 1),
            "max_hp": self.max_hp,
            "shield": round(self.shield, 1),
            "max_shield": self.max_shield,
            "is_alive": self.is_alive,
            "has_flag": self.has_flag,
            "state": self.state,
            "kills": self.kills,
            "deaths": self.deaths,
            "captures": self.captures,
            "damage_dealt": int(self.damage_dealt),
            "healing_done": int(self.healing_done),
            "is_healing": self.is_healing,
            "is_dashing": self.is_dashing,
            "is_shielding": self.is_shielding,
            "healing_target_id": self.healing_target_id,
            "is_taking_cover": self.is_taking_cover,
            "overcharge_timer": round(self.overcharge_timer, 2)
        }
