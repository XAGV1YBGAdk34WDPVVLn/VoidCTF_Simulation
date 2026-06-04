import logging
import math
import time
import random
import numpy as np
from simulation.config import TICK_INTERVAL, MATCH_TIME_LIMIT, WINNING_CAPTURES, RESPAWN_COOLDOWN
from simulation.world import get_map_layout
from simulation.player import Player
from simulation.ai_tactics import get_pregame_tactics, get_match_audit

logger = logging.getLogger("simulation.engine")

class GameEngine:
    def __init__(self):
        self.map_layout = get_map_layout()
        self.state = "PREGAME"  # PREGAME, RUNNING, POSTGAME, AUDITING
        self.timer = 15.0  # Pregame timer starts at 15 seconds
        self.match_time = MATCH_TIME_LIMIT
        
        # Team Names & Scores
        self.scores = {"blue": 0, "orange": 0}
        
        # Players
        self.players = {}
        self.init_players()
        
        # Flags
        self.flags = {
            "blue": {
                "team": "blue",
                "pos": [-80.0, 0.0, 0.0],
                "carrier_id": None,
                "at_base": True
            },
            "orange": {
                "team": "orange",
                "pos": [80.0, 0.0, 0.0],
                "carrier_id": None,
                "at_base": True
            }
        }
        
        # Active projectles (light discs)
        self.projectiles = []
        
        # Match statistics log
        self.match_log = []
        self.start_time = 0.0
        self.end_time = 0.0
        
        # Tactical Strategy & Audit State
        self.tactics = {
            "blue": {"strategy": "SPLIT", "rationale": "Initializing grid protocols...", "source": "Default"},
            "orange": {"strategy": "SPLIT", "rationale": "Initializing grid protocols...", "source": "Default"}
        }
        self.audit_report = None
        self.audit_loading = False
        self.sim_time = 0.0

        # Midfield overcharge node initialization
        node_z = 0.0
        for p in self.map_layout.get("platforms", []):
            if p["x"] <= 0.0 <= p["x"] + p["w"] and p["y"] <= 0.0 <= p["y"] + p["d"]:
                node_z = p["z"]
                break
        self.overcharge_node = {
            "pos": [0.0, 0.0, node_z],
            "active": True,
            "respawn_timer": 0.0
        }

    def init_players(self):
        # 3 Blue Players
        blue_spawns = self.map_layout["spawns"]["blue"]
        self.players[0] = Player(0, "A-Tron", "blue", "Stalker", blue_spawns[0])
        self.players[1] = Player(1, "B-Block", "blue", "Enforcer", blue_spawns[1])
        self.players[2] = Player(2, "C-Medic", "blue", "Tactician", blue_spawns[2])
        
        # 3 Orange Players
        orange_spawns = self.map_layout["spawns"]["orange"]
        self.players[3] = Player(3, "X-Vector", "orange", "Stalker", orange_spawns[0])
        self.players[4] = Player(4, "Y-Shield", "orange", "Enforcer", orange_spawns[1])
        self.players[5] = Player(5, "Z-Solder", "orange", "Tactician", orange_spawns[2])

    def fetch_tactics_async(self):
        """Called at start of PREGAME to initialize strategies for both teams."""
        logger.info("Triggering pregame tactics initialization...")
        # To avoid blocking the engine's tick loop, we run this in a thread or 
        # let the Tornado server do it in the background. For simplicity, we can do it 
        # in the server and push strategies via set_strategies.
        # But we'll write the set_strategies function here to receive them.
        pass

    def apply_strategies(self, blue_strategy: dict, orange_strategy: dict):
        """Applies strategies to all players."""
        self.tactics["blue"] = blue_strategy
        self.tactics["orange"] = orange_strategy
        
        # Update strategy state in each player
        for p in self.players.values():
            if p.team == "blue":
                p.strategy = blue_strategy["strategy"]
            else:
                p.strategy = orange_strategy["strategy"]
        self.log_event(f"Tactic applied: Blue -> {blue_strategy['strategy']} | Orange -> {orange_strategy['strategy']}")

    def log_event(self, msg: str):
        self.match_log.append(f"[{round(MATCH_TIME_LIMIT - self.match_time, 1)}s] {msg}")
        logger.info(msg)

    def reset_match(self):
        """Resets the game state for a new match."""
        self.state = "PREGAME"
        self.timer = 15.0
        self.match_time = MATCH_TIME_LIMIT
        self.scores = {"blue": 0, "orange": 0}
        self.projectiles.clear()
        self.match_log.clear()
        self.audit_report = None
        self.audit_loading = False
        self.sim_time = 0.0
        
        # Reset players
        for p in self.players.values():
            p.hp = p.max_hp
            p.shield = p.max_shield
            p.is_alive = True
            p.has_flag = False
            p.pos = np.copy(p.spawn_pos)
            p.vel = np.array([0.0, 0.0, 0.0])
            p.kills = 0
            p.deaths = 0
            p.captures = 0
            p.damage_dealt = 0
            p.healing_done = 0
            p.state = "SPAWNING"
            p.is_healing = False
            p.is_dashing = False
            p.is_shielding = False
            p.healing_target_id = None
            
        # Reset flags
        self.flags["blue"]["carrier_id"] = None
        self.flags["blue"]["pos"] = list(self.map_layout["bases"]["blue"]["pos"])
        self.flags["blue"]["at_base"] = True
        
        self.flags["orange"]["carrier_id"] = None
        self.flags["orange"]["pos"] = list(self.map_layout["bases"]["orange"]["pos"])
        self.flags["orange"]["at_base"] = True
        
        # Reset midfield overcharge node
        node_z = 0.0
        for p in self.map_layout.get("platforms", []):
            if p["x"] <= 0.0 <= p["x"] + p["w"] and p["y"] <= 0.0 <= p["y"] + p["d"]:
                node_z = p["z"]
                break
        self.overcharge_node = {
            "pos": [0.0, 0.0, node_z],
            "active": True,
            "respawn_timer": 0.0
        }

        self.log_event("Grid rebooted. Preparing match parameters...")

    def update(self, dt: float, time_now: float):
        """Core engine tick."""
        self.sim_time += dt
        if self.state == "PREGAME":
            self.timer -= dt
            if self.timer <= 0:
                self.state = "RUNNING"
                self.start_time = time.time()
                self.log_event("Match started! Grid is ACTIVE.")
                
        elif self.state == "RUNNING":
            self.match_time -= dt

            # Update overcharge node
            active = self.overcharge_node.get("active", False)
            respawn_timer = self.overcharge_node.get("respawn_timer", 0.0)
            node_pos = self.overcharge_node["pos"]
            
            if not active:
                respawn_timer -= dt
                if respawn_timer <= 0.0:
                    active = True
                    respawn_timer = 0.0
                    self.log_event("MIDFIELD OVERCHARGE NODE spawned!")
                    
            if active:
                for p in self.players.values():
                    if p.is_alive:
                        h_dist = np.linalg.norm(p.pos[:2] - np.array(node_pos[:2]))
                        v_dist = abs(p.pos[2] - node_pos[2])
                        if h_dist < 4.0 and v_dist < 4.0:
                            p.overcharge_timer = 6.0
                            p.shield = p.max_shield * 1.5
                            active = False
                            respawn_timer = 30.0
                            self.log_event(f"{p.name} picked up the MIDFIELD OVERCHARGE!")
                            break
                            
            self.overcharge_node["active"] = active
            self.overcharge_node["respawn_timer"] = respawn_timer

            if self.match_time <= 0:
                self.end_match()
                return
                
            # Update players
            for p in self.players.values():
                p.update_timers(dt, time_now)
                p.choose_action(self, dt, time_now)
                
            # Update projectiles and detect collisions
            self.update_projectiles(dt, time_now)
            
            # Update flag physics and state
            self.update_flags()
            
            # Check score/win threshold
            if self.scores["blue"] >= WINNING_CAPTURES:
                self.end_match(winner="blue")
            elif self.scores["orange"] >= WINNING_CAPTURES:
                self.end_match(winner="orange")

    def update_projectiles(self, dt: float, time_now: float):
        active_projectiles = []
        for proj in self.projectiles:
            pos = np.array(proj["pos"])
            vel = np.array(proj["vel"])
            
            # Move projectile
            new_pos = pos + vel * dt
            delta_move_len = np.linalg.norm(vel * dt)
            proj["range_left"] -= delta_move_len
            
            bounces = proj.get("bounces", 0)
            hit = False
            
            # 1. Map boundaries check
            if abs(new_pos[0]) > 99.0:
                if bounces < 2:
                    vel[0] = -vel[0]
                    new_pos[0] = 99.0 if new_pos[0] > 0 else -99.0
                    bounces += 1
                else:
                    hit = True
            if not hit and abs(new_pos[1]) > 99.0:
                if bounces < 2:
                    vel[1] = -vel[1]
                    new_pos[1] = 99.0 if new_pos[1] > 0 else -99.0
                    bounces += 1
                else:
                    hit = True
                
            # 2. Buildings (obstacles) collisions
            if not hit:
                for b in self.map_layout["buildings"]:
                    # Box check
                    bx, by, bw, bd, bz, bh = b["x"], b["y"], b["w"], b["d"], b["z"], b["h"]
                    if bx <= new_pos[0] <= bx + bw and by <= new_pos[1] <= by + bd:
                        if bz <= new_pos[2] <= bz + bh:
                            if bounces < 2:
                                crossed_x = pos[0] < bx or pos[0] > bx + bw
                                crossed_y = pos[1] < by or pos[1] > by + bd
                                if crossed_x and not crossed_y:
                                    vel[0] = -vel[0]
                                    new_pos[0] = bx + bw + 0.05 if vel[0] > 0 else bx - 0.05
                                elif crossed_y and not crossed_x:
                                    vel[1] = -vel[1]
                                    new_pos[1] = by + bd + 0.05 if vel[1] > 0 else by - 0.05
                                else:
                                    vel[0] = -vel[0]
                                    vel[1] = -vel[1]
                                    new_pos[0] = bx + bw + 0.05 if vel[0] > 0 else bx - 0.05
                                    new_pos[1] = by + bd + 0.05 if vel[1] > 0 else by - 0.05
                                bounces += 1
                            else:
                                hit = True
                            break
                            
            # 3. Player collisions
            if not hit:
                owner = self.players.get(proj["owner_id"])
                for player in self.players.values():
                    if player.is_alive and player.team != proj["team"]:
                        # Simple cylinder bounding box check
                        h_dist = np.linalg.norm(player.pos[:2] - new_pos[:2])
                        v_dist = abs(player.pos[2] + 1.0 - new_pos[2])  # Shift to center of height
                        
                        if h_dist < 3.0 and v_dist < 4.0:
                            # Direct Hit!
                            dmg = proj["damage"]
                            
                            # Reduce damage if player is an Enforcer and shielding
                            if player.is_shielding:
                                dmg *= 0.3 # 70% damage reduction
                                
                            de_rezzed = player.take_damage(dmg, time_now)
                            hit = True
                            
                            # Log damage
                            if owner:
                                owner.damage_dealt += dmg
                                if de_rezzed:
                                    owner.kills += 1
                                    self.log_event(f"{owner.name} ({owner.class_type}) de-rezzed {player.name} ({player.class_type})")
                            break
                            
            # 4. Out of range check
            if proj["range_left"] <= 0:
                hit = True
                
            # Keep projectile if it didn't impact anything
            if not hit:
                proj["pos"] = new_pos.tolist()
                proj["vel"] = vel.tolist()
                proj["bounces"] = bounces
                active_projectiles.append(proj)
                
        self.projectiles = active_projectiles

    def update_flags(self):
        blue_base_pos = np.array(self.map_layout["bases"]["blue"]["pos"])
        orange_base_pos = np.array(self.map_layout["bases"]["orange"]["pos"])
        
        # 1. Update Carrier Position Tracking
        for team in ["blue", "orange"]:
            flag = self.flags[team]
            if flag["carrier_id"] is not None:
                carrier = self.players[flag["carrier_id"]]
                if not carrier.is_alive:
                    # Carrier was de-rezzed! Flag drops.
                    self.log_event(f"{team.capitalize()} Flag dropped by {carrier.name}!")
                    flag["carrier_id"] = None
                    flag["at_base"] = False
                    # Drop at carrier's last pos
                    flag["pos"] = carrier.pos.tolist()
                else:
                    # Flag follows carrier (centered slightly above)
                    flag["pos"] = (carrier.pos + np.array([0.0, 0.0, 2.0])).tolist()

        # 2. Check Interceptions & Grabs
        for team in ["blue", "orange"]:
            flag = self.flags[team]
            opposing_team = "orange" if team == "blue" else "blue"
            
            # If flag is NOT carried by anyone
            if flag["carrier_id"] is None:
                flag_pos = np.array(flag["pos"])
                for player in self.players.values():
                    if not player.is_alive:
                        continue
                        
                    dist = np.linalg.norm(player.pos - flag_pos)
                    # Interaction radius
                    if dist < 4.5:
                        if player.team == opposing_team:
                            # Stole or picked up enemy flag!
                            flag["carrier_id"] = player.id
                            flag["at_base"] = False
                            player.has_flag = True
                            self.log_event(f"{player.name} ({player.team.capitalize()}) secured {team.capitalize()} Flag!")
                        elif player.team == team and not flag["at_base"]:
                            # Touched own dropped flag -> Return it!
                            flag["pos"] = list(blue_base_pos if team == "blue" else orange_base_pos)
                            flag["at_base"] = True
                            self.log_event(f"{player.name} returned the {team.capitalize()} Flag to base!")

        # 3. Check Captures/Scoring
        # Blue player scores by bringing Orange Flag to Blue Base (while Blue Flag is at home)
        blue_flag = self.flags["blue"]
        orange_flag = self.flags["orange"]
        
        if orange_flag["carrier_id"] is not None:
            carrier = self.players[orange_flag["carrier_id"]]
            dist_to_base = np.linalg.norm(carrier.pos - blue_base_pos)
            if dist_to_base < 5.0:
                if blue_flag["at_base"]:
                    # SCORE!
                    self.scores["blue"] += 1
                    carrier.captures += 1
                    carrier.has_flag = False
                    self.log_event(f"SCORE! Blue Team captures Orange Flag! Blue: {self.scores['blue']} | Orange: {self.scores['orange']}")
                    
                    # Reset orange flag
                    orange_flag["carrier_id"] = None
                    orange_flag["pos"] = orange_base_pos.tolist()
                    orange_flag["at_base"] = True
                else:
                    # Allied flag is stolen, cannot score yet!
                    pass
                    
        if blue_flag["carrier_id"] is not None:
            carrier = self.players[blue_flag["carrier_id"]]
            dist_to_base = np.linalg.norm(carrier.pos - orange_base_pos)
            if dist_to_base < 5.0:
                if orange_flag["at_base"]:
                    # SCORE!
                    self.scores["orange"] += 1
                    carrier.captures += 1
                    carrier.has_flag = False
                    self.log_event(f"SCORE! Orange Team captures Blue Flag! Blue: {self.scores['blue']} | Orange: {self.scores['orange']}")
                    
                    # Reset blue flag
                    blue_flag["carrier_id"] = None
                    blue_flag["pos"] = blue_base_pos.tolist()
                    blue_flag["at_base"] = True
                else:
                    # Allied flag is stolen, cannot score yet!
                    pass

        # Check for comeback strategy shift
        if self.scores["blue"] == 2 and self.scores["orange"] == 0:
            if self.tactics["orange"]["strategy"] != "RUSH":
                self.trigger_comeback("orange")
        elif self.scores["orange"] == 2 and self.scores["blue"] == 0:
            if self.tactics["blue"]["strategy"] != "RUSH":
                self.trigger_comeback("blue")

    def trigger_comeback(self, team: str):
        self.log_event(f"TACTICAL SHIFT: {team.upper()} Team triggers COMEBACK strategy: RUSH!")
        for p in self.players.values():
            if p.team == team:
                p.strategy = "RUSH"
        self.tactics[team]["strategy"] = "RUSH"
        self.tactics[team]["rationale"] = "Auto-triggered fallback due to 2-0 score deficit."
        self.tactics[team]["source"] = "Engine"

    def end_match(self, winner: str = None):
        self.state = "POSTGAME"
        self.end_time = time.time()
        
        # Determine winner by scores if not explicitly provided
        if winner is None:
            if self.scores["blue"] > self.scores["orange"]:
                winner = "blue"
            elif self.scores["orange"] > self.scores["blue"]:
                winner = "orange"
            else:
                winner = "tie"
                
        self.log_event(f"GRID CLOSED. Match Winner: {winner.capitalize() if winner != 'tie' else 'TIE'}!")
        
        # Compile stats for auditor
        player_stats = {}
        for pid, p in self.players.items():
            player_stats[p.name] = {
                "team": p.team,
                "class": p.class_type,
                "kills": p.kills,
                "deaths": p.deaths,
                "captures": p.captures,
                "damage_dealt": p.damage_dealt,
                "healing_done": p.healing_done
            }
            
        self.summary_stats = {
            "winner": winner.capitalize(),
            "duration_seconds": int(MATCH_TIME_LIMIT - self.match_time),
            "blue_captures": self.scores["blue"],
            "orange_captures": self.scores["orange"],
            "blue_strategy": self.tactics["blue"]["strategy"],
            "orange_strategy": self.tactics["orange"]["strategy"],
            "player_performance": player_stats,
            "match_logs": self.match_log[-20:] # Last 20 logs for context
        }

    def to_dict(self) -> dict:
        """Serializes current frame state for client WebSockets."""
        return {
            "state": self.state,
            "timer": round(self.timer, 1),
            "match_time": round(self.match_time, 1),
            "scores": self.scores,
            "players": {pid: p.to_dict() for pid, p in self.players.items()},
            "flags": self.flags,
            "projectiles": self.projectiles,
            "tactics": self.tactics,
            "audit_report": self.audit_report,
            "audit_loading": self.audit_loading,
            "sim_time": self.sim_time,
            "overcharge_node": self.overcharge_node,
            "logs": self.match_log
        }
