// game.js: Playback animation loop, interpolation calculation, and event listeners entry point.

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
        overcharge_node: stateA.overcharge_node,
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
    const dt = Math.min((now - lastFrameTime) / 1000.0, 0.05); // cap at 50ms
    lastFrameTime = now;

    // Calculate and display FPS
    fpsFrames++;
    if (now >= fpsLastTime + 300) {
        const fps = Math.round((fpsFrames * 1000) / (now - fpsLastTime));
        const fpsEl = document.getElementById("fps-counter");
        if (fpsEl) {
            fpsEl.innerText = `FPS: ${fps}`;
            if (fps >= 55) {
                fpsEl.style.color = "#39ff14";
            } else if (fps >= 30) {
                fpsEl.style.color = "#a0aec0";
            } else {
                fpsEl.style.color = "#ef4444";
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
            const framesAhead = stateBuffer.filter(s => s.sim_time > clientRenderTime).length;
            
            let targetTimeScale = 1.0;
            if (framesAhead === 0) {
                targetTimeScale = 0.0;
            } else if (framesAhead === 1) {
                targetTimeScale = 0.4;
            } else if (framesAhead >= 2 && framesAhead <= 6) {
                targetTimeScale = 1.0;
            } else if (framesAhead >= 7 && framesAhead <= 9) {
                targetTimeScale = 1.15;
            } else {
                targetTimeScale = 1.4;
            }

            clientTimeScale = clientTimeScale * 0.6 + targetTimeScale * 0.4;
            clientRenderTime += dt * clientTimeScale;
        }

        const maxClampTime = Math.max(earliestSimTime, latestSimTime - 0.001);
        clientRenderTime = Math.max(earliestSimTime, Math.min(maxClampTime, clientRenderTime));

        let stateA = null;
        let stateB = null;

        for (let i = 0; i < stateBuffer.length - 1; i++) {
            if (stateBuffer[i].sim_time <= clientRenderTime && stateBuffer[i + 1].sim_time > clientRenderTime) {
                stateA = stateBuffer[i];
                stateB = stateBuffer[i + 1];
                break;
            }
        }

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

            const interpolatedState = interpolateState(stateA, stateB, t);
            currentGameState = interpolatedState;

            processStateUpdate(interpolatedState);
        }
    }

    for (const id in meshCache.projectiles) {
        const projObj = meshCache.projectiles[id];
        projObj.mesh.rotation.z += 0.25;
    }

    updateParticles(dt);

    if (gridBlue && gridOrange) {
        let matchTime = 180.0;
        if (currentGameState && currentGameState.match_time !== undefined) {
            matchTime = Math.max(0.0, currentGameState.match_time);
        }
        const maxTime = 180.0;
        const timeRatio = 1.0 - (matchTime / maxTime);
        const pulseFreq = 1.5 + timeRatio * 8.5;
        const time = Date.now() * 0.001;
        const pulseVal = Math.sin(time * pulseFreq * 2.0 * Math.PI);
        const opacity = 0.15 + (pulseVal + 1.0) * 0.5 * 0.20;
        gridBlue.material.opacity = opacity;
        gridOrange.material.opacity = opacity;
    }

    updateCamera(dt);

    if (useBloom) {
        composer.render();
    } else {
        renderer.render(scene, camera);
    }
}

function setupEventListeners() {
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

    const selectFocus = document.getElementById("focus-select");
    selectFocus.addEventListener("change", (e) => {
        trackingTargetId = e.target.value;
        if (cameraMode !== "action") {
            cameraMode = "action";
            btnAction.classList.add("active");
            btnGhost.classList.remove("active");
        }
    });

    const toggleBloom = document.getElementById("toggle-bloom");
    const bloomIntensityContainer = document.getElementById("bloom-intensity-container");
    const bloomIntensityInput = document.getElementById("bloom-intensity");
    const bloomValSpan = document.getElementById("bloom-val");

    const updateBloomUI = () => {
        if (!bloomIntensityContainer || !bloomIntensityInput) return;
        if (useBloom) {
            bloomIntensityContainer.style.opacity = "1";
            bloomIntensityInput.disabled = false;
        } else {
            bloomIntensityContainer.style.opacity = "0.4";
            bloomIntensityInput.disabled = true;
        }
    };

    if (toggleBloom) {
        toggleBloom.addEventListener("change", (e) => {
            useBloom = e.target.checked;
            updateBloomUI();
        });
    }

    if (bloomIntensityInput) {
        bloomIntensityInput.addEventListener("input", (e) => {
            const val = parseFloat(e.target.value);
            if (bloomValSpan) {
                bloomValSpan.textContent = val.toFixed(1);
            }
            if (bloomPass) {
                bloomPass.strength = val;
            }
        });
    }

    updateBloomUI();

    const rebootBtn = document.getElementById("btn-reboot");
    const auditRebootBtn = document.getElementById("btn-audit-reboot");
    const pauseBtn = document.getElementById("btn-pause");
    
    const sendReboot = () => {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "reboot_grid" }));
        }
    };
    if (rebootBtn) {
        rebootBtn.addEventListener("click", sendReboot);
    }
    if (auditRebootBtn) {
        auditRebootBtn.addEventListener("click", sendReboot);
    }
    if (pauseBtn) {
        pauseBtn.addEventListener("click", () => {
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({ type: "toggle_pause" }));
            }
        });
    }

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

    const bracketToggleBtn = document.getElementById("bracket-toggle");
    const bracketPanel = document.getElementById("tournament-bracket-panel");
    if (bracketToggleBtn && bracketPanel) {
        bracketToggleBtn.addEventListener("click", () => {
            bracketPanel.classList.toggle("expanded");
        });
    }

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
