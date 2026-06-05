// src/tournament.rs
// Tournament state machine and data models for 4-team Pool & Bracket system.

use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentTeam {
    pub name: String,
    pub color_name: String,      // "pink", "cyan", "green", "purple"
    pub primary_hex: String,      // hex color code
    pub strategy_archetype: String, // "RUSH", "TURTLE", "SPLIT"
    pub description: String,
    pub match_wins: u32,
    pub match_losses: u32,
    pub championships: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentMatch {
    pub id: usize,
    pub name: String,             // "Pool A", "Pool B", "Finals"
    pub blue_team_index: usize,
    pub orange_team_index: usize,
    pub winner_team_index: Option<usize>,
    pub blue_score: Option<u32>,
    pub orange_score: Option<u32>,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentState {
    pub teams: Vec<TournamentTeam>,
    pub matches: Vec<TournamentMatch>,
    pub current_match_index: usize,
    pub champion_index: Option<usize>,
    pub is_active: bool,
}

impl TournamentState {
    pub fn new() -> Self {
        let teams = vec![
            TournamentTeam {
                name: "Aero Strike".to_string(),
                color_name: "pink".to_string(),
                primary_hex: "#ff007f".to_string(),
                strategy_archetype: "RUSH".to_string(),
                description: "Hyper-offensive squad. Focuses on speed and aggressive blitz runs.".to_string(),
                match_wins: 0,
                match_losses: 0,
                championships: 0,
            },
            TournamentTeam {
                name: "Neon Aegis".to_string(),
                color_name: "cyan".to_string(),
                primary_hex: "#00e5ff".to_string(),
                strategy_archetype: "TURTLE".to_string(),
                description: "Defensive masters. Prioritizes fortifying base and protecting flag.".to_string(),
                match_wins: 0,
                match_losses: 0,
                championships: 0,
            },
            TournamentTeam {
                name: "Grid Reapers".to_string(),
                color_name: "green".to_string(),
                primary_hex: "#39ff14".to_string(),
                strategy_archetype: "SPLIT".to_string(),
                description: "Balanced tacticians. Adapts dynamically and coordinates mid-field.".to_string(),
                match_wins: 0,
                match_losses: 0,
                championships: 0,
            },
            TournamentTeam {
                name: "Plasma Void".to_string(),
                color_name: "purple".to_string(),
                primary_hex: "#bd00ff".to_string(),
                strategy_archetype: "SPLIT".to_string(),
                description: "Flex control roster. Excels in long-range disc control and healing.".to_string(),
                match_wins: 0,
                match_losses: 0,
                championships: 0,
            },
            TournamentTeam {
                name: "Solar Flare".to_string(),
                color_name: "yellow".to_string(),
                primary_hex: "#ffff00".to_string(),
                strategy_archetype: "RUSH".to_string(),
                description: "High-velocity solar blitzers. Relies on speed and intense solar bursts.".to_string(),
                match_wins: 0,
                match_losses: 0,
                championships: 0,
            },
            TournamentTeam {
                name: "Apex Shadow".to_string(),
                color_name: "red".to_string(),
                primary_hex: "#ff3333".to_string(),
                strategy_archetype: "TURTLE".to_string(),
                description: "Stealth and defensive agents. Moves from the shadows to guard the base.".to_string(),
                match_wins: 0,
                match_losses: 0,
                championships: 0,
            },
        ];

        use rand::seq::SliceRandom;
        let mut indices = (0..teams.len()).collect::<Vec<usize>>();
        let mut rng = rand::thread_rng();
        indices.shuffle(&mut rng);

        let pool_a_blue = indices[0];
        let pool_a_orange = indices[1];
        let pool_b_blue = indices[2];
        let pool_b_orange = indices[3];

        let matches = vec![
            TournamentMatch {
                id: 0,
                name: "Pool A".to_string(),
                blue_team_index: pool_a_blue,
                orange_team_index: pool_a_orange,
                winner_team_index: None,
                blue_score: None,
                orange_score: None,
                is_completed: false,
            },
            TournamentMatch {
                id: 1,
                name: "Pool B".to_string(),
                blue_team_index: pool_b_blue,
                orange_team_index: pool_b_orange,
                winner_team_index: None,
                blue_score: None,
                orange_score: None,
                is_completed: false,
            },
            TournamentMatch {
                id: 2,
                name: "Finals".to_string(),
                blue_team_index: 0,   // Placeholder
                orange_team_index: 0, // Placeholder
                winner_team_index: None,
                blue_score: None,
                orange_score: None,
                is_completed: false,
            },
        ];

        let mut state = Self {
            teams,
            matches,
            current_match_index: 0,
            champion_index: None,
            is_active: true,
        };
        state.load_stats();
        state
    }

    /// Reset for a brand new tournament, preserving historic team wins and losses
    pub fn reset_tournament(&mut self) {
        use rand::seq::SliceRandom;
        let mut indices = (0..self.teams.len()).collect::<Vec<usize>>();
        let mut rng = rand::thread_rng();
        indices.shuffle(&mut rng);

        let pool_a_blue = indices[0];
        let pool_a_orange = indices[1];
        let pool_b_blue = indices[2];
        let pool_b_orange = indices[3];

        self.matches = vec![
            TournamentMatch {
                id: 0,
                name: "Pool A".to_string(),
                blue_team_index: pool_a_blue,
                orange_team_index: pool_a_orange,
                winner_team_index: None,
                blue_score: None,
                orange_score: None,
                is_completed: false,
            },
            TournamentMatch {
                id: 1,
                name: "Pool B".to_string(),
                blue_team_index: pool_b_blue,
                orange_team_index: pool_b_orange,
                winner_team_index: None,
                blue_score: None,
                orange_score: None,
                is_completed: false,
            },
            TournamentMatch {
                id: 2,
                name: "Finals".to_string(),
                blue_team_index: 0,   // Placeholder
                orange_team_index: 0, // Placeholder
                winner_team_index: None,
                blue_score: None,
                orange_score: None,
                is_completed: false,
            },
        ];
        self.current_match_index = 0;
        self.champion_index = None;
        self.is_active = true;
    }

    /// Update tournament bracket and accumulate wins/losses/championships
    pub fn complete_current_match(&mut self, blue_score: u32, orange_score: u32) {
        let idx = self.current_match_index;
        if idx >= self.matches.len() {
            return;
        }

        let (winner_idx, loser_idx) = if blue_score > orange_score {
            (self.matches[idx].blue_team_index, self.matches[idx].orange_team_index)
        } else {
            (self.matches[idx].orange_team_index, self.matches[idx].blue_team_index)
        };

        // Update match stats
        self.matches[idx].blue_score = Some(blue_score);
        self.matches[idx].orange_score = Some(orange_score);
        self.matches[idx].winner_team_index = Some(winner_idx);
        self.matches[idx].is_completed = true;

        // Record stats to teams
        self.teams[winner_idx].match_wins += 1;
        self.teams[loser_idx].match_losses += 1;

        println!(
            "TOURNAMENT: Match {} ('{}') complete. Winner: {} ({} - {})",
            idx,
            self.matches[idx].name,
            self.teams[winner_idx].name,
            blue_score,
            orange_score
        );

        if idx == 0 || idx == 1 {
            if idx == 0 {
                self.matches[3].blue_team_index = winner_idx;
            } else {
                self.matches[3].orange_team_index = winner_idx;
            }
        } else if idx == 2 || idx == 3 {
            if idx == 3 {
                self.matches[4].blue_team_index = winner_idx; // Winner of Semi-Finals
            } else {
                self.matches[4].orange_team_index = winner_idx; // Winner of Pool C (bye)
            }
        } else if idx == 4 {
            // Finals completed! Crown champion and add championship count
            self.champion_index = Some(winner_idx);
            self.teams[winner_idx].championships += 1;
            println!("TOURNAMENT: Champion crowned: {}", self.teams[winner_idx].name);
        }

        // Save updated stats to file
        self.save_stats();
    }

    pub fn save_stats(&self) {
        let stats: Vec<serde_json::Value> = self.teams.iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "match_wins": t.match_wins,
                "match_losses": t.match_losses,
                "championships": t.championships,
            })
        }).collect();

        if let Ok(content) = serde_json::to_string_pretty(&stats) {
            if let Err(e) = std::fs::write("tournament_stats.json", content) {
                eprintln!("TOURNAMENT: Failed to save stats to file: {}", e);
            } else {
                println!("TOURNAMENT: Saved persistent stats to tournament_stats.json");
            }
        }
    }

    pub fn load_stats(&mut self) {
        let path = Path::new("tournament_stats.json");
        if !path.exists() {
            return;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(stats_val) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                for stat in stats_val {
                    if let (Some(name), Some(wins), Some(losses), Some(champs)) = (
                        stat.get("name").and_then(|v| v.as_str()),
                        stat.get("match_wins").and_then(|v| v.as_u64()),
                        stat.get("match_losses").and_then(|v| v.as_u64()),
                        stat.get("championships").and_then(|v| v.as_u64()),
                    ) {
                        // Find matching team and update
                        if let Some(team) = self.teams.iter_mut().find(|t| t.name == name) {
                            team.match_wins = wins as u32;
                            team.match_losses = losses as u32;
                            team.championships = champs as u32;
                            println!("TOURNAMENT: Loaded persistent stats for {}: {}W-{}L, {}🏆", name, wins, losses, champs);
                        }
                    }
                }
            }
        }
    }
}
