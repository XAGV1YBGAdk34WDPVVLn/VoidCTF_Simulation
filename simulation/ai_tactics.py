import json
import logging
import random
from simulation.config import STRATEGY_TEMPLATES

logger = logging.getLogger("simulation.ai_tactics")

# Fallback tactics rationales
FALLBACK_RATIONALES_BLUE = {
    "RUSH": "Blue team will utilize the Stalker's raw speed to blitz the enemy flag, supported by the Tactician's mobile healing. The Enforcer will create a path through the midfield.",
    "TURTLE": "Blue team will establish a tight defensive perimeter around our pedestal. The Enforcer will bodyblock key choke points while the Tactician keeps them fully repaired.",
    "SPLIT": "Blue team will execute a flanking maneuver. The Stalker will circle high, while the Enforcer draws attention in the lower lanes, supported dynamically by the Tactician."
}

FALLBACK_RATIONALES_ORANGE = {
    "RUSH": "Orange team intends to bypass Blue's defenses with a high-velocity offensive push. The Stalker will rush the flag under cover of the Enforcer's heavy shield.",
    "TURTLE": "Orange team will play conservatively, locking down our base. We will wait for Blue to overcommit, defeat them, and then launch a counter-attack.",
    "SPLIT": "Orange team will split. The Enforcer holds the middle platform, the Tactician heals from behind cover, and the Stalker exploits the blind spots to capture the flag."
}

def get_pregame_tactics(team_name: str, team_composition: list) -> dict:
    """
    Returns a customized tactic strategy and rationale instantly using local heuristics.
    """
    strategy = random.choice(["RUSH", "TURTLE", "SPLIT"])
    rationales = FALLBACK_RATIONALES_BLUE if team_name.lower() == "blue" else FALLBACK_RATIONALES_ORANGE
    rationale = rationales.get(strategy, "Execute grid optimization vectors for tactical superiority.")
    logger.info(f"Local tactics generated for {team_name}: {strategy}")
    return {"strategy": strategy, "rationale": rationale, "source": "Tactical Advisor Subroutine"}

def get_match_audit(match_stats: dict) -> str:
    """
    Generates a localized MCP Systems Audit Report evaluating the match based on telemetry.
    """
    winner = match_stats.get("winner", "Tie").upper()
    duration = match_stats.get("duration_seconds", 120)
    blue_caps = match_stats.get("blue_captures", 0)
    orange_caps = match_stats.get("orange_captures", 0)
    blue_strategy = match_stats.get("blue_strategy", "Unknown")
    orange_strategy = match_stats.get("orange_strategy", "Unknown")
    
    # Dynamic telemetry evaluation: Find MVP and peak performance
    perf = match_stats.get("player_performance", {})
    mvp_name = "N/A"
    max_dmg = 0
    for name, stats in perf.items():
        dmg = stats.get("damage_dealt", 0)
        if dmg > max_dmg:
            max_dmg = dmg
            mvp_name = name
            
    audit = f"""=== MCP SYSTEMS AUDIT REPORT ===
TIMESTAMP: SUB-CYCLE {random.randint(1000, 9999)}
MATCH RESULT: Winner - Team {winner}
ELAPSED GRID TIME: {duration} SECONDS

[TACTICAL MATRIX ANALYSIS]
- Team Blue Strategy: {blue_strategy} (Captures: {blue_caps})
- Team Orange Strategy: {orange_strategy} (Captures: {orange_caps})

[CRITICAL SUBROUTINE EVALUATION]
- Telemetry analysis indicates that Team {winner} successfully optimized their coordinate pathing.
- MVP routine '{mvp_name}' achieved top efficiency with {max_dmg} points of directed grid damage.
- Underperforming nodes showed excessive latency during retreat vectors, leading to de-rezzing events.

[SYSTEM RECOMMENDATIONS]
1. Adjust safety subroutines to trigger defensive retreats 10% earlier.
2. Coordinate Stalker sprint bursts to synchronize with Tactician nanite coverage.
3. Establish tighter midfield interception blocks to prevent horizontal grid bypasses.
"""
    return audit
