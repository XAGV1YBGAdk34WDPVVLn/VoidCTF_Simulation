// game_audio.js: Synthesis-based audio system for retro-cyberpunk sound effects.

class GridAudio {
    constructor() {
        this.ctx = null;
        this.volume = 0.25;
        this.muted = false;
        this.lastPlayTime = {};
    }

    init() {
        if (this.ctx) return;
        try {
            this.ctx = new (window.AudioContext || window.webkitAudioContext)();
            console.log("GridAudio: AudioContext initialized.");
        } catch (e) {
            console.error("GridAudio: Web Audio API not supported", e);
        }
    }

    setVolume(vol) {
        this.volume = Math.max(0, Math.min(1, vol));
        const display = document.getElementById("audio-volume-display");
        if (display) {
            display.innerText = `${Math.round(this.volume * 100)}%`;
        }
    }

    toggleMute() {
        this.muted = !this.muted;
        const btn = document.getElementById("btn-audio-mute");
        if (btn) {
            if (this.muted) {
                btn.innerText = "UNMUTE";
                btn.classList.add("active");
            } else {
                btn.innerText = "MUTE";
                btn.classList.remove("active");
            }
        }
        return this.muted;
    }

    canPlay(type, cooldownMs = 100) {
        if (this.muted || !this.ctx) return false;
        if (this.ctx.state === "suspended") {
            this.ctx.resume();
        }
        const now = Date.now();
        const last = this.lastPlayTime[type] || 0;
        if (now - last < cooldownMs) return false;
        this.lastPlayTime[type] = now;
        return true;
    }

    playSpark() {
        if (!this.canPlay("spark", 150)) return;
        
        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        
        osc.type = "triangle";
        osc.frequency.setValueAtTime(900, this.ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(150, this.ctx.currentTime + 0.07);
        
        gain.gain.setValueAtTime(this.volume * 0.4, this.ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + 0.07);
        
        osc.connect(gain);
        gain.connect(this.ctx.destination);
        
        osc.start();
        osc.stop(this.ctx.currentTime + 0.07);
    }

    playOvercharge() {
        if (!this.canPlay("overcharge", 1000)) return;
        
        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        
        osc.type = "sine";
        osc.frequency.setValueAtTime(300, this.ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(1600, this.ctx.currentTime + 0.35);
        
        gain.gain.setValueAtTime(0.01, this.ctx.currentTime);
        gain.gain.linearRampToValueAtTime(this.volume * 0.6, this.ctx.currentTime + 0.05);
        gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + 0.35);
        
        osc.connect(gain);
        gain.connect(this.ctx.destination);
        
        osc.start();
        osc.stop(this.ctx.currentTime + 0.35);
    }

    playFlagPickup() {
        if (!this.canPlay("flag_pickup", 500)) return;
        
        const notes = [523.25, 659.25, 783.99]; // C5, E5, G5
        notes.forEach((freq, index) => {
            const osc = this.ctx.createOscillator();
            const gain = this.ctx.createGain();
            
            osc.type = "sine";
            osc.frequency.setValueAtTime(freq, this.ctx.currentTime + index * 0.06);
            
            gain.gain.setValueAtTime(0.0, this.ctx.currentTime + index * 0.06);
            gain.gain.linearRampToValueAtTime(this.volume * 0.5, this.ctx.currentTime + index * 0.06 + 0.01);
            gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + index * 0.06 + 0.25);
            
            osc.connect(gain);
            gain.connect(this.ctx.destination);
            
            osc.start(this.ctx.currentTime + index * 0.06);
            osc.stop(this.ctx.currentTime + index * 0.06 + 0.25);
        });
    }

