// game_three.js: Static and dynamic map environment and mesh generators.

// 2. ENVIRONMENT CREATION (from WebSocket Map Layout packet)
function buildMapEnvironment(layout) {
    // Update map name display in DOM and window title
    const mapNameEl = document.getElementById("map-name-display");
    if (layout.name) {
        if (mapNameEl) {
            mapNameEl.innerText = `MAP: ${layout.name.toUpperCase()}`;
        }
        document.title = `Void Grid - Map: ${layout.name}`;
    }

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

    // Vibrant neon translucent fills — ghostly holographic volume, matching Tron aesthetic
    meshCache.matBlue = new THREE.MeshBasicMaterial({ color: TEAM_COLORS.blue, transparent: true, opacity: 0.18, depthWrite: false });
    meshCache.matOrange = new THREE.MeshBasicMaterial({ color: TEAM_COLORS.orange, transparent: true, opacity: 0.18, depthWrite: false });
    meshCache.matNeutral = new THREE.MeshBasicMaterial({ color: TEAM_COLORS.neutral, transparent: true, opacity: 0.18, depthWrite: false });

    function addPlatform(x, y, w, d, z, team) {
        const heightThickness = 0.8;
        const geom = new THREE.BoxGeometry(w, heightThickness, d);
        const sideMat = team === "blue" ? meshCache.matBlue : (team === "orange" ? meshCache.matOrange : meshCache.matNeutral);
        const mesh = new THREE.Mesh(geom, sideMat);
        mesh.position.set(x + w/2, z - heightThickness/2, y + d/2);
        scene.add(mesh);
        meshCache.mapElements.push(mesh);

        // Blazing neon edges (double-layered for glow and thickness)
        const edges = new THREE.EdgesGeometry(geom);
        const colorVal = TEAM_COLORS[team];
        const glowColor = new THREE.Color(colorVal).multiplyScalar(10.0);
        const lineMat = new THREE.LineBasicMaterial({ color: glowColor, linewidth: 3 });
        
        const line1 = new THREE.LineSegments(edges, lineMat);
        line1.position.copy(mesh.position);
        line1.team = team;
        scene.add(line1);
        meshCache.mapElements.push(line1);

        const line2 = new THREE.LineSegments(edges, lineMat);
        line2.position.copy(mesh.position);
        line2.scale.set(1.002, 1.01, 1.002);
        line2.team = team;
        scene.add(line2);
        meshCache.mapElements.push(line2);
    }

    function addBuilding(x, y, w, d, z, h, team) {
        const geom = new THREE.BoxGeometry(w, h, d);
        const bldMat = team === "blue" ? meshCache.matBlue : (team === "orange" ? meshCache.matOrange : meshCache.matNeutral);
        const mesh = new THREE.Mesh(geom, bldMat);
        mesh.position.set(x + w/2, z + h/2, y + d/2);
        scene.add(mesh);
        meshCache.mapElements.push(mesh);

        // Blazing neon outline (double-layered for thickness)
        const edges = new THREE.EdgesGeometry(geom);
        const colorVal = TEAM_COLORS[team];
        const glowColor = new THREE.Color(colorVal).multiplyScalar(10.0);
        const lineMat = new THREE.LineBasicMaterial({ color: glowColor, linewidth: 2.5 });
        
        const line1 = new THREE.LineSegments(edges, lineMat);
        line1.position.copy(mesh.position);
        line1.team = team;
        scene.add(line1);
        meshCache.mapElements.push(line1);

        const line2 = new THREE.LineSegments(edges, lineMat);
        line2.position.copy(mesh.position);
        line2.scale.set(1.002, 1.002, 1.002);
        line2.team = team;
        scene.add(line2);
        meshCache.mapElements.push(line2);
    }

    function addRamp(x1, x2, y1, y2, z1, z2, team) {
        const w = x2 - x1; // Horizontal span (X)
        const d = y2 - y1; // Depth (Z)
        const h = 0.5;     // Thickness of the ramp surface slab
        const slopeLength = Math.sqrt(w * w + (z2 - z1) * (z2 - z1));
        const cx = (x1 + x2) / 2.0;
        const cy = (z1 + z2) / 2.0;
        const cz = (y1 + y2) / 2.0;

        const geom = new THREE.BoxGeometry(slopeLength, h, d);
        const rampMat = team === "blue" ? meshCache.matBlue : meshCache.matOrange;
        const mesh = new THREE.Mesh(geom, rampMat);
        mesh.position.set(cx, cy, cz);
        const angle = Math.atan2(z2 - z1, x2 - x1);
        mesh.rotation.z = angle;
        scene.add(mesh);
        meshCache.mapElements.push(mesh);
        
        // Blazing inclined neon borders (double-layered for thickness)
        const edges = new THREE.EdgesGeometry(geom);
        const colorVal = TEAM_COLORS[team];
        const glowColor = new THREE.Color(colorVal).multiplyScalar(10.0);
        const lineMat = new THREE.LineBasicMaterial({ color: glowColor, linewidth: 3 });
        
        const line1 = new THREE.LineSegments(edges, lineMat);
        line1.position.copy(mesh.position);
        line1.rotation.z = angle;
        line1.team = team;
        scene.add(line1);
        meshCache.mapElements.push(line1);

        const line2 = new THREE.LineSegments(edges, lineMat);
        line2.position.copy(mesh.position);
        line2.rotation.z = angle;
        line2.scale.set(1.002, 1.01, 1.002);
        line2.team = team;
        scene.add(line2);
        meshCache.mapElements.push(line2);
    }

    // 1. Build elevated platforms
    layout.platforms.forEach(platform => {
        const { x, y, w, d, z } = platform;
        // If the platform crosses the center (x < 0 && x + w > 0), split it
        if (x < 0 && x + w > 0) {
            addPlatform(x, y, -x, d, z, "blue");
            addPlatform(0, y, x + w, d, z, "orange");
        } else {
            const team = (x + w/2 < 0) ? "blue" : "orange";
            addPlatform(x, y, w, d, z, team);
        }
    });

    // 2. Build non-walkable solid columns / walls (Buildings)
    layout.buildings.forEach(building => {
        const { x, y, w, d, z, h } = building;
        // If the building crosses the center (x < 0 && x + w > 0), split it
        if (x < 0 && x + w > 0) {
            addBuilding(x, y, -x, d, z, h, "blue");
            addBuilding(0, y, x + w, d, z, h, "orange");
        } else {
            let team = "neutral";
            if (x + w/2 < -30) team = "blue";
            else if (x + w/2 > 30) team = "orange";
            addBuilding(x, y, w, d, z, h, team);
        }
    });

    // 2.5 Build walkable ramps (inclined slabs connecting height layers)
    if (layout.ramps) {
        layout.ramps.forEach(ramp => {
            const { x1, x2, y1, y2, z1, z2 } = ramp;
            const cx = (x1 + x2) / 2.0;
            const team = cx < 0 ? "blue" : "orange";
            addRamp(x1, x2, y1, y2, z1, z2, team);
        });
    }

    // 3. Build Flag bases (Pedestals)
    createFlagPedestal("blue", pyToThreeVec(layout.bases.blue.pos));
    createFlagPedestal("orange", pyToThreeVec(layout.bases.orange.pos));
}

