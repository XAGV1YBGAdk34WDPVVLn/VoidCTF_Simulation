def get_map_layout() -> dict:
    """
    Returns the geometry of the map layers, buildings, obstacles, ramps, and bases.
    This layout is sent to the frontend so Three.js renders the identical environment
    and the backend computes collisions against the same structures.
    """
    # Elevated Platforms (walkable surfaces)
    # x, y are top-left corners; w, d are width (X) and depth (Y); z is height
    platforms = [
        # Ground layer is represented by Z=0. We define elevated layers.
        # Center platform (High Ground)
        {"id": "p_center", "x": -30.0, "y": -30.0, "w": 60.0, "d": 60.0, "z": 12.0},
        
        # Blue side platform (Medium Ground)
        {"id": "p_blue_side", "x": -60.0, "y": -40.0, "w": 20.0, "d": 80.0, "z": 7.0},
        
        # Orange side platform (Medium Ground)
        {"id": "p_orange_side", "x": 40.0, "y": -40.0, "w": 20.0, "d": 80.0, "z": 7.0},
    ]

    # Buildings & Obstacles (non-walkable solid columns/walls)
    buildings = [
        # Central Pillars
        {"id": "b_mid_1", "x": -5.0, "y": -5.0, "w": 10.0, "d": 10.0, "z": 0.0, "h": 50.0},
        
        # Core structures around center platform
        {"id": "b_corner_bl", "x": -25.0, "y": -25.0, "w": 8.0, "d": 8.0, "z": 0.0, "h": 25.0},
        {"id": "b_corner_tl", "x": -25.0, "y": 17.0, "w": 8.0, "d": 8.0, "z": 0.0, "h": 25.0},
        {"id": "b_corner_br", "x": 17.0, "y": -25.0, "w": 8.0, "d": 8.0, "z": 0.0, "h": 25.0},
        {"id": "b_corner_tr", "x": 17.0, "y": 17.0, "w": 8.0, "d": 8.0, "z": 0.0, "h": 25.0},
        
        # Blue Base Defenses (walls protecting the base)
        {"id": "b_blue_wall_top", "x": -70.0, "y": 20.0, "w": 6.0, "d": 20.0, "z": 0.0, "h": 15.0},
        {"id": "b_blue_wall_bot", "x": -70.0, "y": -40.0, "w": 6.0, "d": 20.0, "z": 0.0, "h": 15.0},
        
        # Orange Base Defenses
        {"id": "b_orange_wall_top", "x": 64.0, "y": 20.0, "w": 6.0, "d": 20.0, "z": 0.0, "h": 15.0},
        {"id": "b_orange_wall_bot", "x": 64.0, "y": -40.0, "w": 6.0, "d": 20.0, "z": 0.0, "h": 15.0},
    ]

    # Spawn positions for players (3 per team)
    spawns = {
        "blue": [
            [-90.0, -10.0, 0.0],  # Pos 1
            [-90.0, 0.0, 0.0],   # Pos 2
            [-90.0, 10.0, 0.0]    # Pos 3
        ],
        "orange": [
            [90.0, -10.0, 0.0],   # Pos 1
            [90.0, 0.0, 0.0],    # Pos 2
            [90.0, 10.0, 0.0]     # Pos 3
        ]
    }

    # Flag Pedestals
    bases = {
        "blue": {"pos": [-80.0, 0.0, 0.0]},
        "orange": {"pos": [80.0, 0.0, 0.0]}
    }

    # Ramps (inclined connectors between height layers)
    # x1, x2 are horizontal boundaries; y1, y2 are depth boundaries; z1, z2 are start/end heights
    ramps = [
        # Blue Ground to Side Platform ramp
        {"id": "r_blue_ground", "x1": -78.0, "x2": -60.0, "y1": -8.0, "y2": 8.0, "z1": 0.0, "z2": 7.0},
        # Orange Ground to Side Platform ramp
        {"id": "r_orange_ground", "x1": 60.0, "x2": 78.0, "y1": -8.0, "y2": 8.0, "z1": 7.0, "z2": 0.0},
        # Blue Side Platform to Center Platform ramp (connects X -60 to -30, Y -10 to 10)
        {"id": "r_blue_center", "x1": -40.0, "x2": -30.0, "y1": -10.0, "y2": 10.0, "z1": 7.0, "z2": 12.0},
        # Orange Side Platform to Center Platform ramp
        {"id": "r_orange_center", "x1": 30.0, "x2": 40.0, "y1": -10.0, "y2": 10.0, "z1": 12.0, "z2": 7.0},
    ]

    return {
        "style": 0,
        "platforms": platforms,
        "buildings": buildings,
        "spawns": spawns,
        "bases": bases,
        "ramps": ramps
    }
