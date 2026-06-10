// src/main.rs
// Entrypoint for Axum WebSocket and static asset hosting.

mod config;
mod math;
mod world;
mod player;
mod engine;
mod ai_tactics;
mod tournament;

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tower_http::services::ServeDir;
use futures_util::{sink::SinkExt, stream::StreamExt};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

type ClientSender = mpsc::UnboundedSender<Message>;

struct AppState {
    engine: Arc<Mutex<crate::engine::GameEngine>>,
    clients: Arc<Mutex<Vec<ClientSender>>>,
    // Track async task status
    tactics_fetching: Arc<Mutex<bool>>,
    tactics_fetched: Arc<Mutex<bool>>,
    audit_fetching: Arc<Mutex<bool>>,
    audit_fetched: Arc<Mutex<bool>>,
}

#[tokio::main]
async fn main() {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let engine = Arc::new(Mutex::new(crate::engine::GameEngine::new()));
    let clients = Arc::new(Mutex::new(Vec::new()));

    let state = Arc::new(AppState {
        engine: Arc::clone(&engine),
        clients: Arc::clone(&clients),
        tactics_fetching: Arc::new(Mutex::new(false)),
        tactics_fetched: Arc::new(Mutex::new(false)),
        audit_fetching: Arc::new(Mutex::new(false)),
        audit_fetched: Arc::new(Mutex::new(false)),
    });

    // Start background simulation loop (30Hz)
    let state_loop_clone = Arc::clone(&state);
    tokio::spawn(async move {
        run_sim_loop(state_loop_clone).await;
    });

    // Setup routes
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(index_handler))
        .with_state(state);

    let port = 8080;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    info!("Grid server running at http://localhost:{}/", port);
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> impl IntoResponse {
    axum::response::Html(include_str!("../static/index.html"))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for this client
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Send initial map layout immediately
    let map_layout = {
        let lock = state.engine.lock().await;
        lock.map_layout.clone()
    };
    let layout_msg = serde_json::json!({
        "type": "map_layout",
        "data": map_layout
    });
    if tx.send(Message::Text(layout_msg.to_string().into())).is_err() {
        return;
    }

    // Add to global client list
    {
        let mut lock = state.clients.lock().await;
        lock.push(tx.clone());
        info!("Client connected. Active clients: {}", lock.len());
    }

    // Spawn a writer task to forward channel messages to websocket
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Reader task
    let state_clone = Arc::clone(&state);
    let reader_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if msg_type == "reboot_grid" {
                    info!("Reboot command received from client.");
                    {
                        let mut lock = state_clone.tactics_fetched.lock().await;
                        *lock = false;
                        let mut lock = state_clone.tactics_fetching.lock().await;
                        *lock = false;
                        let mut lock = state_clone.audit_fetched.lock().await;
                        *lock = false;
                        let mut lock = state_clone.audit_fetching.lock().await;
                        *lock = false;
                    }
                    let map_layout = {
                        let mut engine_lock = state_clone.engine.lock().await;
                        engine_lock.reset_match();
                        engine_lock.map_layout.clone()
                    };

                    let map_msg = Message::Text(serde_json::json!({
                        "type": "map_layout",
                        "data": map_layout
                    }).to_string().into());

                    {
                        let mut clients_lock = state_clone.clients.lock().await;
                        clients_lock.retain(|tx| {
                            tx.send(map_msg.clone()).is_ok()
                        });
                    }

                    broadcast_state(&state_clone).await;
                } else if msg_type == "toggle_pause" {
                    info!("Pause toggle command received from client.");
                    {
                        let mut engine_lock = state_clone.engine.lock().await;
                        engine_lock.is_paused = !engine_lock.is_paused;
                        let pause_status = engine_lock.is_paused;
                        engine_lock.log_event(&format!(
                            "Simulation {}", 
                            if pause_status { "PAUSED" } else { "RESUMED" }
                        ));
                    }
                    broadcast_state(&state_clone).await;
                } else if msg_type == "apply_override_strategy" {
                    let team = data.get("team").and_then(|v| v.as_str()).unwrap_or("");
                    let strat = data.get("strategy").and_then(|v| v.as_str()).unwrap_or("");
                    if (team == "blue" || team == "orange") && ["RUSH", "TURTLE", "SPLIT"].contains(&strat) {
                        let mut engine_lock = state_clone.engine.lock().await;
                        let tactics_payload = serde_json::json!({
                            "strategy": strat,
                            "rationale": format!("User override applied: Executing {} vectors.", strat),
                            "source": "Manual Override"
                        });
                        
                        let blue_strat = if team == "blue" { tactics_payload.clone() } else { engine_lock.tactics["blue"].clone() };
                        let orange_strat = if team == "orange" { tactics_payload } else { engine_lock.tactics["orange"].clone() };
                        
                        engine_lock.apply_strategies(blue_strat, orange_strat);
                    }
                    broadcast_state(&state_clone).await;
                }
            }
        }
    });

    // Wait until reader or writer task finishes
    tokio::select! {
        _ = writer_task => {},
        _ = reader_task => {},
    };

    // Remove client from global list on disconnect
    {
        let mut lock = state.clients.lock().await;
        // Filter out disconnected channels by trying to send a ping or just keeping track
        // Simple way: check which sender is closed or just match address.
        // We can just retain senders that are not closed.
        lock.retain(|tx_chan| !tx_chan.is_closed());
        info!("Client disconnected. Active clients: {}", lock.len());
    }
}

