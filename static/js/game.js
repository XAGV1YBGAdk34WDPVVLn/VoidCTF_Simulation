// Game.js: Void Grid 3v3 CTF Three.js Frontend

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
    overchargeNode: null
};

// Particle System for De-rezzing
const particleGroups = [];

// Initialize Page
document.addEventListener("DOMContentLoaded", () => {
    initThree();
    setupEventListeners();
    connectWebSocket();
    animate();
});

// 1. THREE.JS INITIALIZATION
function initThree() {
    const container = document.getElementById("canvas-container");
    const width = container.clientWidth;
    const height = container.clientHeight;

    // Create Scene
    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x020208);
    // Add glowing fog
    scene.fog = new THREE.FogExp2(0x020208, 0.0035);

    // Create Camera
    camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 1000);
    camera.position.set(0, 80, 140);

    // Create Renderer
    renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: "high-performance" });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2)); // Limit pixel ratio to 2 for older laptops
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.0;
    container.appendChild(renderer.domElement);

    // Setup Orbit Controls (for Ghost Cam)
    orbitControls = new THREE.OrbitControls(camera, renderer.domElement);
    orbitControls.enableDamping = true;
    orbitControls.dampingFactor = 0.05;
    orbitControls.maxPolarAngle = Math.PI / 2 - 0.01; // Don't go below ground
    orbitControls.minDistance = 10;
    orbitControls.maxDistance = 300;

    // Setup Lights (Ambient + Neon Accent Lights)
    const ambientLight = new THREE.AmbientLight(0x0a0a20, 1.5);
    scene.add(ambientLight);

    const dirLight1 = new THREE.DirectionalLight(0x333366, 1.0);
    dirLight1.position.set(100, 150, 50);
    scene.add(dirLight1);

    // Postprocessing: Unreal Bloom Pass
    const renderScene = new THREE.RenderPass(scene, camera);
    bloomPass = new THREE.UnrealBloomPass(
        new THREE.Vector2(width, height),
        1.3,  // Strength
        0.4,  // Radius
        0.55  // Threshold: only bright emissive meshes will glow
    );
    
    composer = new THREE.EffectComposer(renderer);
    composer.addPass(renderScene);
    composer.addPass(bloomPass);

    // Window Resize Event
    window.addEventListener("resize", onWindowResize);
    
    // Draw default elements before map loads
    drawBaseGrids();
}

let gridBlue = null;
let gridOrange = null;

function drawBaseGrids() {
    // Left Grid (Blue Team Zone)
    gridBlue = new THREE.GridHelper(200, 40, 0x00f0ff, 0x001122);
    gridBlue.position.set(-50, 0, 0);
    gridBlue.material.opacity = 0.25;
    gridBlue.material.transparent = true;
    scene.add(gridBlue);

    // Right Grid (Orange Team Zone)
    gridOrange = new THREE.GridHelper(200, 40, 0xff7b00, 0x221100);
    gridOrange.position.set(50, 0, 0);
    gridOrange.material.opacity = 0.25;
    gridOrange.material.transparent = true;
    scene.add(gridOrange);

    // Mid Divider Line (Glowing Purple)
    const midLineGeom = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(0, 0.1, -100),
        new THREE.Vector3(0, 0.1, 100)
    ]);
    const midLineMat = new THREE.LineBasicMaterial({ color: 0xbd00ff, linewidth: 2 });
    const midLine = new THREE.Line(midLineGeom, midLineMat);
    scene.add(midLine);
}

// Coordinate Mapper Helper: Python Simulation coordinates to Three.js coordinates
// Python: X is horizontal (-100 to 100), Y is horizontal depth (-100 to 100), Z is height (0 to 50)
// Three.js: X is horizontal, Y is height (up), Z is horizontal depth
function pyToThreeVec(pyPos) {
    if (!pyPos) return new THREE.Vector3(0, 0, 0);
    return new THREE.Vector3(pyPos[0], pyPos[2], pyPos[1]);
}


// 2. ENVIRONMENT CREATION (from WebSocket Map Layout packet)
function buildMapEnvironment(layout) {
    // Clear old map meshes if they exist
    if (meshCache.mapElements.length > 0) {
        meshCache.mapElements.forEach(m => scene.remove(m));
        meshCache.mapElements = [];
    }
    
    // Clear old flag pedestals
    for (const team in meshCache.flags) {
        const flagObj = meshCache.flags[team];
        if (flagObj) {
            scene.remove(flagObj.group);
            scene.remove(flagObj.pedestal);
        }
    }
    meshCache.flags = {};

    // Clear old overcharge node
    if (meshCache.overchargeNode) {
        scene.remove(meshCache.overchargeNode);
        meshCache.overchargeNode = null;
    }

    mapLayout = layout;

    const wallMaterial = new THREE.MeshStandardMaterial({
        color: 0x03030f,
        roughness: 0.1,
        metalness: 0.9,
        transparent: true,
        opacity: 0.6
    });

    // 1. Build elevated platforms
    layout.platforms.forEach(platform => {
        const { x, y, w, d, z } = platform;
        // Three.js geometry: w (X), h (Y - vertical height of platform thickness), d (Z)
        const heightThickness = 0.8;
        const geom = new THREE.BoxGeometry(w, heightThickness, d);
        const mesh = new THREE.Mesh(geom, wallMaterial);
        
        // Python: X is top-left, Y is top-left. Three.js needs center position.
        // Also map Python Z to Three.js Y height.
        mesh.position.set(x + w/2, z - heightThickness/2, y + d/2);
        scene.add(mesh);
        meshCache.mapElements.push(mesh);

        // Add glowing neon edges for the platform
        const edges = new THREE.EdgesGeometry(geom);
        const color = x < 0 ? TEAM_COLORS.blue : TEAM_COLORS.orange;
        const lineMat = new THREE.LineBasicMaterial({ color: color, linewidth: 2 });
        const line = new THREE.LineSegments(edges, lineMat);
        line.position.copy(mesh.position);
        scene.add(line);
        meshCache.mapElements.push(line);
    });

    // 2. Build non-walkable solid columns / walls (Buildings)
    layout.buildings.forEach(building => {
        const { x, y, w, d, z, h } = building;
        const geom = new THREE.BoxGeometry(w, h, d);
        const mesh = new THREE.Mesh(geom, wallMaterial);
        mesh.position.set(x + w/2, z + h/2, y + d/2);
        scene.add(mesh);
        meshCache.mapElements.push(mesh);

        // Neon outline
        const edges = new THREE.EdgesGeometry(geom);
        // Distribute colors based on coordinates
        let color = TEAM_COLORS.neutral;
        if (x < -30) color = TEAM_COLORS.blue;
        else if (x > 30) color = TEAM_COLORS.orange;
        
        const lineMat = new THREE.LineBasicMaterial({ color: color, linewidth: 1.5 });
        const line = new THREE.LineSegments(edges, lineMat);
        line.position.copy(mesh.position);
        scene.add(line);
        meshCache.mapElements.push(line);
    });

    // 2.5 Build walkable ramps (inclined slabs connecting height layers)
    if (layout.ramps) {
        layout.ramps.forEach(ramp => {
            const { x1, x2, y1, y2, z1, z2 } = ramp;
            
            const w = x2 - x1; // Horizontal span (X)
            const d = y2 - y1; // Depth (Z)
            const h = 0.5;     // Thickness of the ramp surface slab
            
            // Actual length of the ramp along the slope/incline
            const slopeLength = Math.sqrt(w * w + (z2 - z1) * (z2 - z1));
            
            const geom = new THREE.BoxGeometry(slopeLength, h, d);
            const mesh = new THREE.Mesh(geom, wallMaterial);
            
            // Center coordinates: Python Z maps to Three.js vertical Y
            const cx = (x1 + x2) / 2.0;
            const cy = (z1 + z2) / 2.0;
            const cz = (y1 + y2) / 2.0;
            mesh.position.set(cx, cy, cz);
            
            // Calculate slope angle around local Z-axis (rotation lifts one horizontal side)
            const angle = Math.atan2(z2 - z1, x2 - x1);
            mesh.rotation.z = angle;
            
            scene.add(mesh);
            meshCache.mapElements.push(mesh);
            
            // Inclined glowing neon borders
            const edges = new THREE.EdgesGeometry(geom);
            const color = cx < 0 ? TEAM_COLORS.blue : TEAM_COLORS.orange;
            const lineMat = new THREE.LineBasicMaterial({ color: color, linewidth: 2 });
            const line = new THREE.LineSegments(edges, lineMat);
            line.position.copy(mesh.position);
            line.rotation.z = angle;
            scene.add(line);
            meshCache.mapElements.push(line);
        });
    }

    // 3. Build Flag bases (Pedestals)
    createFlagPedestal("blue", pyToThreeVec(layout.bases.blue.pos));
    createFlagPedestal("orange", pyToThreeVec(layout.bases.orange.pos));
}

