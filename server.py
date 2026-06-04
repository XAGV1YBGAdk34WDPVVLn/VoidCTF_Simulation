import os
import json
import logging
import time
from concurrent.futures import ThreadPoolExecutor
import tornado.ioloop
import tornado.web
import tornado.websocket
import tornado.concurrent
from simulation.config import TICK_INTERVAL, TICK_RATE
from simulation.engine import GameEngine
from simulation.ai_tactics import get_pregame_tactics, get_match_audit

# Configure logging
logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")
logger = logging.getLogger("server")

# Game state global variables
engine = GameEngine()
clients = set()
executor = ThreadPoolExecutor(max_workers=4)

# State flags for tactic and audit triggers
tactics_fetching = False
tactics_fetched = False
audit_fetching = False
audit_fetched = False

class MainHandler(tornado.web.RequestHandler):
    def get(self):
        self.render("static/index.html")

class GameWebSocketHandler(tornado.websocket.WebSocketHandler):
    def check_origin(self, origin):
        return True

    def open(self):
        clients.add(self)
        logger.info(f"Client connected. Active clients: {len(clients)}")
        # Send initial map layout data
        self.write_message(json.dumps({
            "type": "map_layout",
            "data": engine.map_layout
        }))

    def on_message(self, message):
        try:
            data = json.loads(message)
            msg_type = data.get("type")
            
            if msg_type == "reboot_grid":
                logger.info("Reboot command received from client.")
                global tactics_fetched, tactics_fetching, audit_fetched, audit_fetching
                tactics_fetched = False
                tactics_fetching = False
                audit_fetched = False
                audit_fetching = False
                engine.reset_match()
                broadcast_state()
            elif msg_type == "apply_override_strategy":
                team = data.get("team")
                strat = data.get("strategy")
                if team in ["blue", "orange"] and strat in ["RUSH", "TURTLE", "SPLIT"]:
                    tactics = {
                        "strategy": strat,
                        "rationale": f"User override applied: Executing {strat} vectors.",
                        "source": "Manual Override"
                    }
                    if team == "blue":
                        engine.apply_strategies(tactics, engine.tactics["orange"])
                    else:
                        engine.apply_strategies(engine.tactics["blue"], tactics)
                    broadcast_state()
        except Exception as e:
            logger.error(f"Error handling websocket message: {e}")

    def on_close(self):
        clients.remove(self)
        logger.info(f"Client disconnected. Active clients: {len(clients)}")

def broadcast_state():
    if not clients:
        return
    state_msg = json.dumps({
        "type": "state_update",
        "data": engine.to_dict()
    })
    for client in clients:
        try:
            client.write_message(state_msg)
        except Exception as e:
            logger.error(f"Error writing to client: {e}")

# Asynchronous worker function to generate pregame tactics
def fetch_tactics_task():
    logger.info("Thread worker: Generating pregame tactics...")
    blue_comp = ["Stalker", "Enforcer", "Tactician"]
    orange_comp = ["Stalker", "Enforcer", "Tactician"]
    
    blue_tactics = get_pregame_tactics("blue", blue_comp)
    orange_tactics = get_pregame_tactics("orange", orange_comp)
    
    return blue_tactics, orange_tactics

# Asynchronous worker function to generate match audit
def fetch_audit_task(match_stats):
    logger.info("Thread worker: Generating match systems audit...")
    return get_match_audit(match_stats)

def on_tactics_fetched(future):
    global tactics_fetching, tactics_fetched
    try:
        blue_tactics, orange_tactics = future.result()
        engine.apply_strategies(blue_tactics, orange_tactics)
        tactics_fetched = True
    except Exception as e:
        logger.error(f"Error applying fetched tactics: {e}")
    finally:
        tactics_fetching = False

def on_audit_fetched(future):
    global audit_fetching, audit_fetched
    try:
        audit_report = future.result()
        engine.audit_report = audit_report
        engine.state = "AUDITING"
        audit_fetched = True
    except Exception as e:
        logger.error(f"Error applying audit: {e}")
        engine.state = "AUDITING"
        engine.audit_report = "--- CORE EXCEPTION TRIGGERED DURING AUDIT GENERATION ---"
    finally:
        engine.audit_loading = False
        audit_fetching = False

last_tick_time = time.time()

def game_loop_tick():
    global tactics_fetching, tactics_fetched, audit_fetching, audit_fetched, last_tick_time
    
    # Calculate actual elapsed time since last tick
    time_now = time.time()
    dt = time_now - last_tick_time
    last_tick_time = time_now
    
    # Clamp dt to avoid physics explosions if the loop stalls
    dt = min(dt, 0.1)
    
    # Check states to trigger background tasks
    if engine.state == "PREGAME":
        if not tactics_fetching and not tactics_fetched:
            tactics_fetching = True
            logger.info("Submitting tactics tasks to thread pool executor...")
            future = executor.submit(fetch_tactics_task)
            tornado.ioloop.IOLoop.current().add_future(future, on_tactics_fetched)
            
    elif engine.state == "POSTGAME":
        if not audit_fetching and not audit_fetched:
            audit_fetching = True
            engine.audit_loading = True
            logger.info("Submitting audit task to thread pool executor...")
            future = executor.submit(fetch_audit_task, engine.summary_stats)
            tornado.ioloop.IOLoop.current().add_future(future, on_audit_fetched)
            
    elif engine.state == "RUNNING":
        # Reset audit trackers when match is running
        audit_fetched = False
        audit_fetching = False
        tactics_fetched = False
        tactics_fetching = False

    # Tick the simulation
    engine.update(dt, time_now)
    
    # Send state to clients
    broadcast_state()

def make_app():
    # Setup static path
    static_path = os.path.join(os.path.dirname(__file__), "static")
    
    return tornado.web.Application([
        (r"/", MainHandler),
        (r"/ws", GameWebSocketHandler),
        (r"/static/(.*)", tornado.web.StaticFileHandler, {"path": static_path}),
    ],
    template_path=os.path.dirname(__file__),
    debug=True)

if __name__ == "__main__":
    app = make_app()
    port = 8080
    app.listen(port)
    logger.info(f"Grid server running at http://localhost:{port}/")
    
    # Setup game loop periodic callback
    game_loop = tornado.ioloop.PeriodicCallback(game_loop_tick, TICK_INTERVAL * 1000)
    game_loop.start()
    
    # Run server
    tornado.ioloop.IOLoop.current().start()
