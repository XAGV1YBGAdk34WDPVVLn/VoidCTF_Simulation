# Simulation Configurations

# Map Parameters
MAP_WIDTH = 200    # X-axis: -100 to 100
MAP_DEPTH = 200    # Y-axis: -100 to 100
MAP_HEIGHT = 50    # Z-axis: 0 to 50 (height levels)

# Game Loop Speed (updates per second)
TICK_RATE = 30
TICK_INTERVAL = 1.0 / TICK_RATE

# Match Conditions
MATCH_TIME_LIMIT = 300  # 5 minutes in seconds
WINNING_CAPTURES = 3

# Respawn Settings
RESPAWN_COOLDOWN = 6.0  # seconds to respawn

# Player Classes and Stats
CLASS_STATS = {
    "Stalker": {
        "max_hp": 80,
        "max_shield": 40,
        "speed": 13.0,
        "melee_damage": 12,
        "disc_damage": 15,
        "disc_cooldown": 0.8,    # seconds between throws
        "disc_speed": 35.0,
        "disc_range": 45.0,
        "shield_regen_rate": 6.0, # per second
        "shield_regen_delay": 4.0 # seconds after taking damage
    },
    "Enforcer": {
        "max_hp": 150,
        "max_shield": 80,
        "speed": 8.0,
        "melee_damage": 25,
        "disc_damage": 30,
        "disc_cooldown": 2.0,
        "disc_speed": 25.0,
        "disc_range": 35.0,
        "shield_regen_rate": 10.0,
        "shield_regen_delay": 5.0
    },
    "Tactician": {
        "max_hp": 100,
        "max_shield": 50,
        "speed": 10.5,
        "melee_damage": 15,
        "disc_damage": 18,
        "disc_cooldown": 1.2,
        "disc_speed": 30.0,
        "disc_range": 40.0,
        "heal_rate": 18.0,       # HP healed per second to ally
        "heal_range": 35.0,
        "shield_regen_rate": 8.0,
        "shield_regen_delay": 4.5
    }
}

# AI Strategy Adjustments (multipliers/overrides)
STRATEGY_TEMPLATES = {
    "RUSH": {
        "offensive_focus": 1.5,
        "defensive_focus": 0.5,
        "retreat_threshold": 0.25, # retreat when HP < 25%
        "description": "All-out blitz targeting the enemy flag. Players play aggressively and push deep."
    },
    "TURTLE": {
        "offensive_focus": 0.4,
        "defensive_focus": 1.8,
        "retreat_threshold": 0.45, # retreat earlier (HP < 45%) to survive
        "description": "Fortify the home base. Heavy stays near the flag, and support prioritizes keeping teammates alive."
    },
    "SPLIT": {
        "offensive_focus": 1.0,
        "defensive_focus": 1.0,
        "retreat_threshold": 0.35, # standard retreat at 35% HP
        "description": "Infiltrator maneuvers stealthily around the flanks, Heavy creates midfield distraction, Support assists dynamically."
    }
}