function createFlagPedestal(team, pos) {
    const color = TEAM_COLORS[team];
    
    // Glowing base pedestal ring
    const geom = new THREE.CylinderGeometry(6, 6, 1.5, 32);
    const mat = new THREE.MeshStandardMaterial({
        color: 0x050510,
        emissive: color,
        emissiveIntensity: 0.15,
        roughness: 0.1,
        metalness: 0.9
    });
    const mesh = new THREE.Mesh(geom, mat);
    mesh.position.copy(pos);
    mesh.position.y += 0.75;
    scene.add(mesh);
    
    // Neon edge ring
    const edges = new THREE.EdgesGeometry(geom);
    const lineMat = new THREE.LineBasicMaterial({ color: color, linewidth: 2 });
    const line = new THREE.LineSegments(edges, lineMat);
    line.position.copy(mesh.position);
    scene.add(line);
    
    // Create the Flag itself (stored in cache)
    const flagGroup = new THREE.Group();
    flagGroup.position.copy(pos);
    flagGroup.position.y += 2.0; // Float above pedestal
    
    // Glowing staff
    const staffGeom = new THREE.CylinderGeometry(0.2, 0.2, 5, 8);
    const staffMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
    const staff = new THREE.Mesh(staffGeom, staffMat);
    flagGroup.add(staff);
    
    // Glowing floating tip (octahedron)
    const tipGeom = new THREE.OctahedronGeometry(1.2, 0);
    const tipMat = new THREE.MeshStandardMaterial({
        color: color,
        emissive: color,
        emissiveIntensity: 1.5
    });
    const tip = new THREE.Mesh(tipGeom, tipMat);
    tip.position.y = 3.0;
    flagGroup.add(tip);
    
    scene.add(flagGroup);
    meshCache.flags[team] = {
        group: flagGroup,
        pedestal: mesh,
        coreMesh: tip
    };
}

// 3. WEBSOCKET CONNECTION AND REAL-TIME UPDATES
function connectWebSocket() {
    const wsProto = window.location.protocol === "https:" ? "wss://" : "ws://";
    const wsUrl = wsProto + window.location.host + "/ws";
    
    ws = new WebSocket(wsUrl);
    
    ws.onopen = () => {
        console.log("WebSocket connected to Grid Core.");
        document.getElementById("status-label").innerText = "GRID SYNCHRONIZED";
    };

    ws.onmessage = (event) => {
        const payload = jsonParseSafe(event.data);
        if (!payload) return;

        if (payload.type === "map_layout") {
            buildMapEnvironment(payload.data);
        } else if (payload.type === "state_update") {
            const newSimTime = payload.data.sim_time;
            
            // Check for reboot / game reset
            if (stateBuffer.length > 0) {
                const prevSimTime = stateBuffer[stateBuffer.length - 1].sim_time;
                const prevState = stateBuffer[stateBuffer.length - 1].state;
                const isReboot = (payload.data.state === "PREGAME" && prevState !== "PREGAME") ||
                                 (payload.data.state === "RUNNING" && newSimTime < prevSimTime);
                if (isReboot) {
                    console.log("Grid reboot/reset detected. Clearing interpolation buffer.");
                    stateBuffer.length = 0;
                    clientRenderTime = null;
                    mapLayout = null;
                    lastTypedAuditReport = "";
                }
            }
            
            stateBuffer.push(payload.data);
            if (stateBuffer.length > maxBufferSize) {
                stateBuffer.shift();
            }
            
            // Update HUD elements and DOM overlays immediately on receipt (30Hz)
            updateHUD(payload.data);
            updateDOMState(payload.data);
        }
    };

    ws.onclose = () => {
        console.warn("WebSocket closed. Attempting reconnect in 3s...");
        document.getElementById("status-label").innerText = "CONNECTION LATENCY / RECONNECTING...";
        setTimeout(connectWebSocket, 3000);
    };

    ws.onerror = (err) => {
        console.error("WebSocket error: ", err);
    };
}

