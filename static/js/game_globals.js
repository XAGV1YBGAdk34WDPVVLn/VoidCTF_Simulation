// game_globals.js: Constants, global state variables, caches, and utilities.

// Constants
const TEAM_COLORS = {
    "blue": 0x00f0ff,
    "orange": 0xff7b00,
    "blue_dark": 0x003355,
    "orange_dark": 0x552200,
    "neutral": 0xbd00ff // Purple
};

const CLASS_SHAPES = {
    "Stalker": "octahedron",
    "Enforcer": "cylinder",
    "Tactician": "diamond"
};

// State Variables
let scene, camera, renderer, composer, bloomPass;
let orbitControls;
let ws = null;
let currentGameState = null;
let mapLayout = null;
let lastTypedAuditReport = "";
let lastActiveBlueTeam = null;
let lastActiveOrangeTeam = null;

// Playhead buffer for entity interpolation (smooth, jitter-free movement)
const stateBuffer = [];
const maxBufferSize = 60; // 2 seconds of history at 30Hz
let clientRenderTime = null;
let clientTimeScale = 1.0;
const interpolationDelay = 0.15; // 150ms interpolation delay (approx 4.5 ticks behind)

// Settings & Camera State
let useBloom = true;
let cameraMode = "ghost"; // "ghost" or "action"
let trackingTargetId = "auto"; // "auto" or player id (0-5)
let keyStates = {}; // Tracking WASD key presses for Ghost flight
let logOffset = 0; // Cache printed logs count to prevent browser freezing

// Three.js object caches
const meshCache = {
    players: {},       // player_id -> { group, bodyMesh, shieldMesh, trailLine, lastPos }
    projectiles: {},   // projectile_id -> { mesh, targetPos }
    flags: {},         // "blue", "orange" -> { group, pedestal, coreMesh }
    healingBeams: {},  // healer_id -> Line
    mapElements: [],   // list of platforms/buildings meshes
    overchargeNode: null,
    matBlue: null,
    matOrange: null,
    matNeutral: null
};

// Particle System for De-rezzing
const particleGroups = [];

// Grid Helpers
let gridBlue = null;
let gridOrange = null;

// Coordinate Mapper Helper: Python Simulation coordinates to Three.js coordinates
// Python: X is horizontal (-100 to 100), Y is horizontal depth (-100 to 100), Z is height (0 to 50)
// Three.js: X is horizontal, Y is height (up), Z is horizontal depth
function pyToThreeVec(pyPos) {
    if (!pyPos) return new THREE.Vector3(0, 0, 0);
    return new THREE.Vector3(pyPos[0], pyPos[2], pyPos[1]);
}

function jsonParseSafe(str) {
    try {
        return JSON.parse(str);
    } catch (e) {
        return null;
    }
}