    playFlagReturn() {
        if (!this.canPlay("flag_return", 500)) return;
        
        const notes = [783.99, 659.25, 523.25]; // G5, E5, C5
        notes.forEach((freq, index) => {
            const osc = this.ctx.createOscillator();
            const gain = this.ctx.createGain();
            
            osc.type = "sine";
            osc.frequency.setValueAtTime(freq, this.ctx.currentTime + index * 0.07);
            
            gain.gain.setValueAtTime(0.0, this.ctx.currentTime + index * 0.07);
            gain.gain.linearRampToValueAtTime(this.volume * 0.5, this.ctx.currentTime + index * 0.07 + 0.01);
            gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + index * 0.07 + 0.3);
            
            osc.connect(gain);
            gain.connect(this.ctx.destination);
            
            osc.start(this.ctx.currentTime + index * 0.07);
            osc.stop(this.ctx.currentTime + index * 0.07 + 0.3);
        });
    }

    playScore() {
        if (!this.canPlay("score", 1000)) return;
        
        const chords = [261.63, 329.63, 392.00, 523.25, 659.25, 783.99, 1046.50]; // C4, E4, G4, C5, E5, G5, C6
        chords.forEach((freq) => {
            const osc = this.ctx.createOscillator();
            const gain = this.ctx.createGain();
            
            osc.type = "sine";
            osc.frequency.setValueAtTime(freq, this.ctx.currentTime);
            
            gain.gain.setValueAtTime(this.volume * 0.3, this.ctx.currentTime);
            gain.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + 0.85);
            
            osc.connect(gain);
            gain.connect(this.ctx.destination);
            
            osc.start();
            osc.stop(this.ctx.currentTime + 0.9);
        });
    }

    playDeRez() {
        if (!this.canPlay("derez", 300)) return;
        
        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        
        osc.type = "sawtooth";
        osc.frequency.setValueAtTime(400, this.ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(80, this.ctx.currentTime + 0.2);
        
        const filter = this.ctx.createBiquadFilter();
        filter.type = "lowpass";
        filter.frequency.setValueAtTime(1000, this.ctx.currentTime);
        filter.frequency.exponentialRampToValueAtTime(100, this.ctx.currentTime + 0.2);
        
        gain.gain.setValueAtTime(this.volume * 0.4, this.ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + 0.2);
        
        osc.connect(filter);
        filter.connect(gain);
        gain.connect(this.ctx.destination);
        
        osc.start();
        osc.stop(this.ctx.currentTime + 0.2);
    }

    playTick(isFinal = false) {
        if (!this.canPlay("tick", 100)) return;
        
        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        
        osc.type = "sine";
        if (isFinal) {
            osc.frequency.setValueAtTime(2200, this.ctx.currentTime);
            gain.gain.setValueAtTime(this.volume * 0.4, this.ctx.currentTime);
            gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + 0.08);
            
            osc.connect(gain);
            gain.connect(this.ctx.destination);
            
            osc.start();
            osc.stop(this.ctx.currentTime + 0.08);
        } else {
            osc.frequency.setValueAtTime(1600, this.ctx.currentTime);
            gain.gain.setValueAtTime(this.volume * 0.2, this.ctx.currentTime);
            gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + 0.015);
            
            osc.connect(gain);
            gain.connect(this.ctx.destination);
            
            osc.start();
            osc.stop(this.ctx.currentTime + 0.015);
        }
    }

    playMatchStart() {
        if (!this.canPlay("start", 1000)) return;
        
        const notes = [600, 900];
        notes.forEach((freq, index) => {
            const osc = this.ctx.createOscillator();
            const gain = this.ctx.createGain();
            
            osc.type = "sine";
            osc.frequency.setValueAtTime(freq, this.ctx.currentTime + index * 0.12);
            
            gain.gain.setValueAtTime(0.0, this.ctx.currentTime + index * 0.12);
            gain.gain.linearRampToValueAtTime(this.volume * 0.6, this.ctx.currentTime + index * 0.12 + 0.02);
            gain.gain.exponentialRampToValueAtTime(0.01, this.ctx.currentTime + index * 0.12 + 0.35);
            
            osc.connect(gain);
            gain.connect(this.ctx.destination);
            
            osc.start(this.ctx.currentTime + index * 0.12);
            osc.stop(this.ctx.currentTime + index * 0.12 + 0.35);
        });
    }
}

// Global instance
const gridAudio = new GridAudio();
window.gridAudio = gridAudio;