function updateDOMState(gameState) {
    const pregameOverlay = document.getElementById("pregame-overlay");
    const auditOverlay = document.getElementById("audit-overlay");
    
    // Always update tactics overlay sidebar/strategies so they don't get stuck on LOADING... if we refresh mid-game
    if (gameState.tactics) {
        updateTacticsOverlay(gameState.tactics);
    }
    
    if (gameState.state === "PREGAME") {
        if (pregameOverlay.classList.contains("hidden")) {
            pregameOverlay.classList.remove("hidden");
        }
        if (!auditOverlay.classList.contains("hidden")) {
            auditOverlay.classList.add("hidden");
        }
        
        const countdownEl = document.getElementById("countdown-number");
        const countdownVal = Math.ceil(gameState.timer);
        if (countdownEl.innerText !== String(countdownVal)) {
            countdownEl.innerText = countdownVal;
        }
        
        // Update Tactics fetching states
        updateTacticsOverlay(gameState.tactics);
        
        // Set circular spinner stroke offset
        const spinner = document.querySelector(".spinner-fill");
        if (spinner) {
            const ratio = gameState.timer / 15.0;
            const offset = 283 * (1 - ratio);
            spinner.style.strokeDashoffset = offset;
        }
    } else if (gameState.state === "RUNNING") {
        if (!pregameOverlay.classList.contains("hidden")) {
            pregameOverlay.classList.add("hidden");
        }
        if (!auditOverlay.classList.contains("hidden")) {
            auditOverlay.classList.add("hidden");
        }
        
        // Handle Game Time display
        const totalSecs = Math.max(0, gameState.match_time);
        const mins = Math.floor(totalSecs / 60);
        const secs = Math.floor(totalSecs % 60);
        const timerText = `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        
        const timerEl = document.getElementById("timer-display");
        if (timerEl.innerText !== timerText) {
            timerEl.innerText = timerText;
        }
        
        const statusEl = document.getElementById("status-label");
        if (statusEl.innerText !== "GRID SIMULATION ACTIVE") {
            statusEl.innerText = "GRID SIMULATION ACTIVE";
        }
    } else if (gameState.state === "POSTGAME" || gameState.state === "AUDITING") {
        if (!pregameOverlay.classList.contains("hidden")) {
            pregameOverlay.classList.add("hidden");
        }
        if (auditOverlay.classList.contains("hidden")) {
            auditOverlay.classList.remove("hidden");
        }
        
        const textContainer = document.getElementById("audit-text");
        const loadingSpinner = document.getElementById("audit-loading-spinner");
        
        if (gameState.audit_loading) {
            if (loadingSpinner.classList.contains("hidden")) {
                loadingSpinner.classList.remove("hidden");
            }
            if (!textContainer.classList.contains("hidden")) {
                textContainer.classList.add("hidden");
            }
            textContainer.innerText = "";
        } else if (gameState.audit_report) {
            if (!loadingSpinner.classList.contains("hidden")) {
                loadingSpinner.classList.add("hidden");
            }
            if (textContainer.classList.contains("hidden")) {
                textContainer.classList.remove("hidden");
            }
            if (lastTypedAuditReport !== gameState.audit_report) {
                lastTypedAuditReport = gameState.audit_report;
                typewriteText(textContainer, gameState.audit_report);
            }
            
            const secondsLeft = Math.ceil(gameState.timer);
            const auditRebootBtn = document.getElementById("btn-audit-reboot");
            if (secondsLeft > 0) {
                auditRebootBtn.innerText = `REBOOT GRID CYCLE (${secondsLeft}s)`;
            } else {
                auditRebootBtn.innerText = "REBOOT GRID CYCLE";
            }
        }
    }
}

function processStateUpdate(gameState) {
    // Update Players (Positions, Health, State)
    const serverTime = (gameState.sim_time !== undefined ? gameState.sim_time : 0.0) * 1000.0;
    Object.values(gameState.players).forEach(playerData => {
        updatePlayerMesh(playerData, serverTime);
    });

    // Update Projectiles (Light Discs)
    updateProjectiles(gameState.projectiles, serverTime);

    // Update Flags
    updateFlags(gameState.flags);

    // Update Midfield Overcharge Node
    updateOverchargeNode(gameState.overcharge_node);

    // Cleanup disconnected or de-rezzed items
    cleanupExpiredEntities(gameState);
}

// 4. PLAYER MESH & TRAIL RENDERING
function updatePlayerMesh(p, serverTime) {
    const teamColor = TEAM_COLORS[p.team];
    let playerObj = meshCache.players[p.id];

    if (!playerObj) {
        // Build the player group
        const group = new THREE.Group();
        
        // Build shape based on Class Type
        let geom;
        let heightOffset = 1.6;
        if (p.class_type === "Stalker") {
            geom = new THREE.OctahedronGeometry(1.6, 0); // Sharp double pyramid
            heightOffset = 1.6;
        } else if (p.class_type === "Enforcer") {
            geom = new THREE.CylinderGeometry(1.3, 1.3, 3.2, 8); // Heavy column
            heightOffset = 1.6;
        } else { // Tactician
            geom = new THREE.OctahedronGeometry(1.8, 0); // Floating Diamond shape
            heightOffset = 1.8;
        }
        
        const mat = new THREE.MeshStandardMaterial({
            color: p.team === "blue" ? 0x002244 : 0x441100, // Dark base color for good contrast
            emissive: teamColor,
            emissiveIntensity: 0.15, // Lower emissive so standard shading is clearly visible
            roughness: 0.2,
            metalness: 0.8
        });
        const bodyMesh = new THREE.Mesh(geom, mat);
        bodyMesh.position.y = heightOffset;
        bodyMesh.castShadow = true;
        group.add(bodyMesh);

        // Add Tron-like neon edge outline for crisp 3D definition
        const edges = new THREE.EdgesGeometry(geom);
        const lineMat = new THREE.LineBasicMaterial({ color: teamColor, linewidth: 2 });
        const edgeLine = new THREE.LineSegments(edges, lineMat);
        bodyMesh.add(edgeLine); // Child of bodyMesh to inherit all parent transformations

        // Add Frontal Hardlight Shield (for Enforcer)
        const shieldGeom = new THREE.SphereGeometry(2.5, 16, 16, 0, Math.PI);
        const shieldMat = new THREE.MeshBasicMaterial({
            color: teamColor,
            transparent: true,
            opacity: 0.25,
            wireframe: true,
            side: THREE.DoubleSide
        });
        const shieldMesh = new THREE.Mesh(shieldGeom, shieldMat);
        shieldMesh.position.y = heightOffset;
        shieldMesh.position.z = 1.0;
        shieldMesh.rotation.y = Math.PI / 2;
        shieldMesh.visible = false;
        group.add(shieldMesh);
        
        // Add glowing name indicator floating tag? (Omitted for neatness, stats are on hud)

        // Light Trail (Rendered as a 3D wall of light with volume and thickness)
        const trailMaxPoints = 30;
        const trailGeom = new THREE.BufferGeometry();
        // 4 vertices per trail point (Bottom-Left, Bottom-Right, Top-Left, Top-Right)
        const trailVertices = new Float32Array(trailMaxPoints * 4 * 3);
        trailGeom.setAttribute("position", new THREE.BufferAttribute(trailVertices, 3));
        
        // 4 color floats (RGBA) per vertex for trail fading gradient
        const trailColors = new Float32Array(trailMaxPoints * 4 * 4);
        trailGeom.setAttribute("color", new THREE.BufferAttribute(trailColors, 4));
        
        // Triangle indices for the 3D wall of light
        const trailIndices = [];
        for (let i = 0; i < trailMaxPoints - 1; i++) {
            const bl = 4 * i;
            const br = 4 * i + 1;
            const tl = 4 * i + 2;
            const tr = 4 * i + 3;
            
            const bl_next = 4 * (i + 1);
            const br_next = 4 * (i + 1) + 1;
            const tl_next = 4 * (i + 1) + 2;
            const tr_next = 4 * (i + 1) + 3;
            
            // Left Face (faces left/outward)
            trailIndices.push(bl, bl_next, tl);
            trailIndices.push(tl, bl_next, tl_next);
            
            // Right Face (faces right/outward)
            trailIndices.push(br_next, br, tr_next);
            trailIndices.push(tr_next, br, tr);
            
            // Top Face (faces upward)
            trailIndices.push(tr, tr_next, tl);
            trailIndices.push(tl, tr_next, tl_next);
        }
        trailGeom.setIndex(trailIndices);
        
        const trailMat = new THREE.MeshBasicMaterial({
            vertexColors: true,
            transparent: true,
            opacity: 0.8,
            side: THREE.DoubleSide,
            depthWrite: false
        });
        const trailLine = new THREE.Mesh(trailGeom, trailMat);
        scene.add(trailLine);

        scene.add(group);
        
        playerObj = {
            group: group,
            bodyMesh: bodyMesh,
            shieldMesh: shieldMesh,
            trailLine: trailLine,
            trailPoints: [],
            lastPos: pyToThreeVec(p.pos),
            prevPos: pyToThreeVec(p.pos),
            targetPos: pyToThreeVec(p.pos),
            vel: p.vel,
            isAlive: p.is_alive,
            lastPacketTime: performance.now(),
            history: []
        };
        meshCache.players[p.id] = playerObj;
    }

    const { group, bodyMesh, shieldMesh, trailLine, trailPoints, lastPos } = playerObj;

    const targetPos = pyToThreeVec(p.pos);
    
    // Smoothly glide position to absorb any remaining playhead or clock jitter.
    // If the distance is large (e.g. on respawn/teleport), snap directly to avoid sliding.
    if (group.position.distanceTo(targetPos) > 10.0) {
        group.position.copy(targetPos);
    } else {
        group.position.lerp(targetPos, 0.35); // 0.35 exponential lerp filter
    }

    // Save variables for tracking and other references
    playerObj.targetPos = targetPos;
    playerObj.vel = p.vel;
    playerObj.isAlive = p.is_alive;

    // Sharp turn rotation (Tron style) using velocity
    const velocity = new THREE.Vector3(p.vel[0], p.vel[2], p.vel[1]);
    if (velocity.lengthSq() > 0.05) {
        const targetAngle = Math.atan2(velocity.x, velocity.z);
        const targetRotation = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), targetAngle);
        group.quaternion.slerp(targetRotation, 0.22); // Fast, responsive slerp to eliminate rotational jitter
    }


    // Handle De-Rez/Death states
    if (!p.is_alive) {
        if (group.visible) {
            group.visible = false;
            trailLine.visible = false;
            // Spawn de-rez particle explosion at last coordinates
            spawnDeRezExplosion(group.position, teamColor);
        }
        return;
    }
    
    group.visible = true;
    trailLine.visible = true;

    // 1. Dashing & Cover & Overcharge (pulse body / hunker down)
    if (p.overcharge_timer > 0.0) {
        // High-intensity pulsing purple glow
        const pulse = 1.0 + Math.sin(Date.now() * 0.02) * 0.15;
        bodyMesh.material.emissive.setHex(0x9f00ff);
        bodyMesh.material.emissiveIntensity = 1.5 * pulse;
        bodyMesh.scale.set(pulse, pulse, pulse);
    } else {
        bodyMesh.material.emissive.setHex(teamColor);
        if (p.is_dashing) {
            bodyMesh.material.emissiveIntensity = 1.0;
            bodyMesh.scale.set(1.4, 0.8, 1.4); // Squash & stretch
        } else if (p.is_taking_cover) {
            bodyMesh.material.emissiveIntensity = 0.05; // Dim glow in cover
            bodyMesh.scale.set(0.9, 0.6, 0.9);         // Hunkered/crouched down
        } else {
            bodyMesh.material.emissiveIntensity = 0.15;
            bodyMesh.scale.set(1, 1, 1);
        }
    }

    // 2. Enforcer Shield
    shieldMesh.visible = p.is_shielding;
    if (p.is_shielding) {
        shieldMesh.rotation.z += 0.05;
    }

    // 3. Tactician healing beam
    updateHealingBeam(p);

    // 4. Update Fading Light Trail
    // Record current position at low interval
    if (!lastPos.equals(group.position)) {
        trailPoints.push(group.position.clone());
        if (trailPoints.length > 29) trailPoints.shift();
        lastPos.copy(group.position);
    }
    
    // Update ribbon buffer geometry and vertex colors for gradient fade
    const positions = trailLine.geometry.attributes.position.array;
    const colors = trailLine.geometry.attributes.color.array;
    const pointsCount = trailPoints.length;
    const c = new THREE.Color(teamColor);
    for (let i = 0; i < 30; i++) {
        const pt = trailPoints[Math.min(i, pointsCount - 1)] || group.position;
        
        // Find direction/tangent to compute normal in horizontal plane (X-Z)
        const prevIdx = Math.max(0, i - 1);
        const nextIdx = Math.min(pointsCount - 1, i + 1);
        const prevPt = trailPoints[prevIdx] || group.position;
        const nextPt = trailPoints[nextIdx] || group.position;
        
        let dx = nextPt.x - prevPt.x;
        let dz = nextPt.z - prevPt.z;
        let len = Math.sqrt(dx * dx + dz * dz);
        
        let nx = 1.0;
        let nz = 0.0;
        if (len > 0.001) {
            // Normal is perpendicular to tangent: (-dz, dx)
            nx = -dz / len;
            nz = dx / len;
        } else if (p.vel) {
            // Fallback to velocity direction
            let v_len = Math.sqrt(p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]);
            if (v_len > 0.001) {
                nx = -p.vel[1] / v_len; // In Python coordinates, Y maps to Three Z
                nz = p.vel[0] / v_len;
            }
        }
        
        const width = 0.22; // Half-width of the trail (total thickness = 0.44 units)
        const baseIdx = 12 * i;
        
        // Bottom-Left
        positions[baseIdx] = pt.x + nx * width;
        positions[baseIdx + 1] = pt.y - 0.1;
        positions[baseIdx + 2] = pt.z + nz * width;
        
        // Bottom-Right
        positions[baseIdx + 3] = pt.x - nx * width;
        positions[baseIdx + 4] = pt.y - 0.1;
        positions[baseIdx + 5] = pt.z - nz * width;
        
        // Top-Left
        positions[baseIdx + 6] = pt.x + nx * width;
        positions[baseIdx + 7] = pt.y + 1.0;
        positions[baseIdx + 8] = pt.z + nz * width;
        
        // Top-Right
        positions[baseIdx + 9] = pt.x - nx * width;
        positions[baseIdx + 10] = pt.y + 1.0;
        positions[baseIdx + 11] = pt.z - nz * width;
        
        // Set colors with a gradient to nothing (alpha fades to 0 at the tail end i=0)
        const alpha = i / 29.0;
        for (let v = 0; v < 4; v++) {
            const colorIdx = 16 * i + 4 * v;
            colors[colorIdx] = c.r;
            colors[colorIdx + 1] = c.g;
            colors[colorIdx + 2] = c.b;
            colors[colorIdx + 3] = alpha;
        }
    }
    trailLine.geometry.attributes.position.needsUpdate = true;
    trailLine.geometry.attributes.color.needsUpdate = true;
}

function updateHealingBeam(healer) {
    let beam = meshCache.healingBeams[healer.id];
    
    if (!healer.is_healing || healer.healing_target_id === null) {
        if (beam) {
            scene.remove(beam);
            delete meshCache.healingBeams[healer.id];
        }
        return;
    }
    
    const targetPlayer = currentGameState.players[healer.healing_target_id];
    if (!targetPlayer || !targetPlayer.is_alive) {
        if (beam) {
            scene.remove(beam);
            delete meshCache.healingBeams[healer.id];
        }
        return;
    }

    // Get positions
    const pHealer = pyToThreeVec(healer.pos).add(new THREE.Vector3(0, 1.5, 0));
    const pTarget = pyToThreeVec(targetPlayer.pos).add(new THREE.Vector3(0, 1.5, 0));

    if (!beam) {
        // Create glowing neon line cylinder
        const geom = new THREE.BufferGeometry().setFromPoints([pHealer, pTarget]);
        const mat = new THREE.LineBasicMaterial({
            color: TEAM_COLORS.blue, // Lime Green for Tacticians is cool but let's use the team color
            linewidth: 3,
            transparent: true,
            opacity: 0.8
        });
        beam = new THREE.Line(geom, mat);
        scene.add(beam);
        meshCache.healingBeams[healer.id] = beam;
    } else {
        // Update line endpoints
        const positions = beam.geometry.attributes.position.array;
        positions[0] = pHealer.x;
        positions[1] = pHealer.y;
        positions[2] = pHealer.z;
        positions[3] = pTarget.x;
        positions[4] = pTarget.y;
        positions[5] = pTarget.z;
        beam.geometry.attributes.position.needsUpdate = true;
    }
}

// 5. PROJECTILES (LIGHT DISCS)
function updateProjectiles(projectiles, serverTime) {
    projectiles.forEach(proj => {
        let projObj = meshCache.projectiles[proj.id];
        const targetPos = pyToThreeVec(proj.pos);
        
        if (!projObj) {
            // Spawn layered Tron energy light disc
            const group = new THREE.Group();
            
            // Outer Rim: Torus geometry in team color
            const outerGeom = new THREE.TorusGeometry(0.8, 0.08, 8, 32);
            const outerMat = new THREE.MeshBasicMaterial({ color: TEAM_COLORS[proj.team] });
            const outerMesh = new THREE.Mesh(outerGeom, outerMat);
            group.add(outerMesh);
            
            // Inner Core: Translucent white ring to create a strong energy core bloom
            const innerGeom = new THREE.RingGeometry(0.15, 0.72, 32);
            const innerMat = new THREE.MeshBasicMaterial({
                color: 0xffffff,
                transparent: true,
                opacity: 0.65,
                side: THREE.DoubleSide
            });
            const innerMesh = new THREE.Mesh(innerGeom, innerMat);
            group.add(innerMesh);
            
            // Central Hub: Small central ring in team color
            const hubGeom = new THREE.TorusGeometry(0.18, 0.04, 8, 16);
            const hubMat = new THREE.MeshBasicMaterial({ color: TEAM_COLORS[proj.team] });
            const hubMesh = new THREE.Mesh(hubGeom, hubMat);
            group.add(hubMesh);
            
            group.position.copy(targetPos);
            group.rotation.x = Math.PI / 2; // Lie flat
            scene.add(group);
            
            // Let's create a ribbon trail for the projectile!
            const trailMaxPoints = 12;
            const trailGeom = new THREE.BufferGeometry();
            const trailVertices = new Float32Array(trailMaxPoints * 4 * 3);
            trailGeom.setAttribute("position", new THREE.BufferAttribute(trailVertices, 3));
            
            const trailColors = new Float32Array(trailMaxPoints * 4 * 4);
            trailGeom.setAttribute("color", new THREE.BufferAttribute(trailColors, 4));
            
            const trailIndices = [];
            for (let i = 0; i < trailMaxPoints - 1; i++) {
                const bl = 4 * i;
                const br = 4 * i + 1;
                const tl = 4 * i + 2;
                const tr = 4 * i + 3;
                
                const bl_next = 4 * (i + 1);
                const br_next = 4 * (i + 1) + 1;
                const tl_next = 4 * (i + 1) + 2;
                const tr_next = 4 * (i + 1) + 3;
                
                // Bottom-to-Top Face / Sides
                trailIndices.push(bl, bl_next, tl);
                trailIndices.push(tl, bl_next, tl_next);
                
                trailIndices.push(br_next, br, tr_next);
                trailIndices.push(tr_next, br, tr);
                
                // Top Flat Face
                trailIndices.push(tr, tr_next, tl);
                trailIndices.push(tl, tr_next, tl_next);
            }
            trailGeom.setIndex(trailIndices);
            
            const trailMat = new THREE.MeshBasicMaterial({
                vertexColors: true,
                transparent: true,
                opacity: 0.8,
                side: THREE.DoubleSide,
                depthWrite: false
            });
            const trailMesh = new THREE.Mesh(trailGeom, trailMat);
            scene.add(trailMesh);
            
            projObj = { 
                mesh: group, 
                targetPos: targetPos.clone(), 
                vel: proj.vel,
                trailMesh: trailMesh,
                trailPoints: [targetPos.clone()],
                lastPos: targetPos.clone()
            };
            meshCache.projectiles[proj.id] = projObj;
        }

        // Set position directly from interpolated target position
        projObj.mesh.position.copy(targetPos);

        // Record projectile movement history for trail points
        if (!projObj.lastPos.equals(targetPos)) {
            projObj.trailPoints.push(targetPos.clone());
            if (projObj.trailPoints.length > 12) {
                projObj.trailPoints.shift();
            }
            projObj.lastPos.copy(targetPos);
        }

        // Update trail buffer geometry and opacity gradient
        const trailPositions = projObj.trailMesh.geometry.attributes.position.array;
        const trailColors = projObj.trailMesh.geometry.attributes.color.array;
        const pointsCount = projObj.trailPoints.length;
        const color = new THREE.Color(TEAM_COLORS[proj.team]);
        const trailMaxPoints = 12;

        for (let i = 0; i < trailMaxPoints; i++) {
            const pt = projObj.trailPoints[Math.min(i, pointsCount - 1)] || targetPos;
            
            // Calculate normal vector perpendicular to projectile trajectory in X-Z plane
            const prevIdx = Math.max(0, i - 1);
            const nextIdx = Math.min(pointsCount - 1, i + 1);
            const prevPt = projObj.trailPoints[prevIdx] || targetPos;
            const nextPt = projObj.trailPoints[nextIdx] || targetPos;
            
            let dx = nextPt.x - prevPt.x;
            let dz = nextPt.z - prevPt.z;
            let len = Math.sqrt(dx * dx + dz * dz);
            
            let nx = 1.0;
            let nz = 0.0;
            if (len > 0.001) {
                nx = -dz / len;
                nz = dx / len;
            } else if (projObj.vel) {
                let v_len = Math.sqrt(projObj.vel[0] * projObj.vel[0] + projObj.vel[1] * projObj.vel[1]);
                if (v_len > 0.001) {
                    nx = -projObj.vel[1] / v_len;
                    nz = projObj.vel[0] / v_len;
                }
            }
            
            const width = 0.35;       // Trail width slightly smaller than disk torus radius (0.8)
            const heightHalf = 0.03;  // Thin vertical thickness for flat light discs
            const baseIdx = 12 * i;
            
            // Bottom-Left
            trailPositions[baseIdx] = pt.x + nx * width;
            trailPositions[baseIdx + 1] = pt.y - heightHalf;
            trailPositions[baseIdx + 2] = pt.z + nz * width;
            
            // Bottom-Right
            trailPositions[baseIdx + 3] = pt.x - nx * width;
            trailPositions[baseIdx + 4] = pt.y - heightHalf;
            trailPositions[baseIdx + 5] = pt.z - nz * width;
            
            // Top-Left
            trailPositions[baseIdx + 6] = pt.x + nx * width;
            trailPositions[baseIdx + 7] = pt.y + heightHalf;
            trailPositions[baseIdx + 8] = pt.z + nz * width;
            
            // Top-Right
            trailPositions[baseIdx + 9] = pt.x - nx * width;
            trailPositions[baseIdx + 10] = pt.y + heightHalf;
            trailPositions[baseIdx + 11] = pt.z - nz * width;
            
            // Color and opacity gradient (fading to 0 at index 0)
            const alpha = i / (trailMaxPoints - 1);
            for (let v = 0; v < 4; v++) {
                const colorIdx = 16 * i + 4 * v;
                trailColors[colorIdx] = color.r;
                trailColors[colorIdx + 1] = color.g;
                trailColors[colorIdx + 2] = color.b;
                trailColors[colorIdx + 3] = alpha * 0.75; // Max opacity 0.75
            }
        }
        projObj.trailMesh.geometry.attributes.position.needsUpdate = true;
        projObj.trailMesh.geometry.attributes.color.needsUpdate = true;

        // Save dynamic target position for reference
        projObj.targetPos = targetPos;
        projObj.vel = proj.vel;
        projObj.updated = true;
    });

    // Remove projectiles no longer in physics loop
    Object.keys(meshCache.projectiles).forEach(id => {
        const match = projectiles.find(p => p.id == id);
        if (!match) {
            // Impact animation splash
            const mesh = meshCache.projectiles[id].mesh;
            const trailMesh = meshCache.projectiles[id].trailMesh;
            let colorHex = 0xffffff;
            if (mesh.material && mesh.material.color) {
                colorHex = mesh.material.color.getHex();
            } else if (mesh.children && mesh.children[0] && mesh.children[0].material) {
                colorHex = mesh.children[0].material.color.getHex();
            }
            spawnDeRezExplosion(mesh.position, colorHex, 8);
            scene.remove(mesh);
            if (trailMesh) {
                scene.remove(trailMesh);
            }
            delete meshCache.projectiles[id];
        }
    });
}

// 5.5 MIDFIELD OVERCHARGE NODE INTERACTION
function updateOverchargeNode(nodeData) {
    if (!nodeData) return;
    if (nodeData.active) {
        if (!meshCache.overchargeNode) {
            const group = new THREE.Group();
            const nodePos = pyToThreeVec(nodeData.pos);
            group.position.copy(nodePos);
            
            // Outer glowing purple wireframe icosahedron
            const outerGeo = new THREE.IcosahedronGeometry(2.0, 1);
            const outerMat = new THREE.MeshBasicMaterial({
                color: 0x9f00ff,
                wireframe: true,
                transparent: true,
                opacity: 0.8
            });
            const outerMesh = new THREE.Mesh(outerGeo, outerMat);
            group.add(outerMesh);
            
            // Inner white/bright core
            const innerGeo = new THREE.SphereGeometry(0.8, 16, 16);
            const innerMat = new THREE.MeshStandardMaterial({
                color: 0xffffff,
                emissive: 0x9f00ff,
                emissiveIntensity: 2.0,
                roughness: 0.1,
                metalness: 0.1
            });
            const innerMesh = new THREE.Mesh(innerGeo, innerMat);
            group.add(innerMesh);
            
            // Neon vertical laser beam from the sky
            const beamGeo = new THREE.CylinderGeometry(1.5, 1.5, 150, 16, 1, true);
            const beamMat = new THREE.MeshBasicMaterial({
                color: 0x9f00ff,
                transparent: true,
                opacity: 0.3,
                blending: THREE.AdditiveBlending,
                side: THREE.DoubleSide,
                depthWrite: false
            });
            const beamMesh = new THREE.Mesh(beamGeo, beamMat);
            beamMesh.position.y = 75; // Centered at y=75 relative to the node, going up to y=150
            group.add(beamMesh);
            
            scene.add(group);
            meshCache.overchargeNode = group;
        }
        meshCache.overchargeNode.visible = true;
        
        // Animate rotation & floating
        const time = Date.now() * 0.003;
        meshCache.overchargeNode.rotation.y = time * 0.5;
        meshCache.overchargeNode.rotation.x = time * 0.3;
        
        // Animate beam rotation and slight pulsing/opacity flicker
        if (meshCache.overchargeNode.children[2]) {
            const beam = meshCache.overchargeNode.children[2];
            beam.rotation.y = -time * 0.2;
            const beamPulse = 1.0 + Math.sin(time * 6.0) * 0.08;
            beam.scale.set(beamPulse, 1.0, beamPulse);
            beam.material.opacity = 0.22 + Math.sin(time * 15.0) * 0.04;
        }
        
        // Float relative to base Python Z coordinate mapped to Three Y
        const baseZ = nodeData.pos[2];
        meshCache.overchargeNode.position.y = baseZ + 1.5 + Math.sin(time * 2.0) * 0.4;
    } else {
        if (meshCache.overchargeNode) {
            meshCache.overchargeNode.visible = false;
        }
    }
}

// 6. FLAGS INTERACTION
function updateFlags(flagsData) {
    for (const team in flagsData) {
        const data = flagsData[team];
        const flagObj = meshCache.flags[team];
        if (!flagObj) continue;

        let targetPos;
        const time = Date.now() * 0.003;
        const bounce = Math.sin(time) * 0.6;

        let finalTarget = new THREE.Vector3();
        if (data.carrier_id !== null) {
            const carrierObj = meshCache.players[data.carrier_id];
            if (carrierObj && carrierObj.group.visible && carrierObj.isAlive) {
                targetPos = carrierObj.group.position;
                finalTarget.set(targetPos.x, targetPos.y + 3.0, targetPos.z);
            } else {
                targetPos = pyToThreeVec(data.pos);
                finalTarget.set(targetPos.x, targetPos.y + 2.0 + bounce, targetPos.z);
            }
        } else {
            targetPos = pyToThreeVec(data.pos);
            finalTarget.set(targetPos.x, targetPos.y + 2.0 + bounce, targetPos.z);
        }
        
        // Smoothly lerp flag to target position (or snap if far away)
        if (flagObj.group.position.distanceTo(finalTarget) > 15.0) {
            flagObj.group.position.copy(finalTarget);
        } else {
            flagObj.group.position.lerp(finalTarget, 0.25);
        }
        
        // Rotate the floating top ring
        flagObj.coreMesh.rotation.y += 0.02;
        flagObj.coreMesh.rotation.x = Math.sin(time * 0.5) * 0.2;
        
        // Change color pulse based on stolen state
        if (data.carrier_id !== null) {
            flagObj.coreMesh.material.emissiveIntensity = 2.5;
            // Pulsing scale
            const scale = 1.0 + Math.sin(time * 5) * 0.15;
            flagObj.coreMesh.scale.set(scale, scale, scale);
        } else {
            flagObj.coreMesh.material.emissiveIntensity = 1.2;
            flagObj.coreMesh.scale.set(1, 1, 1);
        }
    }
}

// Particle System for explosions
function spawnDeRezExplosion(pos, color, count = 25) {
    const geom = new THREE.BufferGeometry();
    const positions = [];
    const velocities = [];
    
    for (let i = 0; i < count; i++) {
        positions.push(pos.x, pos.y, pos.z);
        // Random spherical direction
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.acos((Math.random() * 2) - 1);
        const speed = 5.0 + Math.random() * 15.0;
        
        velocities.push(
            Math.sin(phi) * Math.cos(theta) * speed,
            Math.sin(phi) * Math.sin(theta) * speed,
            Math.cos(phi) * speed
        );
    }
    
    geom.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    
    // Glowing particle texture
    const mat = new THREE.PointsMaterial({
        color: color,
        size: 0.6,
        transparent: true,
        opacity: 1.0,
        blending: THREE.AdditiveBlending
    });
    
    const pSystem = new THREE.Points(geom, mat);
    scene.add(pSystem);
    
    particleGroups.push({
        points: pSystem,
        velocities: velocities,
        age: 0.0,
        maxAge: 0.8
    });
}

function updateParticles(dt) {
    for (let i = particleGroups.length - 1; i >= 0; i--) {
        const p = particleGroups[i];
        p.age += dt;
        
        if (p.age >= p.maxAge) {
            scene.remove(p.points);
            particleGroups.splice(i, 1);
            continue;
        }

        const positions = p.points.geometry.attributes.position.array;
        const opacity = 1.0 - (p.age / p.maxAge);
        p.points.material.opacity = opacity;
        
        // Move particles
        for (let j = 0; j < positions.length / 3; j++) {
            positions[j * 3] += p.velocities[j * 3] * dt;
            positions[j * 3 + 1] += p.velocities[j * 3 + 1] * dt;
            positions[j * 3 + 2] += p.velocities[j * 3 + 2] * dt;
        }
        p.points.geometry.attributes.position.needsUpdate = true;
    }
}

// 7. CLEANUP ENTITIES
function cleanupExpiredEntities(gameState) {
    // Check players de-rez state or removal
    Object.keys(meshCache.players).forEach(pid => {
        if (!gameState.players[pid]) {
            // Remove completely
            scene.remove(meshCache.players[pid].group);
            scene.remove(meshCache.players[pid].trailLine);
            delete meshCache.players[pid];
        }
    });
}

// 8. CAMERA MODES CONTROLS (GHOST FLIGHT VS ACTION TRACKING)
function updateCamera(dt) {
    if (cameraMode === "ghost") {
        orbitControls.enabled = true;
        
        // Ghost flying flight inputs
        const moveSpeed = 60.0 * dt;
        const forward = new THREE.Vector3();
        camera.getWorldDirection(forward);
        forward.y = 0; // Lock vertical movement
        forward.normalize();
        
        const right = new THREE.Vector3();
        right.crossVectors(forward, camera.up).normalize();

        // Keyboard Flight navigation
        if (keyStates['KeyW']) camera.position.addScaledVector(forward, moveSpeed);
        if (keyStates['KeyS']) camera.position.addScaledVector(forward, -moveSpeed);
        if (keyStates['KeyA']) camera.position.addScaledVector(right, -moveSpeed);
        if (keyStates['KeyD']) camera.position.addScaledVector(right, moveSpeed);
        if (keyStates['Space']) camera.position.y += moveSpeed;
        if (keyStates['ShiftLeft']) camera.position.y -= moveSpeed;
        
        // Keep camera target aligned with current movement
        orbitControls.target.addScaledVector(forward, (keyStates['KeyW']?moveSpeed:0) - (keyStates['KeyS']?moveSpeed:0));
        orbitControls.target.addScaledVector(right, (keyStates['KeyD']?moveSpeed:0) - (keyStates['KeyA']?moveSpeed:0));
        
        orbitControls.update();
    } else if (cameraMode === "action" && currentGameState) {
        orbitControls.enabled = false;
        
        // Determine camera lookAt target
        let lookTargetPos = new THREE.Vector3(0, 0, 0);
        let trackingPlayer = null;
        
        if (trackingTargetId === "auto") {
            // AUTO FOCUS DECISION MATRIX
            // 1. Focus on flag carriers first
            let carrier = Object.values(currentGameState.players).find(p => p.has_flag && p.is_alive);
            if (carrier) {
                trackingPlayer = carrier;
            } else {
                // 2. Focus on whoever is in combat (e.g. throwing discs / healing / close to base)
                let combatants = Object.values(currentGameState.players).filter(p => p.is_alive && (p.is_healing || p.is_dashing || p.is_shielding));
                if (combatants.length > 0) {
                    trackingPlayer = combatants[0];
                } else {
                    // 3. Focus on closest player to enemy flag
                    let alivePlayers = Object.values(currentGameState.players).filter(p => p.is_alive);
                    if (alivePlayers.length > 0) {
                        trackingPlayer = alivePlayers[0];
                    }
                }
            }
        } else {
            // Manual selection from select tag
            const targetId = parseInt(trackingTargetId);
            trackingPlayer = currentGameState.players[targetId];
        }

        if (trackingPlayer && trackingPlayer.is_alive) {
            const pObj = meshCache.players[trackingPlayer.id];
            if (pObj) {
                lookTargetPos.copy(pObj.group.position);
            } else {
                lookTargetPos = pyToThreeVec(trackingPlayer.pos);
            }
            
            // Compute tracking camera position
            // Calculate a position slightly behind and above the player's position
            const playerVel = new THREE.Vector3(trackingPlayer.vel[0], trackingPlayer.vel[2], trackingPlayer.vel[1]);
            const facingDir = new THREE.Vector3(0, 0, 1);
            if (playerVel.lengthSq() > 0.1) {
                facingDir.copy(playerVel).normalize();
            } else {
                // Point towards opponent's base by default
                facingDir.set(trackingPlayer.team === "blue" ? 1 : -1, 0, 0);
            }
            
            // Desired camera coordinate offsets
            const distanceBehind = 35.0;
            const heightOffset = 18.0;
            
            const targetCamPos = lookTargetPos.clone()
                .addScaledVector(facingDir, -distanceBehind)
                .add(new THREE.Vector3(0, heightOffset, 0));
                
            // Smoothly interpolate camera position & lookAt Target
            camera.position.lerp(targetCamPos, 0.08);
            
            // Smoothly interpolate lookAt
            if (!camera.userData.smoothedLookAt) camera.userData.smoothedLookAt = new THREE.Vector3();
            camera.userData.smoothedLookAt.lerp(lookTargetPos, 0.1);
            camera.lookAt(camera.userData.smoothedLookAt);
        } else {
            // Default center viewing if target is de-rezzed
            const centerTarget = new THREE.Vector3(0, 10, 0);
            camera.position.lerp(new THREE.Vector3(0, 70, 130), 0.05);
            camera.lookAt(centerTarget);
        }
    }
}

// 9. ANIMATION LOOP
let lastFrameTime = performance.now();
let fpsFrames = 0;
let fpsLastTime = performance.now();

function interpolateState(stateA, stateB, t) {
    const interpolated = {
        state: stateA.state,
        timer: stateA.timer,
        match_time: stateA.match_time,
        scores: stateA.scores,
        players: {},
        flags: {},
        projectiles: [],
        tactics: stateA.tactics,
        audit_report: stateA.audit_report,
        audit_loading: stateA.audit_loading,
        sim_time: stateA.sim_time + (stateB.sim_time - stateA.sim_time) * t,
        logs: stateA.logs
    };

    // 1. Interpolate players
    for (const pid in stateA.players) {
        const pA = stateA.players[pid];
        const pB = stateB.players[pid];

        if (pB) {
            const dist = Math.sqrt(
                Math.pow(pB.pos[0] - pA.pos[0], 2) +
                Math.pow(pB.pos[1] - pA.pos[1], 2) +
                Math.pow(pB.pos[2] - pA.pos[2], 2)
            );
            
            let interpPos;
            if (dist > 12.0) {
                interpPos = t < 0.5 ? pA.pos : pB.pos;
            } else {
                interpPos = [
                    pA.pos[0] + (pB.pos[0] - pA.pos[0]) * t,
                    pA.pos[1] + (pB.pos[1] - pA.pos[1]) * t,
                    pA.pos[2] + (pB.pos[2] - pA.pos[2]) * t
                ];
            }

            interpolated.players[pid] = {
                ...pA,
                pos: interpPos,
                vel: [
                    pA.vel[0] + (pB.vel[0] - pA.vel[0]) * t,
                    pA.vel[1] + (pB.vel[1] - pA.vel[1]) * t,
                    pA.vel[2] + (pB.vel[2] - pA.vel[2]) * t
                ],
                hp: pA.hp + (pB.hp - pA.hp) * t,
                shield: pA.shield + (pB.shield - pA.shield) * t
            };
        } else {
            interpolated.players[pid] = pA;
        }
    }

    // 2. Interpolate flags
    for (const team in stateA.flags) {
        const flagA = stateA.flags[team];
        const flagB = stateB.flags[team];

        if (flagB) {
            interpolated.flags[team] = {
                ...flagA,
                pos: [
                    flagA.pos[0] + (flagB.pos[0] - flagA.pos[0]) * t,
                    flagA.pos[1] + (flagB.pos[1] - flagA.pos[1]) * t,
                    flagA.pos[2] + (flagB.pos[2] - flagA.pos[2]) * t
                ]
            };
        } else {
            interpolated.flags[team] = flagA;
        }
    }

    // 3. Interpolate projectiles
    stateA.projectiles.forEach(projA => {
        const projB = stateB.projectiles.find(p => p.id === projA.id);
        if (projB) {
            interpolated.projectiles.push({
                ...projA,
                pos: [
                    projA.pos[0] + (projB.pos[0] - projA.pos[0]) * t,
                    projA.pos[1] + (projB.pos[1] - projA.pos[1]) * t,
                    projA.pos[2] + (projB.pos[2] - projA.pos[2]) * t
                ],
                vel: [
                    projA.vel[0] + (projB.vel[0] - projA.vel[0]) * t,
                    projA.vel[1] + (projB.vel[1] - projA.vel[1]) * t,
                    projA.vel[2] + (projB.vel[2] - projA.vel[2]) * t
                ]
            });
        } else {
            const dtLocal = interpolated.sim_time - stateA.sim_time;
            interpolated.projectiles.push({
                ...projA,
                pos: [
                    projA.pos[0] + projA.vel[0] * dtLocal,
                    projA.pos[1] + projA.vel[1] * dtLocal,
                    projA.pos[2] + projA.vel[2] * dtLocal
                ]
            });
        }
    });

    return interpolated;
}

function animate() {
    requestAnimationFrame(animate);

    const now = performance.now();
    const dt = Math.min((now - lastFrameTime) / 1000.0, 0.05); // cap at 50ms to absorb GC/tab-switch stalls
    lastFrameTime = now;

    // Calculate and display FPS (updated every 300ms to prevent text flickering)
    fpsFrames++;
    if (now >= fpsLastTime + 300) {
        const fps = Math.round((fpsFrames * 1000) / (now - fpsLastTime));
        const fpsEl = document.getElementById("fps-counter");
        if (fpsEl) {
            fpsEl.innerText = `FPS: ${fps}`;
            if (fps >= 55) {
                fpsEl.style.color = "#39ff14"; // Lime Green (active)
            } else if (fps >= 30) {
                fpsEl.style.color = "#a0aec0"; // Slate Grey (stable)
            } else {
                fpsEl.style.color = "#ef4444"; // Crimson Red (stuttering)
            }
        }
        fpsFrames = 0;
        fpsLastTime = now;
    }

    // Playhead progression and interpolation
    if (stateBuffer.length > 0) {
        const latestSimTime = stateBuffer[stateBuffer.length - 1].sim_time;
        const earliestSimTime = stateBuffer[0].sim_time;

        if (clientRenderTime === null) {
            clientRenderTime = latestSimTime - interpolationDelay;
        } else {
            // Count how many states in the buffer are ahead of our current playhead
            const framesAhead = stateBuffer.filter(s => s.sim_time > clientRenderTime).length;
            
            // Adjust speed dynamically to maintain a stable buffer cushion (target: 3-5 frames ahead)
            // We use a broad deadband [2, 6] to keep the playback speed at exactly 1.0x under normal network conditions.
            let targetTimeScale = 1.0;
            if (framesAhead === 0) {
                targetTimeScale = 0.0;      // Run out of frames! Pause immediately to prevent snapping.
            } else if (framesAhead === 1) {
                targetTimeScale = 0.4;      // Starving, slow down.
            } else if (framesAhead >= 2 && framesAhead <= 6) {
                targetTimeScale = 1.0;      // Ideal stable deadband zone. Play at constant 1.0x speed.
            } else if (framesAhead >= 7 && framesAhead <= 9) {
                targetTimeScale = 1.15;     // Slightly high buffer, gentle catch-up.
            } else {
                targetTimeScale = 1.4;      // Far behind, fast catch-up.
            }

            // Low phase lag smoothing (lerp factor 0.4) to quickly adjust without rubber-banding
            clientTimeScale = clientTimeScale * 0.6 + targetTimeScale * 0.4;
            
            // Apply scale to elapsed time
            clientRenderTime += dt * clientTimeScale;
        }

        // Clamp render time to valid buffer range — never snap backward (that causes visible jumps)
        // Clamp to slightly less than latestSimTime to avoid hitting the exact end where interpolation fails
        const maxClampTime = Math.max(earliestSimTime, latestSimTime - 0.001);
        clientRenderTime = Math.max(earliestSimTime, Math.min(maxClampTime, clientRenderTime));

        // Find stateA and stateB for interpolation
        let stateA = null;
        let stateB = null;

        for (let i = 0; i < stateBuffer.length - 1; i++) {
            if (stateBuffer[i].sim_time <= clientRenderTime && stateBuffer[i + 1].sim_time > clientRenderTime) {
                stateA = stateBuffer[i];
                stateB = stateBuffer[i + 1];
                break;
            }
        }

        // Fallbacks
        if (!stateA) {
            if (clientRenderTime < stateBuffer[0].sim_time) {
                stateA = stateBuffer[0];
                stateB = stateBuffer[0];
            } else {
                stateA = stateBuffer[stateBuffer.length - 1];
                stateB = stateBuffer[stateBuffer.length - 1];
            }
        }

        if (stateA && stateB) {
            let t = 0.0;
            const denom = stateB.sim_time - stateA.sim_time;
            if (denom > 0.0001) {
                t = (clientRenderTime - stateA.sim_time) / denom;
            }
            t = Math.max(0.0, Math.min(1.0, t));

            // Create interpolated state
            const interpolatedState = interpolateState(stateA, stateB, t);
            currentGameState = interpolatedState;

            // Process visual updates for entities based on interpolated state
            processStateUpdate(interpolatedState);
        }
    }

    // Spin projectiles (only rotation update, position is set directly in updateProjectiles)
    for (const id in meshCache.projectiles) {
        const projObj = meshCache.projectiles[id];
        projObj.mesh.rotation.z += 0.25; // Spin disc
    }

    // Update Particles
    updateParticles(dt);

    // Floor grid breathing pulsation
    if (gridBlue && gridOrange) {
        let matchTime = 180.0;
        if (currentGameState && currentGameState.match_time !== undefined) {
            matchTime = Math.max(0.0, currentGameState.match_time);
        }
        const maxTime = 180.0;
        const timeRatio = 1.0 - (matchTime / maxTime); // 0.0 at start, 1.0 at end
        const pulseFreq = 1.5 + timeRatio * 8.5; // pulses faster as time runs down (1.5Hz to 10Hz)
        const time = Date.now() * 0.001;
        const pulseVal = Math.sin(time * pulseFreq * 2.0 * Math.PI); // full sine wave cycles
        const opacity = 0.15 + (pulseVal + 1.0) * 0.5 * 0.20; // range 0.15 to 0.35
        gridBlue.material.opacity = opacity;
        gridOrange.material.opacity = opacity;
    }

    // Update camera positions based on Mode
    updateCamera(dt);

    // Render Scene with or without Neon Glow
    if (useBloom) {
        composer.render();
    } else {
        renderer.render(scene, camera);
    }
}

// 10. HUD UI UPDATES
function updateHUD(gameState) {
    // Scores
    document.getElementById("score-blue").innerText = gameState.scores.blue;
    document.getElementById("score-orange").innerText = gameState.scores.orange;

    // Roster tables
    const blueRoster = document.getElementById("blue-roster");
    const orangeRoster = document.getElementById("orange-roster");

    Object.values(gameState.players).forEach(p => {
        const cardId = `bot-card-${p.id}`;
        let card = document.getElementById(cardId);
        
        // Percentages for Health and Shield bars
        const hpPercent = (p.hp / p.max_hp) * 100;
        const shieldPercent = (p.shield / p.max_shield) * 100;
        
        // Display cover state if player is taking cover
        let displayState = p.state;
        if (p.is_alive && p.is_taking_cover) {
            displayState = "COVER";
        }
        
        const stateText = p.is_alive ? displayState : 'DE-REZZING';
        
        if (!card) {
            // Create card if it doesn't exist yet
            card = document.createElement("div");
            card.id = cardId;
            card.className = `bot-card ${p.is_alive ? '' : 'dead'}`;
            
            card.innerHTML = `
                <div class="bot-header">
                    <span class="bot-name">${p.name}</span>
                    <span class="bot-class">${p.class_type}</span>
                    <span class="bot-state">${stateText}</span>
                </div>
                <div class="meters">
                    <div class="meter-row">
                        <span class="meter-label">HP</span>
                        <div class="meter-track">
                            <div class="meter-fill hp-fill" style="width: ${hpPercent}%"></div>
                        </div>
                    </div>
                    <div class="meter-row">
                        <span class="meter-label">SHD</span>
                        <div class="meter-track">
                            <div class="meter-fill shield-fill" style="width: ${shieldPercent}%"></div>
                        </div>
                    </div>
                </div>
                <div class="bot-kd">
                    <span class="kd-stats">K: ${p.kills} | D: ${p.deaths} | C: ${p.captures}</span>
                    <span class="dmg-stats">DMG: ${p.damage_dealt} | HEAL: ${p.healing_done}</span>
                </div>
                <div class="flag-badge-container"></div>
            `;
            
            if (p.team === "blue") {
                blueRoster.appendChild(card);
            } else {
                orangeRoster.appendChild(card);
            }
        } else {
            // Update existing card properties in-place (no layout recalculations!)
            card.className = `bot-card ${p.is_alive ? '' : 'dead'}`;
            
            const stateEl = card.querySelector(".bot-state");
            if (stateEl && stateEl.innerText !== stateText) stateEl.innerText = stateText;
            
            const kdEl = card.querySelector(".kd-stats");
            const expectedKD = `K: ${p.kills} | D: ${p.deaths} | C: ${p.captures}`;
            if (kdEl && kdEl.innerText !== expectedKD) kdEl.innerText = expectedKD;
            
            const dmgEl = card.querySelector(".dmg-stats");
            const expectedDmg = `DMG: ${p.damage_dealt} | HEAL: ${p.healing_done}`;
            if (dmgEl && dmgEl.innerText !== expectedDmg) dmgEl.innerText = expectedDmg;
            
            const hpFillEl = card.querySelector(".hp-fill");
            if (hpFillEl) hpFillEl.style.width = `${hpPercent}%`;
            
            const shieldFillEl = card.querySelector(".shield-fill");
            if (shieldFillEl) shieldFillEl.style.width = `${shieldPercent}%`;
        }
        
        // Update carried flag status
        const badgeContainer = card.querySelector(".flag-badge-container");
        if (badgeContainer) {
            if (p.has_flag) {
                if (badgeContainer.innerHTML === "") {
                    badgeContainer.innerHTML = '<div class="flag-badge">HAS FLAG</div>';
                }
            } else {
                if (badgeContainer.innerHTML !== "") {
                    badgeContainer.innerHTML = "";
                }
            }
        }
    });

    // Update Match Log console (optimised append to prevent browser freezing)
    const logBox = document.getElementById("match-log");
    if (gameState.logs.length < logOffset) {
        logBox.innerHTML = "";
        logOffset = 0;
    }
    if (gameState.logs.length > logOffset) {
        const wasScrolledToBottom = logBox.scrollHeight - logBox.clientHeight <= logBox.scrollTop + 10;
        for (let i = logOffset; i < gameState.logs.length; i++) {
            const log = gameState.logs[i];
            const div = document.createElement("div");
            div.className = "log-entry";
            if (log.includes("SCORE")) {
                div.innerHTML = `<span class="neon-text-blue">${log}</span>`;
            } else if (log.includes("Flag")) {
                div.innerHTML = `<span class="neon-text-orange">${log}</span>`;
            } else {
                div.innerHTML = `<span>${log}</span>`;
            }
            logBox.appendChild(div);
        }
        logOffset = gameState.logs.length;
        if (wasScrolledToBottom) {
            logBox.scrollTop = logBox.scrollHeight;
        }
    }
}

function updateTacticsOverlay(tactics) {
    // Sidebar update
    document.getElementById("blue-strategy-title").innerText = tactics.blue.strategy;
    document.getElementById("blue-strategy-rationale").innerText = tactics.blue.rationale;
    document.getElementById("blue-strategy-source").innerText = tactics.blue.source;
    
    document.getElementById("orange-strategy-title").innerText = tactics.orange.strategy;
    document.getElementById("orange-strategy-rationale").innerText = tactics.orange.rationale;
    document.getElementById("orange-strategy-source").innerText = tactics.orange.source;

    // Pregame overlay status indicators
    const blueStatus = document.getElementById("blue-tactics-status");
    const orangeStatus = document.getElementById("orange-tactics-status");

    if (tactics.blue.source !== "Default") {
        blueStatus.innerText = "COMPILED";
        blueStatus.className = "status-indicator ready";
    } else {
        blueStatus.innerText = "LOADING";
        blueStatus.className = "status-indicator loading";
    }

    if (tactics.orange.source !== "Default") {
        orangeStatus.innerText = "COMPILED";
        orangeStatus.className = "status-indicator ready";
    } else {
        orangeStatus.innerText = "LOADING";
        orangeStatus.className = "status-indicator loading";
    }
}

// 11. HELPER / EVENT HANDLERS
function onWindowResize() {
    const container = document.getElementById("canvas-container");
    const w = container.clientWidth;
    const h = container.clientHeight;
    
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    
    renderer.setSize(w, h);
    composer.setSize(w, h);
}

function typewriteText(element, text) {
    element.innerText = "";
    let i = 0;
    // Clear old interval if any
    if (element.typewriterInterval) clearInterval(element.typewriterInterval);
    
    element.typewriterInterval = setInterval(() => {
        if (i < text.length) {
            element.innerText += text.charAt(i);
            i++;
            // Scroll container
            const container = element.parentElement;
            container.scrollTop = container.scrollHeight;
        } else {
            clearInterval(element.typewriterInterval);
        }
    }, 15); // Fast typing speed
}

function setupEventListeners() {
    // Camera toggles
    const btnGhost = document.getElementById("btn-cam-ghost");
    const btnAction = document.getElementById("btn-cam-action");

    btnGhost.addEventListener("click", () => {
        cameraMode = "ghost";
        btnGhost.classList.add("active");
        btnAction.classList.remove("active");
    });

    btnAction.addEventListener("click", () => {
        cameraMode = "action";
        btnAction.classList.add("active");
        btnGhost.classList.remove("active");
    });

    // Tracking dropdown selection
    const selectFocus = document.getElementById("focus-select");
    selectFocus.addEventListener("change", (e) => {
        trackingTargetId = e.target.value;
        if (cameraMode !== "action") {
            // Automatically switch to Action Cam when select is modified
            cameraMode = "action";
            btnAction.classList.add("active");
            btnGhost.classList.remove("active");
        }
    });

    // Bloom Toggle for Performance
    const toggleBloom = document.getElementById("toggle-bloom");
    toggleBloom.addEventListener("change", (e) => {
        useBloom = e.target.checked;
    });

    // Reboot commands
    const rebootBtn = document.getElementById("btn-reboot");
    const auditRebootBtn = document.getElementById("btn-audit-reboot");
    
    const sendReboot = () => {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "reboot_grid" }));
        }
    };
    rebootBtn.addEventListener("click", sendReboot);
    auditRebootBtn.addEventListener("click", sendReboot);

    // Override manual strategy buttons
    document.querySelectorAll(".btn-override").forEach(btn => {
        btn.addEventListener("click", (e) => {
            const team = e.target.getAttribute("data-team");
            const strat = e.target.getAttribute("data-strat");
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    type: "apply_override_strategy",
                    team: team,
                    strategy: strat
                }));
            }
        });
    });

    // Keyboard capture for Ghost Camera flight
    window.addEventListener("keydown", (e) => {
        if (['KeyW', 'KeyS', 'KeyA', 'KeyD', 'Space', 'ShiftLeft'].includes(e.code)) {
            keyStates[e.code] = true;
        }
    });

    window.addEventListener("keyup", (e) => {
        if (['KeyW', 'KeyS', 'KeyA', 'KeyD', 'Space', 'ShiftLeft'].includes(e.code)) {
            keyStates[e.code] = false;
        }
    });
}

function jsonParseSafe(str) {
    try {
        return JSON.parse(str);
    } catch (e) {
        return null;
    }
}
