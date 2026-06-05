// game_render_world.js: Telemetry dispatcher and updates for flags, overcharge node, projectiles, and action cam tracking.

function processStateUpdate(gameState) {
    // Update Players (Positions, Health, State)
    const serverTime = (gameState.sim_time !== undefined ? gameState.sim_time : 0.0) * 1000.0;
    Object.values(gameState.players).forEach(playerData => {
        updatePlayerMesh(playerData, serverTime);
    });

    // Update Buzzsaw Tethers (Laser Link Bridges between teammates)
    updateBuzzsawTethers(gameState);

    // Update Projectiles (Light Discs)
    updateProjectiles(gameState.projectiles, serverTime);

    // Update Flags
    updateFlags(gameState.flags);

    // Update Midfield Overcharge Node
    updateOverchargeNode(gameState.overcharge_node);

    // Cleanup disconnected or de-rezzed items
    cleanupExpiredEntities(gameState);
}

function updateProjectiles(projectiles, serverTime) {
    projectiles.forEach(proj => {
        let projObj = meshCache.projectiles[proj.id];
        const targetPos = pyToThreeVec(proj.pos);
        
        if (!projObj) {
            const group = new THREE.Group();
            
            const outerGeom = new THREE.TorusGeometry(0.8, 0.08, 8, 32);
            const outerMat = new THREE.MeshBasicMaterial({ color: TEAM_COLORS[proj.team] });
            const outerMesh = new THREE.Mesh(outerGeom, outerMat);
            group.add(outerMesh);
            
            const innerGeom = new THREE.RingGeometry(0.15, 0.72, 32);
            const innerMat = new THREE.MeshBasicMaterial({
                color: 0xffffff,
                transparent: true,
                opacity: 0.65,
                side: THREE.DoubleSide
            });
            const innerMesh = new THREE.Mesh(innerGeom, innerMat);
            group.add(innerMesh);
            
            const hubGeom = new THREE.TorusGeometry(0.18, 0.04, 8, 16);
            const hubMat = new THREE.MeshBasicMaterial({ color: TEAM_COLORS[proj.team] });
            const hubMesh = new THREE.Mesh(hubGeom, hubMat);
            group.add(hubMesh);
            
            group.position.copy(targetPos);
            group.rotation.x = Math.PI / 2;
            scene.add(group);
            
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
                
                trailIndices.push(bl, bl_next, tl);
                trailIndices.push(tl, bl_next, tl_next);
                
                trailIndices.push(br_next, br, tr_next);
                trailIndices.push(tr_next, br, tr);
                
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

        projObj.mesh.position.copy(targetPos);

        if (!projObj.lastPos.equals(targetPos)) {
            projObj.trailPoints.push(targetPos.clone());
            if (projObj.trailPoints.length > 12) {
                projObj.trailPoints.shift();
            }
            projObj.lastPos.copy(targetPos);
        }

        const trailPositions = projObj.trailMesh.geometry.attributes.position.array;
        const trailColors = projObj.trailMesh.geometry.attributes.color.array;
        const pointsCount = projObj.trailPoints.length;
        const color = new THREE.Color(TEAM_COLORS[proj.team]);
        const trailMaxPoints = 12;

        for (let i = 0; i < trailMaxPoints; i++) {
            const pt = projObj.trailPoints[Math.min(i, pointsCount - 1)] || targetPos;
            
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
            
            const width = 0.35;
            const heightHalf = 0.03;
            const baseIdx = 12 * i;
            
            trailPositions[baseIdx] = pt.x + nx * width;
            trailPositions[baseIdx + 1] = pt.y - heightHalf;
            trailPositions[baseIdx + 2] = pt.z + nz * width;
            
            trailPositions[baseIdx + 3] = pt.x - nx * width;
            trailPositions[baseIdx + 4] = pt.y - heightHalf;
            trailPositions[baseIdx + 5] = pt.z - nz * width;
            
            trailPositions[baseIdx + 6] = pt.x + nx * width;
            trailPositions[baseIdx + 7] = pt.y + heightHalf;
            trailPositions[baseIdx + 8] = pt.z + nz * width;
            
            trailPositions[baseIdx + 9] = pt.x - nx * width;
            trailPositions[baseIdx + 10] = pt.y + heightHalf;
            trailPositions[baseIdx + 11] = pt.z - nz * width;
            
            const alpha = i / (trailMaxPoints - 1);
            for (let v = 0; v < 4; v++) {
                const colorIdx = 16 * i + 4 * v;
                trailColors[colorIdx] = color.r;
                trailColors[colorIdx + 1] = color.g;
                trailColors[colorIdx + 2] = color.b;
                trailColors[colorIdx + 3] = alpha * 0.75;
            }
        }
        projObj.trailMesh.geometry.attributes.position.needsUpdate = true;
        projObj.trailMesh.geometry.attributes.color.needsUpdate = true;

        projObj.targetPos = targetPos;
        projObj.vel = proj.vel;
        projObj.updated = true;
    });

    Object.keys(meshCache.projectiles).forEach(id => {
        const match = projectiles.find(p => p.id == id);
        if (!match) {
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

function updateOverchargeNode(nodeData) {
    if (!nodeData) return;
    if (nodeData.active) {
        if (!meshCache.overchargeNode) {
            const group = new THREE.Group();
            const nodePos = pyToThreeVec(nodeData.pos);
            group.position.copy(nodePos);
            
            const outerGeo = new THREE.IcosahedronGeometry(2.0, 1);
            const outerMat = new THREE.MeshBasicMaterial({
                color: new THREE.Color(0x9f00ff).multiplyScalar(3.0),
                wireframe: true,
                transparent: true,
                opacity: 0.8
            });
            const outerMesh = new THREE.Mesh(outerGeo, outerMat);
            group.add(outerMesh);
            
            const innerGeo = new THREE.SphereGeometry(0.8, 16, 16);
            const innerMat = new THREE.MeshStandardMaterial({
                color: 0xffffff,
                emissive: new THREE.Color(0x9f00ff).multiplyScalar(2.0),
                emissiveIntensity: 2.0,
                roughness: 0.1,
                metalness: 0.1
            });
            const innerMesh = new THREE.Mesh(innerGeo, innerMat);
            group.add(innerMesh);
            
            const beamGeo = new THREE.CylinderGeometry(0.5, 0.5, 150, 16, 1, true);
            const beamMat = new THREE.MeshBasicMaterial({
                color: new THREE.Color(0x9f00ff).multiplyScalar(4.0),
                transparent: true,
                opacity: 0.3,
                blending: THREE.AdditiveBlending,
                side: THREE.DoubleSide,
                depthWrite: false
            });
            const beamMesh = new THREE.Mesh(beamGeo, beamMat);
            beamMesh.position.y = 75;
            group.add(beamMesh);
            
            scene.add(group);
            meshCache.overchargeNode = group;
        }
        meshCache.overchargeNode.visible = true;
        
        const time = Date.now() * 0.003;
        
        meshCache.overchargeNode.rotation.set(0, 0, 0);
        
        const outer = meshCache.overchargeNode.children[0];
        const inner = meshCache.overchargeNode.children[1];
        const beam = meshCache.overchargeNode.children[2];
        
        if (outer) {
            outer.rotation.y = time * 0.5;
            outer.rotation.x = time * 0.3;
        }
        if (inner) {
            inner.rotation.y = -time * 0.4;
            inner.rotation.z = time * 0.2;
        }
        
        if (beam) {
            beam.rotation.y = -time * 0.2;
            const beamPulse = 1.0 + Math.sin(time * 6.0) * 0.08;
            beam.scale.set(beamPulse, 1.0, beamPulse);
            beam.material.opacity = 0.22 + Math.sin(time * 15.0) * 0.04;
        }
        
        const baseZ = nodeData.pos[2];
        meshCache.overchargeNode.position.y = baseZ + 1.5 + Math.sin(time * 2.0) * 0.4;
    } else {
        if (meshCache.overchargeNode) {
            meshCache.overchargeNode.visible = false;
        }
    }
}

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
        
        if (flagObj.group.position.distanceTo(finalTarget) > 15.0) {
            flagObj.group.position.copy(finalTarget);
        } else {
            flagObj.group.position.lerp(finalTarget, 0.25);
        }
        
        flagObj.coreMesh.rotation.y += 0.02;
        flagObj.coreMesh.rotation.x = Math.sin(time * 0.5) * 0.2;
        
        if (data.carrier_id !== null) {
            flagObj.coreMesh.material.emissiveIntensity = 2.5;
            const scale = 1.0 + Math.sin(time * 5) * 0.15;
            flagObj.coreMesh.scale.set(scale, scale, scale);
        } else {
            flagObj.coreMesh.material.emissiveIntensity = 1.2;
            flagObj.coreMesh.scale.set(1, 1, 1);
        }
    }
}

function updateCamera(dt) {
    if (cameraMode === "ghost" && currentGameState && currentGameState.state === "CHAMPION_CELEBRATION") {
        cameraMode = "action";
    }

    if (cameraMode === "ghost") {
        orbitControls.enabled = true;
        
        const moveSpeed = 60.0 * dt;
        const forward = new THREE.Vector3();
        camera.getWorldDirection(forward);
        forward.y = 0;
        forward.normalize();
        
        const right = new THREE.Vector3();
        right.crossVectors(forward, camera.up).normalize();

        if (keyStates['KeyW']) camera.position.addScaledVector(forward, moveSpeed);
        if (keyStates['KeyS']) camera.position.addScaledVector(forward, -moveSpeed);
        if (keyStates['KeyA']) camera.position.addScaledVector(right, -moveSpeed);
        if (keyStates['KeyD']) camera.position.addScaledVector(right, moveSpeed);
        if (keyStates['Space']) camera.position.y += moveSpeed;
        if (keyStates['ShiftLeft']) camera.position.y -= moveSpeed;
        
        orbitControls.target.addScaledVector(forward, (keyStates['KeyW']?moveSpeed:0) - (keyStates['KeyS']?moveSpeed:0));
        orbitControls.target.addScaledVector(right, (keyStates['KeyD']?moveSpeed:0) - (keyStates['KeyA']?moveSpeed:0));
        
        orbitControls.update();
    } else if (cameraMode === "action" && currentGameState) {
        orbitControls.enabled = false;
        
        if (currentGameState.state === "CHAMPION_CELEBRATION") {
            const time = Date.now() * 0.0006;
            const radius = 35.0;
            const camX = Math.sin(time) * radius;
            const camZ = Math.cos(time) * radius;
            const camY = 12.0 + Math.sin(time * 0.5) * 3.0;
            camera.position.set(camX, camY, camZ);
            camera.lookAt(new THREE.Vector3(0, 2.0, 0));
        } else {
            let lookTargetPos = new THREE.Vector3(0, 0, 0);
            let trackingPlayer = null;
            
            if (trackingTargetId === "auto") {
                let carrier = Object.values(currentGameState.players).find(p => p.has_flag && p.is_alive);
                if (carrier) {
                    trackingPlayer = carrier;
                } else {
                    let combatants = Object.values(currentGameState.players).filter(p => p.is_alive && (p.is_healing || p.is_dashing || p.is_shielding));
                    if (combatants.length > 0) {
                        trackingPlayer = combatants[0];
                    } else {
                        let alivePlayers = Object.values(currentGameState.players).filter(p => p.is_alive);
                        if (alivePlayers.length > 0) {
                            trackingPlayer = alivePlayers[0];
                        }
                    }
                }
            } else {
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
                
                const playerVel = new THREE.Vector3(trackingPlayer.vel[0], trackingPlayer.vel[2], trackingPlayer.vel[1]);
                const facingDir = new THREE.Vector3(0, 0, 1);
                if (playerVel.lengthSq() > 0.1) {
                    facingDir.copy(playerVel).normalize();
                } else {
                    facingDir.set(trackingPlayer.team === "blue" ? 1 : -1, 0, 0);
                }
                
                const distanceBehind = 35.0;
                const heightOffset = 18.0;
                
                const targetCamPos = lookTargetPos.clone()
                    .addScaledVector(facingDir, -distanceBehind)
                    .add(new THREE.Vector3(0, heightOffset, 0));
                    
                camera.position.lerp(targetCamPos, 0.08);
                
                if (!camera.userData.smoothedLookAt) camera.userData.smoothedLookAt = new THREE.Vector3();
                camera.userData.smoothedLookAt.lerp(lookTargetPos, 0.1);
                camera.lookAt(camera.userData.smoothedLookAt);
            } else {
                const centerTarget = new THREE.Vector3(0, 10, 0);
                camera.position.lerp(new THREE.Vector3(0, 70, 130), 0.05);
                camera.lookAt(centerTarget);
            }
        }
    }
}

function distToSegment(p, p1, p2) {
    const dx = p2[0] - p1[0];
    const dy = p2[1] - p1[1];
    const dz = p2[2] - p1[2];
    const lenSq = dx * dx + dy * dy + dz * dz;
    if (lenSq < 0.001) {
        return Math.hypot(p[0] - p1[0], p[1] - p1[1], p[2] - p1[2]);
    }
    
    let t = ((p[0] - p1[0]) * dx + (p[1] - p1[1]) * dy + (p[2] - p1[2]) * dz) / lenSq;
    t = Math.max(0.0, Math.min(1.0, t));
    
    const projX = p1[0] + t * dx;
    const projY = p1[1] + t * dy;
    const projZ = p1[2] + t * dz;
    
    return Math.hypot(p[0] - projX, p[1] - projY, p[2] - projZ);
}

function updateBuzzsawTethers(gameState) {
    // Reset link flags for all cached players first
    Object.values(meshCache.players).forEach(pObj => {
        pObj.isLinked = false;
    });

    meshCache.linkBeams = meshCache.linkBeams || {};
    const activeBeamIds = new Set();
    const alivePlayers = Object.values(gameState.players).filter(p => p.is_alive);

    // Check distances for teammates
    for (let i = 0; i < alivePlayers.length; i++) {
        for (let j = i + 1; j < alivePlayers.length; j++) {
            const pA = alivePlayers[i];
            const pB = alivePlayers[j];
            if (pA.team === pB.team) {
                const dist = Math.hypot(pA.pos[0] - pB.pos[0], pA.pos[1] - pB.pos[1]);
                if (dist <= 25.0) {
                    const pAObj = meshCache.players[pA.id];
                    const pBObj = meshCache.players[pB.id];
                    if (pAObj) pAObj.isLinked = true;
                    if (pBObj) pBObj.isLinked = true;

                    if (pAObj && pBObj) {
                        const id = `${pA.id}_${pB.id}`;
                        activeBeamIds.add(id);
                        updateLinkBeam(id, pAObj.group.position, pBObj.group.position, TEAM_COLORS[pA.team]);

                        // Check if any enemy player intersects this team's buzzsaw tether
                        const enemyTeam = pA.team === "blue" ? "orange" : "blue";
                        const enemies = alivePlayers.filter(p => p.team === enemyTeam);
                        enemies.forEach(enemy => {
                            const enemyDist = distToSegment(enemy.pos, pA.pos, pB.pos);
                            if (enemyDist <= 3.5) {
                                const enemyObj = meshCache.players[enemy.id];
                                if (enemyObj) {
                                    const sparkPos = enemyObj.group.position.clone();
                                    sparkPos.y += 1.0; // center of player height
                                    const sparkColors = [0xffaa00, 0xffdd00, 0xffffff];
                                    const c = sparkColors[Math.floor(Math.random() * sparkColors.length)];
                                    if (typeof spawnSparks === "function") {
                                        spawnSparks(sparkPos, c, 6);
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    }

    // Clean up expired beams
    Object.keys(meshCache.linkBeams).forEach(id => {
        if (!activeBeamIds.has(id)) {
            scene.remove(meshCache.linkBeams[id].mesh);
            delete meshCache.linkBeams[id];
        }
    });
}

function updateLinkBeam(id, posA, posB, teamColor) {
    let beamObj = meshCache.linkBeams[id];
    if (!beamObj) {
        // Create cylinder representing the buzzsaw bridge
        const geom = new THREE.CylinderGeometry(0.18, 0.18, 1.0, 6);
        const mat = new THREE.MeshBasicMaterial({
            color: teamColor,
            transparent: true,
            opacity: 0.5,
            wireframe: true
        });
        const mesh = new THREE.Mesh(geom, mat);
        scene.add(mesh);
        beamObj = { mesh: mesh };
        meshCache.linkBeams[id] = beamObj;
    }

    const mesh = beamObj.mesh;
    mesh.visible = true;

    const dir = new THREE.Vector3().subVectors(posB, posA);
    const length = dir.length();
    
    // Position at midpoint (elevated to align with players)
    const midpoint = new THREE.Vector3().addVectors(posA, posB).multiplyScalar(0.5);
    midpoint.y += 1.6;
    mesh.position.copy(midpoint);
    
    // Scale length along its Y-axis
    mesh.scale.set(1.0, length, 1.0);
    
    // Rotate cylinder to point from A to B
    const up = new THREE.Vector3(0, 1, 0);
    mesh.quaternion.setFromUnitVectors(up, dir.clone().normalize());

    // Rotate cylinder on its Y-axis to animate the buzzsaw energy field!
    mesh.rotation.y = (Date.now() * 0.01) % (Math.PI * 2);
}