async fn broadcast_state(state: &AppState) {
    let payload = {
        let lock = state.engine.lock().await;
        lock.to_json()
    };
    
    let msg = Message::Text(serde_json::json!({
        "type": "state_update",
        "data": payload
    }).to_string().into());

    let mut clients_lock = state.clients.lock().await;
    clients_lock.retain(|tx| {
        tx.send(msg.clone()).is_ok()
    });
}

async fn run_sim_loop(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(33)); // ~30Hz
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let start_time_instant = tokio::time::Instant::now();
    let mut last_tick_time = start_time_instant;

    loop {
        interval.tick().await;

        let time_now_instant = tokio::time::Instant::now();
        let dt = (time_now_instant - last_tick_time).as_secs_f32();
        last_tick_time = time_now_instant;

        let time_now_secs = start_time_instant.elapsed().as_secs_f32(); // system relative time

        // Check pre-game tactics trigger
        let (trigger_tactics, trigger_audit) = {
            let engine_lock = state.engine.lock().await;
            let current_state = &engine_lock.state;
            
            let mut tactics_fetch = state.tactics_fetching.lock().await;
            let tactics_done = state.tactics_fetched.lock().await;
            let mut audit_fetch = state.audit_fetching.lock().await;
            let audit_done = state.audit_fetched.lock().await;

            let trigger_tact = current_state == "PREGAME" && !*tactics_fetch && !*tactics_done;
            let trigger_aud = current_state == "POSTGAME" && !*audit_fetch && !*audit_done;

            if trigger_tact {
                *tactics_fetch = true;
            }
            if trigger_aud {
                *audit_fetch = true;
            }

            (trigger_tact, trigger_aud)
        };

        if trigger_tactics {
            let state_clone = Arc::clone(&state);
            tokio::spawn(async move {
                info!("Thread worker: Generating pregame tactics...");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                let blue_tactics = crate::ai_tactics::get_pregame_tactics("blue");
                let orange_tactics = crate::ai_tactics::get_pregame_tactics("orange");
                
                {
                    let mut lock = state_clone.engine.lock().await;
                    lock.apply_strategies(blue_tactics, orange_tactics);
                }
                
                *state_clone.tactics_fetching.lock().await = false;
                *state_clone.tactics_fetched.lock().await = true;
            });
        }

        if trigger_audit {
            let state_clone = Arc::clone(&state);
            tokio::spawn(async move {
                info!("Thread worker: Generating match systems audit...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let stats = {
                    let lock = state_clone.engine.lock().await;
                    lock.summary_stats.clone()
                };

                let audit_text = crate::ai_tactics::get_match_audit(&stats);

                {
                    let mut lock = state_clone.engine.lock().await;
                    lock.audit_report = Some(audit_text);
                    lock.audit_loading = false;
                    lock.state = "AUDITING".to_string();
                    lock.timer = 15.0; // 15 seconds countdown for auto-reboot
                }

                *state_clone.audit_fetching.lock().await = false;
                *state_clone.audit_fetched.lock().await = true;
            });
        }

        // Automatic match restart in AUDITING state
        let mut auto_reboot = false;
        {
            let mut engine_lock = state.engine.lock().await;
            if !engine_lock.is_paused {
                if engine_lock.state == "AUDITING" {
                    if engine_lock.timer > 0.0 {
                        engine_lock.timer -= dt;
                        if engine_lock.timer <= 0.0 {
                            let blue_score = engine_lock.scores["blue"];
                            let orange_score = engine_lock.scores["orange"];
                            
                            engine_lock.tournament.complete_current_match(blue_score, orange_score);
                            
                            if engine_lock.tournament.champion_index.is_some() {
                                engine_lock.state = "CHAMPION_CELEBRATION".to_string();
                                engine_lock.timer = 15.0; // 15 seconds celebration pose
                            } else {
                                engine_lock.tournament.current_match_index += 1;
                                engine_lock.reset_match();
                                auto_reboot = true;
                            }
                        }
                    }
                } else if engine_lock.state == "CHAMPION_CELEBRATION" {
                    if engine_lock.timer > 0.0 {
                        engine_lock.timer -= dt;
                        if engine_lock.timer <= 0.0 {
                            engine_lock.tournament.reset_tournament();
                            engine_lock.reset_match();
                            auto_reboot = true;
                        }
                    }
                }
            }
        }

        if auto_reboot {
            let map_layout = {
                let lock = state.engine.lock().await;
                lock.map_layout.clone()
            };
            let map_msg = axum::extract::ws::Message::Text(serde_json::json!({
                "type": "map_layout",
                "data": map_layout
            }).to_string().into());
            let mut clients_lock = state.clients.lock().await;
            clients_lock.retain(|tx| {
                tx.send(map_msg.clone()).is_ok()
            });

            {
                let mut lock = state.tactics_fetched.lock().await;
                *lock = false;
                let mut lock = state.tactics_fetching.lock().await;
                *lock = false;
                let mut lock = state.audit_fetched.lock().await;
                *lock = false;
                let mut lock = state.audit_fetching.lock().await;
                *lock = false;
            }
        }

        // Tick simulation
        {
            let mut lock = state.engine.lock().await;
            lock.update(dt.min(0.1), time_now_secs);
        }

        // Broadcast state
        broadcast_state(&state).await;
    }
}
