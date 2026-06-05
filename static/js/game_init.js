// game_init.js: Initialization of Three.js context, scene, renderer, basic layouts, and WS connection.

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
        1.8,  // Strength — cranked up for vivid Tron glow
        0.5,  // Radius
        0.30  // Threshold — lower so neon lines bloom hard
    );
    
    composer = new THREE.EffectComposer(renderer);
    composer.addPass(renderScene);
    composer.addPass(bloomPass);

    // Window Resize Event
    window.addEventListener("resize", onWindowResize);
    
    // Draw default elements before map loads
    drawBaseGrids();
}

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
            
            // Update team colors from tournament immediately
            updateTeamColorsFromTournament(payload.data.tournament);
            
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
