// src/world.rs
// Symmetrical procedural map generator for Void Grid CTF matches.

use std::collections::HashMap;
use rand::Rng;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Platform {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub d: f32,
    pub z: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Building {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub d: f32,
    pub z: f32,
    pub h: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BaseInfo {
    pub pos: [f32; 3],
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Ramp {
    pub id: String,
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub z1: f32,
    pub z2: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MapLayout {
    pub style: i32,
    pub platforms: Vec<Platform>,
    pub buildings: Vec<Building>,
    pub spawns: HashMap<String, Vec<[f32; 3]>>,
    pub bases: HashMap<String, BaseInfo>,
    pub ramps: Vec<Ramp>,
}

pub fn get_map_layout() -> MapLayout {
    // Generate a random map on startup
    generate_random_map()
}

pub fn generate_random_map() -> MapLayout {
    let mut rng = rand::thread_rng();
    let style = rng.gen_range(0..3);
    match style {
        0 => generate_midfield_dome_map(),
        1 => generate_flanking_lanes_map(),
        _ => generate_fortress_map(),
    }
}

// Map Style 0: Central Arena / Midfield Dome
fn generate_midfield_dome_map() -> MapLayout {
    let mut rng = rand::thread_rng();
    
    // Procedural variations
    let center_size = rng.gen_range(50.0..65.0); // Random width/depth for center
    let center_z = rng.gen_range(11.0..14.0);    // Random high ground height
    let side_width = 70.0 - (center_size / 2.0) - 12.0; // Meets the center ramp start precisely
    let side_z = rng.gen_range(6.0..8.5);         // Medium height

    let mut platforms = Vec::new();
    let mut buildings = Vec::new();
    let mut ramps = Vec::new();

    // 1. Elevated platforms
    // Center platform
    platforms.push(Platform {
        id: "p_center".to_string(),
        x: -center_size / 2.0,
        y: -center_size / 2.0,
        w: center_size,
        d: center_size,
        z: center_z,
    });
    // Blue side platform
    platforms.push(Platform {
        id: "p_blue_side".to_string(),
        x: -70.0,
        y: -35.0,
        w: side_width,
        d: 70.0,
        z: side_z,
    });
    // Orange side platform (Mirrored)
    platforms.push(Platform {
        id: "p_orange_side".to_string(),
        x: 70.0 - side_width,
        y: -35.0,
        w: side_width,
        d: 70.0,
        z: side_z,
    });

    // 2. Ramps connecting height levels
    // Blue ground ramp (0.0 to side_z)
    let ramp_g_x1 = -85.0;
    let ramp_g_x2 = -70.0;
    ramps.push(Ramp {
        id: "r_blue_ground".to_string(),
        x1: ramp_g_x1,
        x2: ramp_g_x2,
        y1: 8.0,
        y2: 24.0,
        z1: 0.0,
        z2: side_z,
    });
    // Orange ground ramp (side_z to 0.0) (Mirrored)
    ramps.push(Ramp {
        id: "r_orange_ground".to_string(),
        x1: -ramp_g_x2,
        x2: -ramp_g_x1,
        y1: 8.0,
        y2: 24.0,
        z1: side_z,
        z2: 0.0,
    });

    // Blue center ramp (side_z to center_z)
    let ramp_c_x1 = -center_size / 2.0 - 12.0;
    let ramp_c_x2 = -center_size / 2.0;
    ramps.push(Ramp {
        id: "r_blue_center".to_string(),
        x1: ramp_c_x1,
        x2: ramp_c_x2,
        y1: -10.0,
        y2: 10.0,
        z1: side_z,
        z2: center_z,
    });
    // Orange center ramp (center_z to side_z) (Mirrored)
    ramps.push(Ramp {
        id: "r_orange_center".to_string(),
        x1: -ramp_c_x2,
        x2: -ramp_c_x1,
        y1: -10.0,
        y2: 10.0,
        z1: center_z,
        z2: side_z,
    });

    // 3. Buildings / Pillars
    // Center Pillar
    let core_w = rng.gen_range(7.0..11.0);
    buildings.push(Building {
        id: "b_mid_core".to_string(),
        x: -core_w / 2.0,
        y: -core_w / 2.0,
        w: core_w,
        d: core_w,
        z: 0.0,
        h: 45.0,
    });

    // Outer corner pillars (Symmetric)
    let c_dist_x = center_size / 2.0 - 5.0;
    let c_dist_y = center_size / 2.0 - 5.0;
    let corner_w = 6.0;
    let corner_h = 24.0;

    buildings.push(Building { id: "b_corner_bl".to_string(), x: -c_dist_x - corner_w, y: -c_dist_y - corner_w, w: corner_w, d: corner_w, z: 0.0, h: corner_h });
    buildings.push(Building { id: "b_corner_tl".to_string(), x: -c_dist_x - corner_w, y: c_dist_y, w: corner_w, d: corner_w, z: 0.0, h: corner_h });
    buildings.push(Building { id: "b_corner_br".to_string(), x: c_dist_x, y: -c_dist_y - corner_w, w: corner_w, d: corner_w, z: 0.0, h: corner_h });
    buildings.push(Building { id: "b_corner_tr".to_string(), x: c_dist_x, y: c_dist_y, w: corner_w, d: corner_w, z: 0.0, h: corner_h });

    // Symmetrical Defensive obstacles
    let wall_y1 = rng.gen_range(28.0..35.0);
    let wall_y2 = rng.gen_range(-45.0..-35.0);
    let wall_w = 5.0;
    let wall_d = 16.0;

    buildings.push(Building { id: "b_blue_defense_top".to_string(), x: -74.0, y: wall_y1, w: wall_w, d: wall_d, z: 0.0, h: 14.0 });
    buildings.push(Building { id: "b_blue_defense_bot".to_string(), x: -74.0, y: wall_y2, w: wall_w, d: wall_d, z: 0.0, h: 14.0 });
    buildings.push(Building { id: "b_orange_defense_top".to_string(), x: 74.0 - wall_w, y: wall_y1, w: wall_w, d: wall_d, z: 0.0, h: 14.0 });
    buildings.push(Building { id: "b_orange_defense_bot".to_string(), x: 74.0 - wall_w, y: wall_y2, w: wall_w, d: wall_d, z: 0.0, h: 14.0 });

    // Spawns and bases setup
    let mut spawns = HashMap::new();
    spawns.insert(
        "blue".to_string(),
        vec![
            [-92.0, -10.0, 0.0],
            [-92.0, 0.0, 0.0],
            [-92.0, 10.0, 0.0],
        ],
    );
    spawns.insert(
        "orange".to_string(),
        vec![
            [92.0, -10.0, 0.0],
            [92.0, 0.0, 0.0],
            [92.0, 10.0, 0.0],
        ],
    );

    let mut bases = HashMap::new();
    bases.insert("blue".to_string(), BaseInfo { pos: [-80.0, 0.0, 0.0] });
    bases.insert("orange".to_string(), BaseInfo { pos: [80.0, 0.0, 0.0] });

    MapLayout {
        style: 0,
        platforms,
        buildings,
        spawns,
        bases,
        ramps,
    }
}

// Map Style 1: Flanking Lanes / Divided Ridge
fn generate_flanking_lanes_map() -> MapLayout {
    let mut rng = rand::thread_rng();

    let lane_depth = rng.gen_range(20.0..26.0); // Platform depth
    let lane_y_offset = rng.gen_range(38.0..45.0); // Y position for north/south lanes
    let wall_len = rng.gen_range(65.0..85.0);      // Divider wall length

    let mut platforms = Vec::new();
    let mut buildings = Vec::new();
    let mut ramps = Vec::new();

    // 1. Platforms (North and South lanes)
    platforms.push(Platform {
        id: "p_north".to_string(),
        x: -50.0,
        y: lane_y_offset,
        w: 100.0,
        d: lane_depth,
        z: 7.0,
    });
    platforms.push(Platform {
        id: "p_south".to_string(),
        x: -50.0,
        y: -lane_y_offset - lane_depth,
        w: 100.0,
        d: lane_depth,
        z: 7.0,
    });

    // 2. Ramps connecting lanes to ground
    // North lane ramps
    ramps.push(Ramp {
        id: "r_north_blue".to_string(),
        x1: -68.0,
        x2: -50.0,
        y1: lane_y_offset + lane_depth / 3.0,
        y2: lane_y_offset + 2.0 * lane_depth / 3.0,
        z1: 0.0,
        z2: 7.0,
    });
    ramps.push(Ramp {
        id: "r_north_orange".to_string(),
        x1: 50.0,
        x2: 68.0,
        y1: lane_y_offset + lane_depth / 3.0,
        y2: lane_y_offset + 2.0 * lane_depth / 3.0,
        z1: 7.0,
        z2: 0.0,
    });

    // South lane ramps
    ramps.push(Ramp {
        id: "r_south_blue".to_string(),
        x1: -68.0,
        x2: -50.0,
        y1: -lane_y_offset - 2.0 * lane_depth / 3.0,
        y2: -lane_y_offset - lane_depth / 3.0,
        z1: 0.0,
        z2: 7.0,
    });
    ramps.push(Ramp {
        id: "r_south_orange".to_string(),
        x1: 50.0,
        x2: 68.0,
        y1: -lane_y_offset - 2.0 * lane_depth / 3.0,
        y2: -lane_y_offset - lane_depth / 3.0,
        z1: 7.0,
        z2: 0.0,
    });

    // 3. Central dividing wall (splits visual and direct midfield lanes)
    buildings.push(Building {
        id: "b_divider_wall".to_string(),
        x: -wall_len / 2.0,
        y: -10.0,
        w: wall_len,
        d: 20.0,
        z: 0.0,
        h: 22.0,
    });

    // Symmetrical pillars near pedestals
    let p_dist_x = rng.gen_range(52.0..60.0);
    buildings.push(Building { id: "b_blue_pillar_n".to_string(), x: -p_dist_x, y: 15.0, w: 8.0, d: 8.0, z: 0.0, h: 25.0 });
    buildings.push(Building { id: "b_blue_pillar_s".to_string(), x: -p_dist_x, y: -23.0, w: 8.0, d: 8.0, z: 0.0, h: 25.0 });
    buildings.push(Building { id: "b_orange_pillar_n".to_string(), x: p_dist_x - 8.0, y: 15.0, w: 8.0, d: 8.0, z: 0.0, h: 25.0 });
    buildings.push(Building { id: "b_orange_pillar_s".to_string(), x: p_dist_x - 8.0, y: -23.0, w: 8.0, d: 8.0, z: 0.0, h: 25.0 });

    let mut spawns = HashMap::new();
    spawns.insert("blue".to_string(), vec![[-90.0, -10.0, 0.0], [-90.0, 0.0, 0.0], [-90.0, 10.0, 0.0]]);
    spawns.insert("orange".to_string(), vec![[90.0, -10.0, 0.0], [90.0, 0.0, 0.0], [90.0, 10.0, 0.0]]);

    let mut bases = HashMap::new();
    bases.insert("blue".to_string(), BaseInfo { pos: [-80.0, 0.0, 0.0] });
    bases.insert("orange".to_string(), BaseInfo { pos: [80.0, 0.0, 0.0] });

    MapLayout {
        style: 1,
        platforms,
        buildings,
        spawns,
        bases,
        ramps,
    }
}

// Map Style 2: Symmetrical Fortress Tier
fn generate_fortress_map() -> MapLayout {
    let mut rng = rand::thread_rng();

    let fort_w = rng.gen_range(25.0..35.0); // Fortress platform width
    let fort_d = rng.gen_range(50.0..70.0); // Fortress platform depth
    let center_size = rng.gen_range(25.0..35.0); // High ground platform size

    let mut platforms = Vec::new();
    let mut buildings = Vec::new();
    let mut ramps = Vec::new();

    // 1. Platforms
    // Blue fortress platform
    platforms.push(Platform {
        id: "p_blue_fort".to_string(),
        x: -fort_w - 30.0,
        y: -fort_d / 2.0,
        w: fort_w,
        d: fort_d,
        z: 7.0,
    });
    // Orange fortress platform (Mirrored)
    platforms.push(Platform {
        id: "p_orange_fort".to_string(),
        x: 30.0,
        y: -fort_d / 2.0,
        w: fort_w,
        d: fort_d,
        z: 7.0,
    });
    // High center platform
    platforms.push(Platform {
        id: "p_mid_high".to_string(),
        x: -center_size / 2.0,
        y: -center_size / 2.0,
        w: center_size,
        d: center_size,
        z: 12.0,
    });

    // 2. Ramps
    // Blue fortress ground ramp
    ramps.push(Ramp {
        id: "r_fort_blue_g".to_string(),
        x1: -fort_w - 45.0,
        x2: -fort_w - 30.0,
        y1: 8.0,
        y2: 24.0,
        z1: 0.0,
        z2: 7.0,
    });
    // Orange fortress ground ramp (Mirrored)
    ramps.push(Ramp {
        id: "r_fort_orange_g".to_string(),
        x1: fort_w + 30.0,
        x2: fort_w + 45.0,
        y1: 8.0,
        y2: 24.0,
        z1: 7.0,
        z2: 0.0,
    });

    // Blue fortress center ramp (Fortress to Center Platform)
    ramps.push(Ramp {
        id: "r_fort_blue_c".to_string(),
        x1: -30.0,
        x2: -center_size / 2.0,
        y1: -8.0,
        y2: 8.0,
        z1: 7.0,
        z2: 12.0,
    });
    // Orange fortress center ramp (Mirrored)
    ramps.push(Ramp {
        id: "r_fort_orange_c".to_string(),
        x1: center_size / 2.0,
        x2: 30.0,
        y1: -8.0,
        y2: 8.0,
        z1: 12.0,
        z2: 7.0,
    });

    // 3. Buildings / Fortress Shields
    let wall_y1 = fort_d / 2.0 - 15.0;
    let wall_y2 = -fort_d / 2.0 + 5.0;
    let wall_w = 6.0;
    let wall_d = 12.0;

    buildings.push(Building { id: "b_blue_fort_wall".to_string(), x: -fort_w - 20.0, y: wall_y1, w: wall_w, d: wall_d, z: 0.0, h: 25.0 });
    buildings.push(Building { id: "b_blue_fort_wall_2".to_string(), x: -fort_w - 20.0, y: wall_y2, w: wall_w, d: wall_d, z: 0.0, h: 25.0 });
    buildings.push(Building { id: "b_orange_fort_wall".to_string(), x: fort_w + 14.0, y: wall_y1, w: wall_w, d: wall_d, z: 0.0, h: 25.0 });
    buildings.push(Building { id: "b_orange_fort_wall_2".to_string(), x: fort_w + 14.0, y: wall_y2, w: wall_w, d: wall_d, z: 0.0, h: 25.0 });

    // Midfield pillars (outside the platforms)
    buildings.push(Building { id: "b_mid_pillar_n".to_string(), x: -4.0, y: 22.0, w: 8.0, d: 8.0, z: 0.0, h: 30.0 });
    buildings.push(Building { id: "b_mid_pillar_s".to_string(), x: -4.0, y: -30.0, w: 8.0, d: 8.0, z: 0.0, h: 30.0 });

    let mut spawns = HashMap::new();
    spawns.insert("blue".to_string(), vec![[-90.0, -10.0, 0.0], [-90.0, 0.0, 0.0], [-90.0, 10.0, 0.0]]);
    spawns.insert("orange".to_string(), vec![[90.0, -10.0, 0.0], [90.0, 0.0, 0.0], [90.0, 10.0, 0.0]]);

    let mut bases = HashMap::new();
    bases.insert("blue".to_string(), BaseInfo { pos: [-80.0, 0.0, 0.0] });
    bases.insert("orange".to_string(), BaseInfo { pos: [80.0, 0.0, 0.0] });

    MapLayout {
        style: 2,
        platforms,
        buildings,
        spawns,
        bases,
        ramps,
    }
}
