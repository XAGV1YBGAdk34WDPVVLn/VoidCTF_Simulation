// game_render_player.js: Real-time renderer updates for player meshes, healing beams, trails, and death particles.

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
            color: p.team === "blue" ? 0x004466 : 0x661100, // Vibrant but deep contrast color
            emissive: teamColor,
            emissiveIntensity: 0.6, // Richer emissive glow for Tron style
            roughness: 0.15,
            metalness: 0.85
        });
        const bodyMesh = new THREE.Mesh(geom, mat);
        bodyMesh.position.y = heightOffset;
        bodyMesh.castShadow = true;
        group.add(bodyMesh);

        // Add Tron-like neon edge outline for crisp 3D definition (double-layered for glow and thickness)
        const edges = new THREE.EdgesGeometry(geom);
        const outlineGlowColor = new THREE.Color(teamColor).multiplyScalar(8.0);
        const lineMat = new THREE.LineBasicMaterial({ color: outlineGlowColor, linewidth: 2.5 });
        
        const edgeLine1 = new THREE.LineSegments(edges, lineMat);
        bodyMesh.add(edgeLine1);

        const edgeLine2 = new THREE.LineSegments(edges, lineMat);
        edgeLine2.scale.set(1.02, 1.02, 1.02);
        bodyMesh.add(edgeLine2);

        // Add Frontal Hardlight Shield (for Enforcer)
        const shieldGeom = new THREE.SphereGeometry(2.5, 16, 16, 0, Math.PI);
        const shieldGlowColor = new THREE.Color(teamColor).multiplyScalar(2.5);
        const shieldMat = new THREE.MeshBasicMaterial({
            color: shieldGlowColor,
            transparent: true,
            opacity: 0.35,
            wireframe: true,
            side: THREE.DoubleSide
        });
        const shieldMesh = new THREE.Mesh(shieldGeom, shieldMat);
        shieldMesh.position.y = heightOffset;
        shieldMesh.position.z = 1.0;
        shieldMesh.rotation.y = Math.PI / 2;
        shieldMesh.visible = false;
        group.add(shieldMesh);

        // 3D Linked Orbit Shield (Circular Buzzsaw)
        const orbitShieldGroup = new THREE.Group();
        orbitShieldGroup.position.y = heightOffset;
        
        // Circular ring track
        const trackGeom = new THREE.TorusGeometry(3.0, 0.04, 4, 32);
        const trackMat = new THREE.MeshBasicMaterial({
            color: teamColor,
            transparent: true,
            opacity: 0.25
        });
        const trackMesh = new THREE.Mesh(trackGeom, trackMat);
        trackMesh.rotation.x = Math.PI / 2;
        orbitShieldGroup.add(trackMesh);

        // Blade 1
        const blade1Geom = new THREE.TorusGeometry(0.5, 0.08, 6, 16);
        const blade1Mat = new THREE.MeshBasicMaterial({ color: teamColor });
        const blade1Mesh = new THREE.Mesh(blade1Geom, blade1Mat);
        blade1Mesh.position.set(3.0, 0.0, 0.0);
        blade1Mesh.rotation.x = Math.PI / 2;
        orbitShieldGroup.add(blade1Mesh);

        // Blade 2
        const blade2Geom = new THREE.TorusGeometry(0.5, 0.08, 6, 16);
        const blade2Mat = new THREE.MeshBasicMaterial({ color: teamColor });
        const blade2Mesh = new THREE.Mesh(blade2Geom, blade2Mat);
        blade2Mesh.position.set(-3.0, 0.0, 0.0);
        blade2Mesh.rotation.x = Math.PI / 2;
        orbitShieldGroup.add(blade2Mesh);

        orbitShieldGroup.visible = false;
        group.add(orbitShieldGroup);
        
        // Light Trail (Rendered as a 3D wall of light with volume and thickness)
        const trailMaxPoints = 30;
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
            orbitShieldGroup: orbitShieldGroup,
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

    const { group, bodyMesh, shieldMesh, orbitShieldGroup, trailLine, trailPoints, lastPos } = playerObj;

    // Dynamically update materials color if team color changes (e.g. during tournament matches)
    if (bodyMesh.material) {
        bodyMesh.material.emissive.setHex(teamColor);
        bodyMesh.material.color.copy(new THREE.Color(teamColor).multiplyScalar(0.25));
    }
    const outlineColor = new THREE.Color(teamColor).multiplyScalar(8.0);
    bodyMesh.children.forEach(child => {
        if (child instanceof THREE.LineSegments) {
            child.material.color.copy(outlineColor);
        }
    });
    if (shieldMesh && shieldMesh.material) {
        shieldMesh.material.color.copy(new THREE.Color(teamColor).multiplyScalar(2.5));
    }
    if (orbitShieldGroup) {
        const trackMesh = orbitShieldGroup.children[0];
        const blade1 = orbitShieldGroup.children[1];
        const blade2 = orbitShieldGroup.children[2];
        if (trackMesh && trackMesh.material) trackMesh.material.color.setHex(teamColor);
        if (blade1 && blade1.material) blade1.material.color.setHex(teamColor);
        if (blade2 && blade2.material) blade2.material.color.setHex(teamColor);
    }

    const targetPos = pyToThreeVec(p.pos);
    
    // Smoothly glide position to absorb any remaining playhead or clock jitter.
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
        group.quaternion.slerp(targetRotation, 0.22); // Fast, responsive slerp
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

    // Orbit Shield (Linked Buzzsaw)
    if (orbitShieldGroup) {
        orbitShieldGroup.visible = !!playerObj.isLinked;
        if (playerObj.isLinked) {
            orbitShieldGroup.rotation.y = (Date.now() * 0.012) % (Math.PI * 2);
            orbitShieldGroup.children[1].rotation.z = (Date.now() * 0.035) % (Math.PI * 2);
            orbitShieldGroup.children[2].rotation.z = (Date.now() * 0.035) % (Math.PI * 2);
        }
    }

    // 3. Tactician healing beam
    updateHealingBeam(p);

    // Bobbing/Pose animations for Champion Celebration
    const isCelebrating = currentGameState && currentGameState.state === "CHAMPION_CELEBRATION";
    const heightOffset = (p.class_type === "Stalker") ? 1.6 : ((p.class_type === "Enforcer") ? 1.6 : 1.8);
    if (isCelebrating) {
        const time = Date.now() * 0.002;
        if (p.class_type === "Stalker") {
            bodyMesh.rotation.y = time * 4.5;
            bodyMesh.position.y = heightOffset + Math.sin(time * 3.5) * 0.6;
        } else if (p.class_type === "Tactician") {
            bodyMesh.rotation.z = time * 1.5;
            bodyMesh.rotation.y = time * 0.8;
            bodyMesh.position.y = heightOffset + Math.cos(time * 2.5) * 0.4;
        } else if (p.class_type === "Enforcer") {
            bodyMesh.rotation.y = time * 0.3;
            bodyMesh.position.y = heightOffset + Math.sin(time * 1.0) * 0.08;
        }
        bodyMesh.material.emissiveIntensity = 1.8 + Math.sin(time * 4) * 0.3;
    } else {
        bodyMesh.rotation.set(0, 0, 0);
        bodyMesh.position.y = heightOffset;
    }

    // 4. Update Fading Light Trail
    if (!lastPos.equals(group.position)) {
        trailPoints.push(group.position.clone());
        if (trailPoints.length > 29) trailPoints.shift();
        lastPos.copy(group.position);
    }
    
    const positions = trailLine.geometry.attributes.position.array;
    const colors = trailLine.geometry.attributes.color.array;
    const pointsCount = trailPoints.length;
    const c = new THREE.Color(teamColor);
    for (let i = 0; i < 30; i++) {
        const pt = trailPoints[Math.min(i, pointsCount - 1)] || group.position;
        
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
            nx = -dz / len;
            nz = dx / len;
        } else if (p.vel) {
            let v_len = Math.sqrt(p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]);
            if (v_len > 0.001) {
                nx = -p.vel[1] / v_len;
                nz = p.vel[0] / v_len;
            }
        }
        
        const width = 0.22;
        const baseIdx = 12 * i;
        
        positions[baseIdx] = pt.x + nx * width;
        positions[baseIdx + 1] = pt.y - 0.1;
        positions[baseIdx + 2] = pt.z + nz * width;
        
        positions[baseIdx + 3] = pt.x - nx * width;
        positions[baseIdx + 4] = pt.y - 0.1;
        positions[baseIdx + 5] = pt.z - nz * width;
        
        positions[baseIdx + 6] = pt.x + nx * width;
        positions[baseIdx + 7] = pt.y + 1.0;
        positions[baseIdx + 8] = pt.z + nz * width;
        
        positions[baseIdx + 9] = pt.x - nx * width;
        positions[baseIdx + 10] = pt.y + 1.0;
        positions[baseIdx + 11] = pt.z - nz * width;
        
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
    const isCelebrating = currentGameState && currentGameState.state === "CHAMPION_CELEBRATION";
    
    let targetPlayer = null;
    let pTarget = null;
    
    if (isCelebrating) {
        if (healer.class_type === "Tactician") {
            const enforcer = Object.values(currentGameState.players).find(p => p.class_type === "Enforcer" && p.team === healer.team);
            if (enforcer) {
                targetPlayer = enforcer;
                pTarget = new THREE.Vector3(0, 1.5, 0);
            }
        }
    } else {
        if (healer.is_healing && healer.healing_target_id !== null) {
            targetPlayer = currentGameState.players[healer.healing_target_id];
            if (targetPlayer && targetPlayer.is_alive) {
                pTarget = pyToThreeVec(targetPlayer.pos).add(new THREE.Vector3(0, 1.5, 0));
            }
        }
    }
    
    if (!targetPlayer || !pTarget) {
        if (beam) {
            scene.remove(beam);
            delete meshCache.healingBeams[healer.id];
        }
        return;
    }

    const pHealer = pyToThreeVec(healer.pos).add(new THREE.Vector3(0, 1.5, 0));
    const healerColor = TEAM_COLORS[healer.team];

    if (!beam) {
        const geom = new THREE.BufferGeometry().setFromPoints([pHealer, pTarget]);
        const mat = new THREE.LineBasicMaterial({
            color: healerColor,
            linewidth: 3,
            transparent: true,
            opacity: 0.8
        });
        beam = new THREE.Line(geom, mat);
        scene.add(beam);
        meshCache.healingBeams[healer.id] = beam;
    } else {
        const positions = beam.geometry.attributes.position.array;
        positions[0] = pHealer.x;
        positions[1] = pHealer.y;
        positions[2] = pHealer.z;
        positions[3] = pTarget.x;
        positions[4] = pTarget.y;
        positions[5] = pTarget.z;
        beam.geometry.attributes.position.needsUpdate = true;
        if (beam.material) {
            beam.material.color.setHex(healerColor);
        }
    }
}

function spawnDeRezExplosion(pos, color, count = 25) {
    const geom = new THREE.BufferGeometry();
    const positions = [];
    const velocities = [];
    
    for (let i = 0; i < count; i++) {
        positions.push(pos.x, pos.y, pos.z);
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
        
        for (let j = 0; j < positions.length / 3; j++) {
            positions[j * 3] += p.velocities[j * 3] * dt;
            positions[j * 3 + 1] += p.velocities[j * 3 + 1] * dt;
            positions[j * 3 + 2] += p.velocities[j * 3 + 2] * dt;
        }
        p.points.geometry.attributes.position.needsUpdate = true;
    }
}

function cleanupExpiredEntities(gameState) {
    Object.keys(meshCache.players).forEach(pid => {
        if (!gameState.players[pid]) {
            scene.remove(meshCache.players[pid].group);
            scene.remove(meshCache.players[pid].trailLine);
            delete meshCache.players[pid];
        }
    });
}
