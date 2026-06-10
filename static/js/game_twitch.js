// game_twitch.js: Client-side Twitch WebSocket IRC connection and command parser.

class TwitchConnector {
    constructor() {
        this.ws = null;
        this.status = "disconnected"; // "disconnected", "connecting", "connected"
        this.channel = "";
        this.reconnectTimer = null;
        this.autoReconnect = true;

        // DOM Cache
        this.inputEl = document.getElementById("twitch-channel");
        this.btnEl = document.getElementById("btn-twitch-connect");
        this.dotEl = document.getElementById("twitch-status-dot");
        this.statusTextEl = document.getElementById("twitch-status-text");
        this.telemetryEl = document.getElementById("twitch-telemetry");
        this.allowStrategiesEl = document.getElementById("twitch-allow-strategies");
        this.allowBitEl = document.getElementById("twitch-allow-bit");

        if (!this.inputEl || !this.btnEl) {
            console.error("TwitchConnector: UI elements not found.");
            return;
        }

        // Load cached channel name
        const cachedChannel = localStorage.getItem("twitch_channel_name");
        if (cachedChannel) {
            this.inputEl.value = cachedChannel;
        }

        // Setup Event Listeners
        this.btnEl.addEventListener("click", () => this.toggleConnection());
        this.inputEl.addEventListener("keydown", (e) => {
            if (e.key === "Enter") {
                this.toggleConnection();
            }
        });
    }

    addTelemetryLog(text, isAlert = false) {
        if (!this.telemetryEl) return;
        const entry = document.createElement("div");
        entry.className = "twitch-mini-entry";
        if (isAlert) {
            entry.innerHTML = `<span style="color: #ff3300;">[ERR]</span> ${text}`;
        } else {
            entry.innerHTML = `<span>[SYS]</span> ${text}`;
        }
        this.telemetryEl.appendChild(entry);
        this.telemetryEl.scrollTop = this.telemetryEl.scrollHeight;

        // Keep maximum of 30 lines
        while (this.telemetryEl.children.length > 30) {
            this.telemetryEl.removeChild(this.telemetryEl.firstChild);
        }
    }

    addChatLog(user, msg) {
        if (!this.telemetryEl) return;
        const entry = document.createElement("div");
        entry.className = "twitch-mini-entry";
        entry.innerHTML = `<span style="color: #a970ff; font-weight: bold;">@${user}:</span> ${msg}`;
        this.telemetryEl.appendChild(entry);
        this.telemetryEl.scrollTop = this.telemetryEl.scrollHeight;

        // Keep maximum of 30 lines
        while (this.telemetryEl.children.length > 30) {
            this.telemetryEl.removeChild(this.telemetryEl.firstChild);
        }
    }

    updateStatusUI(newStatus) {
        this.status = newStatus;
        if (!this.statusTextEl || !this.dotEl || !this.btnEl) return;

        // Reset classes
        this.dotEl.className = "twitch-status-indicator";

        if (this.status === "disconnected") {
            this.dotEl.classList.add("twitch-status-disconnected");
            this.statusTextEl.innerText = "DISCONNECTED";
            this.statusTextEl.style.color = "#ff3300";
            this.btnEl.innerText = "CONNECT";
            this.btnEl.classList.remove("danger-btn");
            this.btnEl.classList.add("active");
            this.inputEl.disabled = false;
        } else if (this.status === "connecting") {
            this.dotEl.classList.add("twitch-status-connecting");
            this.statusTextEl.innerText = "CONNECTING...";
            this.statusTextEl.style.color = "#ffaa00";
            this.btnEl.innerText = "CANCEL";
            this.btnEl.classList.remove("active");
            this.btnEl.classList.add("danger-btn");
            this.inputEl.disabled = true;
        } else if (this.status === "connected") {
            this.dotEl.classList.add("twitch-status-connected");
            this.statusTextEl.innerText = `LIVE: #${this.channel.toUpperCase()}`;
            this.statusTextEl.style.color = "#00ff66";
            this.btnEl.innerText = "DISCONNECT";
            this.btnEl.classList.remove("active");
            this.btnEl.classList.add("danger-btn");
            this.inputEl.disabled = true;
        }
    }

    toggleConnection() {
        if (this.status === "connected" || this.status === "connecting") {
            this.autoReconnect = false;
            this.disconnect();
        } else {
            const rawVal = this.inputEl.value.trim().toLowerCase();
            if (!rawVal) {
                this.addTelemetryLog("Please specify channel name.", true);
                if (window.gridAudio) window.gridAudio.playBitNo();
                return;
            }
            this.channel = rawVal;
            localStorage.setItem("twitch_channel_name", this.channel);
            this.autoReconnect = true;
            this.connect();
        }
    }