function updateTeamColorsFromTournament(tournament) {
    if (!tournament) return;
    const activeMatch = tournament.matches[tournament.current_match_index];
    if (!activeMatch) return;
    const blueTeam = tournament.teams[activeMatch.blue_team_index];
    const orangeTeam = tournament.teams[activeMatch.orange_team_index];
    
    if (!blueTeam || !orangeTeam) return;
    
    if (lastActiveBlueTeam === blueTeam.name && lastActiveOrangeTeam === orangeTeam.name) {
        return; // Skip redundant updates
    }
    
    lastActiveBlueTeam = blueTeam.name;
    lastActiveOrangeTeam = orangeTeam.name;
    
    // Convert hex to hex number
    const colorBlue = new THREE.Color(blueTeam.primary_hex);
    const colorOrange = new THREE.Color(orangeTeam.primary_hex);
    
    TEAM_COLORS.blue = colorBlue.getHex();
    TEAM_COLORS.orange = colorOrange.getHex();
    
    TEAM_COLORS.blue_dark = colorBlue.clone().multiplyScalar(0.2).getHex();
    TEAM_COLORS.orange_dark = colorOrange.clone().multiplyScalar(0.2).getHex();
    
    // Update materials if they exist
    if (meshCache.matBlue) meshCache.matBlue.color.setHex(TEAM_COLORS.blue);
    if (meshCache.matOrange) meshCache.matOrange.color.setHex(TEAM_COLORS.orange);
    
    // Update grid helpers
    if (gridBlue) gridBlue.material.color.setHex(TEAM_COLORS.blue);
    if (gridOrange) gridOrange.material.color.setHex(TEAM_COLORS.orange);
    
    // Update lines and outlines in scene
    meshCache.mapElements.forEach(m => {
        if (m.isLineSegments || m.isLine) {
            if (m.team === "blue") {
                m.material.color.setHex(TEAM_COLORS.blue);
            } else if (m.team === "orange") {
                m.material.color.setHex(TEAM_COLORS.orange);
            }
        }
    });

    // Update flag pedestals and flag cores
    const blueFlag = meshCache.flags["blue"];
    if (blueFlag) {
        if (blueFlag.pedestal) {
            blueFlag.pedestal.material.color.setHex(TEAM_COLORS.blue);
            blueFlag.pedestal.children.forEach(child => {
                if (child.isLineSegments || child.isLine) {
                    child.material.color.setHex(TEAM_COLORS.blue).multiplyScalar(10.0);
                }
            });
        }
        if (blueFlag.coreMesh) {
            blueFlag.coreMesh.material.color.setHex(TEAM_COLORS.blue);
            blueFlag.coreMesh.material.emissive.setHex(TEAM_COLORS.blue).multiplyScalar(3.0);
        }
    }
    const orangeFlag = meshCache.flags["orange"];
    if (orangeFlag) {
        if (orangeFlag.pedestal) {
            orangeFlag.pedestal.material.color.setHex(TEAM_COLORS.orange);
            orangeFlag.pedestal.children.forEach(child => {
                if (child.isLineSegments || child.isLine) {
                    child.material.color.setHex(TEAM_COLORS.orange).multiplyScalar(10.0);
                }
            });
        }
        if (orangeFlag.coreMesh) {
            orangeFlag.coreMesh.material.color.setHex(TEAM_COLORS.orange);
            orangeFlag.coreMesh.material.emissive.setHex(TEAM_COLORS.orange).multiplyScalar(3.0);
        }
    }
    
    // Update DOM team names & styles
    document.documentElement.style.setProperty('--blue-team-color', blueTeam.primary_hex);
    document.documentElement.style.setProperty('--blue-team-bg', `${blueTeam.primary_hex}06`);
    document.documentElement.style.setProperty('--blue-team-border', `${blueTeam.primary_hex}2b`);

    document.documentElement.style.setProperty('--orange-team-color', orangeTeam.primary_hex);
    document.documentElement.style.setProperty('--orange-team-bg', `${orangeTeam.primary_hex}06`);
    document.documentElement.style.setProperty('--orange-team-border', `${orangeTeam.primary_hex}2b`);
    const blueRosterHeader = document.querySelector("#blue-roster .roster-team-title");
    if (blueRosterHeader) {
        blueRosterHeader.innerText = blueTeam.name.toUpperCase();
        blueRosterHeader.style.color = blueTeam.primary_hex;
    }
    const orangeRosterHeader = document.querySelector("#orange-roster .roster-team-title");
    if (orangeRosterHeader) {
        orangeRosterHeader.innerText = orangeTeam.name.toUpperCase();
        orangeRosterHeader.style.color = orangeTeam.primary_hex;
    }
    
    const blueScoreLabel = document.querySelector(".blue-score-box .team-label");
    if (blueScoreLabel) blueScoreLabel.innerText = blueTeam.name.toUpperCase();
    
    const orangeScoreLabel = document.querySelector(".orange-score-box .team-label");
    if (orangeScoreLabel) orangeScoreLabel.innerText = orangeTeam.name.toUpperCase();
    
    const blueScoreBox = document.querySelector(".blue-score-box");
    if (blueScoreBox) {
        blueScoreBox.style.borderColor = blueTeam.primary_hex;
        blueScoreBox.style.boxShadow = `0 0 10px ${blueTeam.primary_hex}`;
    }
    const orangeScoreBox = document.querySelector(".orange-score-box");
    if (orangeScoreBox) {
        orangeScoreBox.style.borderColor = orangeTeam.primary_hex;
        orangeScoreBox.style.boxShadow = `0 0 10px ${orangeTeam.primary_hex}`;
    }

    // Score text numbers color
    const blueScoreNum = document.getElementById("score-blue");
    if (blueScoreNum) blueScoreNum.style.color = blueTeam.primary_hex;
    const orangeScoreNum = document.getElementById("score-orange");
    if (orangeScoreNum) orangeScoreNum.style.color = orangeTeam.primary_hex;
    
    // Tactics titles and boxes colors
    const blueStratBox = document.querySelector(".blue-strat");
    if (blueStratBox) {
        blueStratBox.style.background = `rgba(${colorBlue.r * 255}, ${colorBlue.g * 255}, ${colorBlue.b * 255}, 0.05)`;
        blueStratBox.style.borderColor = blueTeam.primary_hex + "33"; // semi-transparent border
        blueStratBox.style.borderLeftColor = blueTeam.primary_hex;
    }
    const orangeStratBox = document.querySelector(".orange-strat");
    if (orangeStratBox) {
        orangeStratBox.style.background = `rgba(${colorOrange.r * 255}, ${colorOrange.g * 255}, ${colorOrange.b * 255}, 0.05)`;
        orangeStratBox.style.borderColor = orangeTeam.primary_hex + "33";
        orangeStratBox.style.borderLeftColor = orangeTeam.primary_hex;
    }

    const blueStratHeader = document.querySelector(".blue-strat h3");
    if (blueStratHeader) {
        blueStratHeader.innerHTML = `<span style="color: ${blueTeam.primary_hex}">${blueTeam.name.toUpperCase()}</span> STRATEGY: <span id="blue-strategy-title" class="strategy-badge" style="color: ${blueTeam.primary_hex}">LOADING...</span>`;
    }
    const orangeStratHeader = document.querySelector(".orange-strat h3");
    if (orangeStratHeader) {
        orangeStratHeader.innerHTML = `<span style="color: ${orangeTeam.primary_hex}">${orangeTeam.name.toUpperCase()}</span> STRATEGY: <span id="orange-strategy-title" class="strategy-badge" style="color: ${orangeTeam.primary_hex}">LOADING...</span>`;
    }
    
    // Pregame overlay loading labels
    const bluePregameText = document.querySelector(".tactics-loading-status .status-row:nth-child(1) span:nth-child(1)");
    if (bluePregameText) {
        bluePregameText.innerText = `Initializing Tactical Core (${blueTeam.name} Strategy)...`;
    }
    const orangePregameText = document.querySelector(".tactics-loading-status .status-row:nth-child(2) span:nth-child(1)");
    if (orangePregameText) {
        orangePregameText.innerText = `Initializing Tactical Core (${orangeTeam.name} Strategy)...`;
    }
}