function createFlagPedestal(team, pos) {
    const colorVal = TEAM_COLORS[team];

    // Translucent cylinder fill to match platforms and pillars
    const geom = new THREE.CylinderGeometry(5, 5, 1.2, 8);
    const mat = new THREE.MeshBasicMaterial({
        color: colorVal,
        transparent: true,
        opacity: 0.18,
        depthWrite: false
    });
    const mesh = new THREE.Mesh(geom, mat);
    mesh.position.copy(pos);
    mesh.position.y += 0.6;
    scene.add(mesh);

    // Blazing outer edges (double-layered for thickness and glow)
    const edges = new THREE.EdgesGeometry(geom);
    const glowColor = new THREE.Color(colorVal).multiplyScalar(10.0);
    const lineMat = new THREE.LineBasicMaterial({ color: glowColor, linewidth: 3 });

    const line1 = new THREE.LineSegments(edges, lineMat);
    mesh.add(line1);

    const line2 = new THREE.LineSegments(edges, lineMat);
    line2.scale.set(1.002, 1.01, 1.002);
    mesh.add(line2);

    // Create the Flag itself (stored in cache)
    const flagGroup = new THREE.Group();
    flagGroup.position.copy(pos);
    flagGroup.position.y += 2.0; // Float above pedestal

    // Staff — very dark, well below bloom threshold (0.55 luminance)
    const staffGeom = new THREE.CylinderGeometry(0.12, 0.12, 5, 6);
    const staffMat = new THREE.MeshBasicMaterial({ color: 0x1a1a1a });
    const staff = new THREE.Mesh(staffGeom, staffMat);
    flagGroup.add(staff);

    // Tip (octahedron) — the ONLY bloomable part
    const tipGeom = new THREE.OctahedronGeometry(1.2, 0);
    const tipMat = new THREE.MeshStandardMaterial({
        color: colorVal,
        emissive: new THREE.Color(colorVal).multiplyScalar(3.0),
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

function onWindowResize() {
    const container = document.getElementById("canvas-container");
    const w = container.clientWidth;
    const h = container.clientHeight;
    
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    
    renderer.setSize(w, h);
    composer.setSize(w, h);
}