    connect() {
        if (this.ws) {
            this.ws.close();
        }

        this.updateStatusUI("connecting");
        this.addTelemetryLog(`Dialing Twitch gateway for #${this.channel}...`);

        try {
            this.ws = new WebSocket("wss://irc-ws.chat.twitch.tv:443");
        } catch (err) {
            this.addTelemetryLog(`Init failed: ${err.message}`, true);
            this.updateStatusUI("disconnected");
            return;
        }

        this.ws.onopen = () => {
            if (this.status !== "connecting") return;
            this.addTelemetryLog("Connection open. Logging in anonymously...");
            
            // Log in anonymously
            const username = "justinfan" + Math.floor(10000 + Math.random() * 90000);
            this.ws.send("PASS SCHMOOPY\r\n");
            this.ws.send(`NICK ${username}\r\n`);
            this.ws.send(`JOIN #${this.channel}\r\n`);
        };

        this.ws.onmessage = (event) => {
            const data = event.data;
            const lines = data.split(/\r?\n/);
            
            for (let line of lines) {
                if (!line) continue;

                // Handle Twitch IRC PING/PONG keepalive
                if (line.startsWith("PING")) {
                    this.ws.send("PONG :tmi.twitch.tv\r\n");
                    continue;
                }

                // Parse standard Twitch Message
                // Format: :user!user@user.tmi.twitch.tv PRIVMSG #channel :message text
                const match = line.match(/^:([^!]+)![^@]+@[^ ]+ PRIVMSG #[^ ]+ :(.+)$/);
                if (match) {
                    const user = match[1];
                    const msg = match[2].trim();
                    
                    // We are connected and receiving chat!
                    if (this.status === "connecting") {
                        this.updateStatusUI("connected");
                        this.addTelemetryLog("Channel joined! Monitoring chat...");
                        if (window.gridAudio) window.gridAudio.playBitYes();
                    }

                    this.handleChatMessage(user, msg);
                } else if (line.includes("366")) {
                    // RPL_ENDOFNAMES code means successfully joined channel
                    this.updateStatusUI("connected");
                    this.addTelemetryLog("Channel joined! Monitoring chat...");
                    if (window.gridAudio) window.gridAudio.playBitYes();
                }
            }
        };

        this.ws.onerror = (e) => {
            console.error("Twitch WS Error:", e);
            this.addTelemetryLog("WebSocket error occurred.", true);
        };

        this.ws.onclose = () => {
            this.ws = null;
            this.updateStatusUI("disconnected");
            this.addTelemetryLog("Connection closed.");

            if (this.autoReconnect) {
                this.addTelemetryLog("Retrying connection in 5 seconds...");
                this.reconnectTimer = setTimeout(() => this.connect(), 5000);
            }
        };
    }

    disconnect() {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
        if (this.ws) {
            this.ws.close();
        }
        this.updateStatusUI("disconnected");
        this.addTelemetryLog("Disconnected by user.");
    }

    handleChatMessage(user, msg) {
        // Show chat message in our mini-telemetry screen
        this.addChatLog(user, msg);

        // Check for bot questions (case-insensitive)
        const lowerMsg = msg.toLowerCase().trim();
        let botResponse = "";

        if (lowerMsg.startsWith("!game") || lowerMsg.startsWith("!info") || (lowerMsg.includes("what") && lowerMsg.includes("game"))) {
            botResponse = "VOID GRID: A 3D Capture the Flag simulation. Powered by Rust (engine) & WebGL/Three.js (graphics).";
        } else if (lowerMsg.startsWith("!rules") || lowerMsg.includes("how to play") || lowerMsg.includes("what are the rules")) {
            botResponse = "RULES: 3v3 CTF. Roles: Stalker (runner/dash), Enforcer (tank/shields), Tactician (nanite healer). First to score wins!";
        } else if (lowerMsg.startsWith("!code") || lowerMsg.startsWith("!tech") || lowerMsg.includes("what language")) {
            botResponse = "TECH: Built using Axum (Rust web server), WebSockets, and Three.js for hardware-accelerated 3D rendering.";
        } else if (lowerMsg.startsWith("!commands") || lowerMsg.startsWith("!help")) {
            botResponse = "COMMANDS: !rush [team], !turtle [team], !split [team], !yes, !no, !askbit [question].";
        }

        if (botResponse) {
            // Display bot reply in Grid Events
            if (window.addLocalLogEntry) {
                window.addLocalLogEntry(
                    `<span style="color: #a970ff; font-weight: bold;">[BOT RESPONSE]</span> @${user}: ${botResponse}`
                );
            }
            // Trigger Grid Bit pulse and sound
            if (window.gridBit) {
                window.gridBit.triggerPulse();
            }
            if (window.gridAudio) {
                window.gridAudio.playSpark();
            }
        }

        // Check for interactive commands
        if (!msg.startsWith("!")) return;

        const cmdParts = msg.slice(1).split(/\s+/);
        const command = cmdParts[0].toLowerCase();
        const arg1 = cmdParts[1] ? cmdParts[1].toLowerCase() : "";
        
        // 1. Strategy Overrides: !<strategy> <team>
        // E.g. !rush blue, !turtle orange
        const validStrategies = ["rush", "turtle", "split", "flank", "harass", "counter"];
        if (validStrategies.includes(command)) {
            const team = arg1;
            if (team === "blue" || team === "orange") {
                const allowStrategies = this.allowStrategiesEl ? this.allowStrategiesEl.checked : false;
                if (!allowStrategies) {
                    this.addTelemetryLog(`Command '!${command} ${team}' blocked (Allow Strategy Chat Commands is off)`);
                    return;
                }

                // Trigger main server WS override
                if (window.ws && window.ws.readyState === WebSocket.OPEN) {
                    const strategy = command.toUpperCase();
                    window.ws.send(JSON.stringify({
                        type: "apply_override_strategy",
                        team: team,
                        strategy: strategy
                    }));

                    // Log in Grid Events
                    if (window.addLocalLogEntry) {
                        const teamHex = team === "blue" ? "#00f0ff" : "#ff7b00";
                        window.addLocalLogEntry(
                            `<span style="color: #a970ff;">[TWITCH CHAT]</span> @${user} forced strategy <span style="color: #ffffff; font-weight: bold;">${strategy}</span> for <span style="color: ${teamHex}; font-weight: bold;">TEAM ${team.toUpperCase()}</span>!`
                        );
                    }

                    // Sound cue
                    if (window.gridAudio) {
                        window.gridAudio.playSpark();
                    }
                }
            }
        }

        // 2. Tron Grid Bit: !yes / !no / !bit yes / !bit no
        if (command === "yes" || (command === "bit" && arg1 === "yes")) {
            const allowBit = this.allowBitEl ? this.allowBitEl.checked : false;
            if (!allowBit) return;

            if (window.gridBit) {
                window.gridBit.triggerYes();
                if (window.addLocalLogEntry) {
                    window.addLocalLogEntry(
                        `<span style="color: #a970ff;">[TWITCH CHAT]</span> @${user} triggered Grid Assistant: <span style="color: #00ffff; font-weight: bold;">YES YES YES</span>`
                    );
                }
            }
        } else if (command === "no" || (command === "bit" && arg1 === "no")) {
            const allowBit = this.allowBitEl ? this.allowBitEl.checked : false;
            if (!allowBit) return;

            if (window.gridBit) {
                window.gridBit.triggerNo();
                if (window.addLocalLogEntry) {
                    window.addLocalLogEntry(
                        `<span style="color: #a970ff;">[TWITCH CHAT]</span> @${user} triggered Grid Assistant: <span style="color: #ff3300; font-weight: bold;">NO NO NO</span>`
                    );
                }
            }
        }

        // 3. Ask the Grid Bit: !askbit <question> or !bit <question>
        else if (command === "askbit" || (command === "bit" && cmdParts.length > 1 && arg1 !== "yes" && arg1 !== "no")) {
            const allowBit = this.allowBitEl ? this.allowBitEl.checked : false;
            if (!allowBit) return;

            // Extract question
            let question = "";
            if (command === "askbit") {
                question = msg.slice(8).trim();
            } else {
                question = msg.slice(5).trim();
            }

            if (question.length > 0) {
                const decision = Math.random() > 0.5;
                if (window.gridBit) {
                    if (decision) {
                        window.gridBit.triggerYes();
                    } else {
                        window.gridBit.triggerNo();
                    }
                }

                if (window.addLocalLogEntry) {
                    const decisionText = decision ? 
                        `<span style="color: #00ffff; font-weight: bold;">YES YES YES</span>` : 
                        `<span style="color: #ff3300; font-weight: bold;">NO NO NO</span>`;
                    
                    window.addLocalLogEntry(
                        `<span style="color: #a970ff;">[TWITCH CHAT]</span> @${user} asked: "${question}" -> GRID BIT DECISION: ${decisionText}`
                    );
                }
            }
        }
    }
}

// Instantiate on DOM load
window.addEventListener("DOMContentLoaded", () => {
    window.twitchConnector = new TwitchConnector();
});
