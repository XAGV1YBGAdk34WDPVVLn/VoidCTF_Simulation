// game_ui.js: DOM rendering, HUD panel updater, tactics summary, and tournament bracket visualization.

function updateDOMState(gameState) {
    const pregameOverlay = document.getElementById("pregame-overlay");
    const auditOverlay = document.getElementById("audit-overlay");
    const championOverlay = document.getElementById("champion-overlay");
    
    // Always update tactics overlay sidebar/strategies so they don't get stuck on LOADING... if we refresh mid-game
    if (gameState.tactics) {
        updateTacticsOverlay(gameState.tactics);
    }
    
    // Always update tournament bracket HUD component
    if (gameState.tournament) {
        updateTournamentBracket(gameState.tournament, gameState.state);
    }
    
    if (gameState.state === "CHAMPION_CELEBRATION") {
        if (!pregameOverlay.classList.contains("hidden")) pregameOverlay.classList.add("hidden");
        if (!auditOverlay.classList.contains("hidden")) auditOverlay.classList.add("hidden");
        
        const champTeamName = document.getElementById("champion-team-name");
        const countdownEl = document.getElementById("celebration-countdown");
        if (gameState.tournament && gameState.tournament.champion_index !== null) {
            const champion = gameState.tournament.teams[gameState.tournament.champion_index];
            if (champTeamName) {
                champTeamName.innerText = champion.name.toUpperCase();
                champTeamName.style.color = champion.primary_hex;
                champTeamName.style.textShadow = `0 0 10px #ffffff, 0 0 25px ${champion.primary_hex}, 0 0 50px ${champion.primary_hex}`;
            }
        }
        if (countdownEl) {
            countdownEl.innerText = Math.ceil(gameState.timer);
        }
        if (championOverlay && championOverlay.classList.contains("hidden")) {
            championOverlay.classList.remove("hidden");
        }
    } else {
        if (championOverlay && !championOverlay.classList.contains("hidden")) {
            championOverlay.classList.add("hidden");
        }
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

    // Update Pause Button state
    const pauseBtn = document.getElementById("btn-pause");
    if (pauseBtn) {
        if (gameState.is_paused) {
            pauseBtn.innerText = "RESUME SIMULATION";
            if (!pauseBtn.classList.contains("active-paused")) {
                pauseBtn.classList.add("active-paused");
            }
            const statusEl = document.getElementById("status-label");
            if (statusEl && statusEl.innerText !== "GRID SIMULATION PAUSED") {
                statusEl.innerText = "GRID SIMULATION PAUSED";
            }
        } else {
            pauseBtn.innerText = "PAUSE SIMULATION";
            if (pauseBtn.classList.contains("active-paused")) {
                pauseBtn.classList.remove("active-paused");
            }
        }
    }
    lastState = gameState.state;
}

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
            card.className = `bot-card ${p.team}-card ${p.is_alive ? '' : 'dead'}`;
            
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
            card.className = `bot-card ${p.team}-card ${p.is_alive ? '' : 'dead'}`;
            
            const nameEl = card.querySelector(".bot-name");
            if (nameEl && nameEl.innerText !== p.name) nameEl.innerText = p.name;
            
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

function updateTournamentBracket(tournament, state) {
    if (!tournament || !tournament.matches || tournament.matches.length < 7 || !tournament.teams) return;
    
    // Auto expand/collapse based on game state
    const panel = document.getElementById("tournament-bracket-panel");
    if (panel) {
        if (state === "PREGAME") {
            // Hide HUD bracket panel during pregame, since it is shown full-size in the pregame overlay dashboard
            panel.style.display = "none";
        } else {
            panel.style.display = "block";
            if (state === "AUDITING" || state === "CHAMPION_CELEBRATION") {
                if (!panel.classList.contains("expanded")) {
                    panel.classList.add("expanded");
                }
            } else if (state === "RUNNING") {
                if (lastState === "PREGAME" && panel.classList.contains("expanded")) {
                    panel.classList.remove("expanded");
                }
            }
        }
    }

    
    // Summary toggle header text
    const summaryText = document.getElementById("tournament-summary-text");
    if (summaryText) {
        if (tournament.champion_index !== null) {
            const champion = tournament.teams[tournament.champion_index];
            summaryText.innerText = `🏆 TOURNAMENT WINNER: ${champion.name.toUpperCase()} 🏆`;
            summaryText.className = "neon-text-purple";
        } else {
            const activeMatch = tournament.matches[tournament.current_match_index];
            if (activeMatch) {
                const blueTeam = tournament.teams[activeMatch.blue_team_index];
                const orangeTeam = tournament.teams[activeMatch.orange_team_index];
                summaryText.innerText = `TOURNAMENT ACTIVE: [${activeMatch.name}] ${blueTeam.name} vs ${orangeTeam.name}`;
            }
        }
    }

    // Dynamic bracket layout renderer
    const content = document.getElementById("bracket-content");
    if (content) {
        let gridHtml = `<div class="bracket-grid">`;
        
        // Column 1: Quarter-Finals (QF1, QF2, QF3, QF4)
        gridHtml += `<div class="bracket-column" style="gap: 5px;">`;
        [0, 1, 2, 3].forEach(idx => {
            const m = tournament.matches[idx];
            const tBlue = tournament.teams[m.blue_team_index];
            const tOrange = tournament.teams[m.orange_team_index];
            const isActive = tournament.current_match_index === idx && tournament.champion_index === null;
            const isCompleted = m.is_completed;
            
            let blueWinnerClass = isCompleted && m.winner_team_index === m.blue_team_index ? "winner-text" : "";
            let orangeWinnerClass = isCompleted && m.winner_team_index === m.orange_team_index ? "winner-text" : "";
            let blueScoreClass = isCompleted && m.winner_team_index === m.blue_team_index ? "winner-score" : "";
            let orangeScoreClass = isCompleted && m.winner_team_index === m.orange_team_index ? "winner-score" : "";
            
            gridHtml += `
                <div class="bracket-match ${isActive ? 'active' : ''} ${isCompleted ? 'completed' : ''}">
                    <div class="bracket-match-title">${m.name}</div>
                    <div class="bracket-team-row">
                        <span class="bracket-team-name ${blueWinnerClass}" style="color: ${tBlue.primary_hex}">
                            ${tBlue.name} <span style="font-size: 0.55rem; color: #a0aec0; font-weight: normal; margin-left: 4px;">(${tBlue.match_wins}W-${tBlue.match_losses}L${tBlue.championships > 0 ? ' ' + tBlue.championships + '🏆' : ''})</span>
                        </span>
                        <span class="bracket-team-score ${blueScoreClass}">${m.blue_score !== null ? m.blue_score : '-'}</span>
                    </div>
                    <div class="bracket-team-row">
                        <span class="bracket-team-name ${orangeWinnerClass}" style="color: ${tOrange.primary_hex}">
                            ${tOrange.name} <span style="font-size: 0.55rem; color: #a0aec0; font-weight: normal; margin-left: 4px;">(${tOrange.match_wins}W-${tOrange.match_losses}L${tOrange.championships > 0 ? ' ' + tOrange.championships + '🏆' : ''})</span>
                        </span>
                        <span class="bracket-team-score ${orangeScoreClass}">${m.orange_score !== null ? m.orange_score : '-'}</span>
                    </div>
                </div>
            `;
        });
        gridHtml += `</div>`;
        
        // Column 2: Semi-Finals (SF1, SF2)
        gridHtml += `<div class="bracket-column" style="gap: 30px;">`;
        [4, 5].forEach(idx => {
            const m = tournament.matches[idx];
            const tBlue = tournament.teams[m.blue_team_index];
            const tOrange = tournament.teams[m.orange_team_index];
            const isActive = tournament.current_match_index === idx && tournament.champion_index === null;
            const isCompleted = m.is_completed;
            
            const isBlueDecided = (idx === 4) ? tournament.matches[0].is_completed : tournament.matches[2].is_completed;
            const isOrangeDecided = (idx === 4) ? tournament.matches[1].is_completed : tournament.matches[3].is_completed;
            
            const blueLabel = isBlueDecided ? tBlue.name : (idx === 4 ? "TBD (Winner QF1)" : "TBD (Winner QF3)");
            const orangeLabel = isOrangeDecided ? tOrange.name : (idx === 4 ? "TBD (Winner QF2)" : "TBD (Winner QF4)");
            
            let blueWinnerClass = isCompleted && m.winner_team_index === m.blue_team_index ? "winner-text" : "";
            let orangeWinnerClass = isCompleted && m.winner_team_index === m.orange_team_index ? "winner-text" : "";
            let blueScoreClass = isCompleted && m.winner_team_index === m.blue_team_index ? "winner-score" : "";
            let orangeScoreClass = isCompleted && m.winner_team_index === m.orange_team_index ? "winner-score" : "";
            
            gridHtml += `
                <div class="bracket-match ${isActive ? 'active' : ''} ${isCompleted ? 'completed' : ''}">
                    <div class="bracket-match-title">${m.name}</div>
                    <div class="bracket-team-row">
                        <span class="bracket-team-name ${blueWinnerClass}" style="color: ${isBlueDecided ? tBlue.primary_hex : '#718096'}">
                            ${blueLabel}
                            ${isBlueDecided ? `<span style="font-size: 0.55rem; color: #a0aec0; font-weight: normal; margin-left: 4px;">(${tBlue.match_wins}W-${tBlue.match_losses}L${tBlue.championships > 0 ? ' ' + tBlue.championships + '🏆' : ''})</span>` : ''}
                        </span>
                        <span class="bracket-team-score ${blueScoreClass}">${isBlueDecided && m.blue_score !== null ? m.blue_score : '-'}</span>
                    </div>
                    <div class="bracket-team-row">
                        <span class="bracket-team-name ${orangeWinnerClass}" style="color: ${isOrangeDecided ? tOrange.primary_hex : '#718096'}">
                            ${orangeLabel}
                            ${isOrangeDecided ? `<span style="font-size: 0.55rem; color: #a0aec0; font-weight: normal; margin-left: 4px;">(${tOrange.match_wins}W-${tOrange.match_losses}L${tOrange.championships > 0 ? ' ' + tOrange.championships + '🏆' : ''})</span>` : ''}
                        </span>
                        <span class="bracket-team-score ${orangeScoreClass}">${isOrangeDecided && m.orange_score !== null ? m.orange_score : '-'}</span>
                    </div>
                </div>
            `;
        });
        gridHtml += `</div>`;
        
        // Column 3: Finals
        gridHtml += `<div class="bracket-column">`;
        const mFinal = tournament.matches[6];
        const tBlueFinal = tournament.teams[mFinal.blue_team_index];
        const tOrangeFinal = tournament.teams[mFinal.orange_team_index];
        const isActiveFinal = tournament.current_match_index === 6 && tournament.champion_index === null;
        const isCompletedFinal = mFinal.is_completed;
        
        const isBlueFinalistDecided = tournament.matches[4].is_completed;
        const isOrangeFinalistDecided = tournament.matches[5].is_completed;
        
        const blueFinalLabel = isBlueFinalistDecided ? tBlueFinal.name : "TBD (Winner SF1)";
        const orangeFinalLabel = isOrangeFinalistDecided ? tOrangeFinal.name : "TBD (Winner SF2)";
        
        let blueFinalWinnerClass = isCompletedFinal && mFinal.winner_team_index === mFinal.blue_team_index ? "winner-text" : "";
        let orangeFinalWinnerClass = isCompletedFinal && mFinal.winner_team_index === mFinal.orange_team_index ? "winner-text" : "";
        let blueFinalScoreClass = isCompletedFinal && mFinal.winner_team_index === mFinal.blue_team_index ? "winner-score" : "";
        let orangeFinalScoreClass = isCompletedFinal && mFinal.winner_team_index === mFinal.orange_team_index ? "winner-score" : "";
        
        gridHtml += `
            <div class="bracket-match ${isActiveFinal ? 'active' : ''} ${isCompletedFinal ? 'completed' : ''}">
                <div class="bracket-match-title">Finals</div>
                <div class="bracket-team-row">
                    <span class="bracket-team-name ${blueFinalWinnerClass}" style="color: ${isBlueFinalistDecided ? tBlueFinal.primary_hex : '#718096'}">
                        ${blueFinalLabel}
                        ${isBlueFinalistDecided ? `<span style="font-size: 0.55rem; color: #a0aec0; font-weight: normal; margin-left: 4px;">(${tBlueFinal.match_wins}W-${tBlueFinal.match_losses}L${tBlueFinal.championships > 0 ? ' ' + tBlueFinal.championships + '🏆' : ''})</span>` : ''}
                    </span>
                    <span class="bracket-team-score ${blueFinalScoreClass}">${isBlueFinalistDecided && mFinal.blue_score !== null ? mFinal.blue_score : '-'}</span>
                </div>
                <div class="bracket-team-row">
                    <span class="bracket-team-name ${orangeFinalWinnerClass}" style="color: ${isOrangeFinalistDecided ? tOrangeFinal.primary_hex : '#718096'}">
                        ${orangeFinalLabel}
                        ${isOrangeFinalistDecided ? `<span style="font-size: 0.55rem; color: #a0aec0; font-weight: normal; margin-left: 4px;">(${tOrangeFinal.match_wins}W-${tOrangeFinal.match_losses}L${tOrangeFinal.championships > 0 ? ' ' + tOrangeFinal.championships + '🏆' : ''})</span>` : ''}
                    </span>
                    <span class="bracket-team-score ${orangeFinalScoreClass}">${isOrangeFinalistDecided && mFinal.orange_score !== null ? mFinal.orange_score : '-'}</span>
                </div>
            </div>
        `;
        gridHtml += `</div>`;
        
        // Column 4: Champion Box
        gridHtml += `<div class="bracket-column">`;
        const isCrowned = tournament.champion_index !== null;
        const tChamp = isCrowned ? tournament.teams[tournament.champion_index] : null;
        gridHtml += `
            <div class="bracket-champion-box ${isCrowned ? 'crowned' : ''}">
                <div class="bracket-champion-title">🏆 Champion 🏆</div>
                <div class="bracket-champion-name">${tChamp ? tChamp.name.toUpperCase() : 'AWAITING VECTORS'}</div>
                ${tChamp ? `<div style="font-size: 0.6rem; color: #ffd700; font-family: var(--font-mono); margin-top: 4px;">Total Titles: ${tChamp.championships}🏆</div>` : ''}
            </div>
        `;
        gridHtml += `</div>`;
        
        gridHtml += `</div>`;
        content.innerHTML = gridHtml;
        
        // Also populate the pregame dashboard bracket if it exists
        const pregameContent = document.getElementById("pregame-bracket-content");
        if (pregameContent) {
            pregameContent.innerHTML = gridHtml;
        }
    }
}

let lastState = null;
