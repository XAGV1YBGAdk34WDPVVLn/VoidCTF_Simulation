// src/ai_tactics.rs
// Pregame tactics generation and match auditing telemetry reporter.

use rand::seq::SliceRandom;

pub fn get_pregame_tactics(team_name: &str) -> serde_json::Value {
    let strategies = vec!["RUSH", "TURTLE", "SPLIT", "FLANK", "HARASS", "COUNTER"];
    let mut rng = rand::thread_rng();
    let strategy = strategies.choose(&mut rng).unwrap_or(&"SPLIT");

    let rationale = match team_name.to_lowercase().as_str() {
        "blue" => match *strategy {
            "RUSH" => "Blue team will utilize the Stalker's raw speed to blitz the enemy flag, supported by the Tactician's mobile healing. The Enforcer will create a path through the midfield.",
            "TURTLE" => "Blue team will establish a tight defensive perimeter around our pedestal. The Enforcer will bodyblock key choke points while the Tactician keeps them fully repaired.",
            "SPLIT" => "Blue team will execute a split lane push. The Stalker will circle high, while the Enforcer draws attention in the lower lanes, supported dynamically by the Tactician.",
            "FLANK" => "Blue team will deploy the Stalker along the elevated outer rim to sneak into Orange's flag chamber, while the Enforcer holds the front gateway.",
            "HARASS" => "Blue team will establish midfield containment. The Enforcer and Tactician will engage in aggressive skirmishes to block Orange from staging transitions.",
            _ => "Blue team will secure base defense first, using the Enforcer to check enemy rushes before launching a rapid counter-offensive on the flag.", // COUNTER
        },
        _ => match *strategy {
            "RUSH" => "Orange team intends to bypass Blue's defenses with a high-velocity offensive push. The Stalker will rush the flag under cover of the Enforcer's heavy shield.",
            "TURTLE" => "Orange team will play conservatively, locking down our base. We will wait for Blue to overcommit, defeat them, and then launch a counter-attack.",
            "SPLIT" => "Orange team will split. The Enforcer holds the middle platform, the Tactician heals from behind cover, and the Stalker exploits the blind spots to capture the flag.",
            "FLANK" => "Orange team will execute a flanking maneuver. The Stalker will utilize elevated lanes while the Enforcer and Tactician create a heavy distraction.",
            "HARASS" => "Orange team will deploy aggressive midfield brawlers to lock down the high platforms and intercept Blue before they can organize captures.",
            _ => "Orange team will absorb Blue's initial offensive wave, keeping nodes alive, and strike back with a rapid counter-push once Blue overcommits.", // COUNTER
        },
    };

    serde_json::json!({
        "strategy": strategy,
        "rationale": rationale,
        "source": "Tactical Advisor Subroutine"
    })
}

pub fn get_match_audit(match_stats: &serde_json::Value) -> String {
    let winner_team = match_stats.get("winner_team_name").and_then(|v| v.as_str()).unwrap_or("Tie").to_uppercase();
    let blue_team_name = match_stats.get("blue_team_name").and_then(|v| v.as_str()).unwrap_or("Blue Team").to_string();
    let orange_team_name = match_stats.get("orange_team_name").and_then(|v| v.as_str()).unwrap_or("Orange Team").to_string();

    let winner_key = match_stats.get("winner").and_then(|v| v.as_str()).unwrap_or("tie");
    let winner_team_lower = if winner_key == "blue" {
        blue_team_name.clone()
    } else if winner_key == "orange" {
        orange_team_name.clone()
    } else {
        "Tie".to_string()
    };

    let duration = match_stats.get("duration_seconds").and_then(|v| v.as_i64()).unwrap_or(120);
    let blue_caps = match_stats.get("blue_captures").and_then(|v| v.as_i64()).unwrap_or(0);
    let orange_caps = match_stats.get("orange_captures").and_then(|v| v.as_i64()).unwrap_or(0);
    let blue_strategy = match_stats.get("blue_strategy").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let orange_strategy = match_stats.get("orange_strategy").and_then(|v| v.as_str()).unwrap_or("Unknown");

    // Extract performance stats
    let mut mvp_name = "N/A".to_string();
    let mut max_dmg = 0;
    if let Some(perf) = match_stats.get("player_performance").and_then(|v| v.as_object()) {
        for (name, stats) in perf {
            let dmg = stats.get("damage_dealt").and_then(|v| v.as_i64()).unwrap_or(0);
            if dmg > max_dmg {
                max_dmg = dmg;
                mvp_name = name.clone();
            }
        }
    }

    let sub_cycle = rand::random::<u32>() % 9000 + 1000;

    format!(
        "=== MCP SYSTEMS AUDIT REPORT ===\n\
         TIMESTAMP: SUB-CYCLE {}\n\
         MATCH RESULT: Winner - Team {}\n\
         ELAPSED GRID TIME: {} SECONDS\n\n\
         [TACTICAL MATRIX ANALYSIS]\n\
         - Team {} Strategy: {} (Captures: {})\n\
         - Team {} Strategy: {} (Captures: {})\n\n\
         [CRITICAL SUBROUTINE EVALUATION]\n\
         - Telemetry analysis indicates that Team {} successfully optimized their coordinate pathing.\n\
         - MVP routine '{}' achieved top efficiency with {} points of directed grid damage.\n\
         - Underperforming nodes showed excessive latency during retreat vectors, leading to de-rezzing events.\n\n\
         [SYSTEM RECOMMENDATIONS]\n\
         1. Adjust safety subroutines to trigger defensive retreats 10% earlier.\n\
         2. Coordinate Stalker sprint bursts to synchronize with Tactician nanite coverage.\n\
         3. Establish tighter midfield interception blocks to prevent horizontal grid bypasses.\n",
        sub_cycle, winner_team, duration, blue_team_name, blue_strategy, blue_caps, orange_team_name, orange_strategy, orange_caps, winner_team_lower, mvp_name, max_dmg
    )
}
