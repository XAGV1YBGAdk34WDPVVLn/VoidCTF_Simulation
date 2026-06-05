// game_audio.js: Synthesis-based audio system for retro-cyberpunk sound effects.

class GridAudio {
    constructor() {
        this.ctx = null;
        this.volume = 0.25;
        this.muted = false;
        this.lastPlayTime = {};
        this.buffers = {}; // Store local audio buffers
        // Proactively try to initialize immediately (may start suspended)
        this.init();
    }

    async loadBuffer(name, url) {
        try {
            const response = await fetch(url);
            const arrayBuffer = await response.arrayBuffer();
            const decodedData = await this.ctx.decodeAudioData(arrayBuffer);
            this.buffers[name] = decodedData;
            console.log(`GridAudio: Decoded buffer successfully for ${name}`);
        } catch (e) {
            console.error(`GridAudio: Failed to load/decode sound ${name} from ${url}`, e);
        }
    }

    init() {
        if (this.ctx) return;
        try {
            this.ctx = new (window.AudioContext || window.webkitAudioContext)();
            console.log("GridAudio: AudioContext initialized. State:", this.ctx.state);
            // Load local MP3 assets
            this.loadBuffer("bit_yes", "/static/audio/bit-yes.mp3");
            this.loadBuffer("bit_no", "/static/audio/bit-no.mp3");
        } catch (e) {
            console.error("GridAudio: Web Audio API not supported", e);
        }
    }

    playBuffer(bufferName, rate = 1.0, count = 1, interval = 250) {
        if (this.muted || !this.ctx) return;
        const buffer = this.buffers[bufferName];
        if (!buffer) return;

        const now = this.ctx.currentTime;
        for (let i = 0; i < count; i++) {
            const startTime = now + i * (interval / 1000);
            const source = this.ctx.createBufferSource();
            source.buffer = buffer;
            source.playbackRate.setValueAtTime(rate, startTime);
            
            const gain = this.ctx.createGain();
            gain.gain.setValueAtTime(this.volume, startTime);
            
            source.connect(gain);
            gain.connect(this.ctx.destination);
            
            source.start(startTime);
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

    playBitYes() {
        if (!this.canPlay("bit_yes", 800)) return;
        if (this.buffers["bit_yes"]) {
            // Play authentic MP3 YES sound 3 times, spaced 250ms apart
            this.playBuffer("bit_yes", 1.0, 3, 250);
            return;
        }
        
        // Fallback custom synthesis
        const now = this.ctx.currentTime;
        const vol = this.volume;
        const numSyllables = 5;
        const interval = 0.11;
        const syllableDur = 0.08;
        for (let i = 0; i < numSyllables; i++) {
            const tStart = now + i * interval;
            const osc1 = this.ctx.createOscillator();
            osc1.type = "square";
            osc1.frequency.setValueAtTime(320, tStart);
            osc1.frequency.linearRampToValueAtTime(360, tStart + syllableDur);
            const osc2 = this.ctx.createOscillator();
            osc2.type = "square";
            osc2.frequency.setValueAtTime(328, tStart);
            osc2.frequency.linearRampToValueAtTime(368, tStart + syllableDur);
            const f1 = this.ctx.createBiquadFilter();
            f1.type = "bandpass";
            f1.Q.setValueAtTime(10, tStart);
            f1.frequency.setValueAtTime(320, tStart);
            f1.frequency.exponentialRampToValueAtTime(600, tStart + syllableDur);
            const f2 = this.ctx.createBiquadFilter();
            f2.type = "bandpass";
            f2.Q.setValueAtTime(10, tStart);
            f2.frequency.setValueAtTime(2400, tStart);
            f2.frequency.exponentialRampToValueAtTime(1800, tStart + syllableDur);
            const voicedGain = this.ctx.createGain();
            voicedGain.gain.setValueAtTime(0.001, tStart);
            voicedGain.gain.linearRampToValueAtTime(vol * 0.45, tStart + 0.01);
            voicedGain.gain.exponentialRampToValueAtTime(0.001, tStart + syllableDur);
            osc1.connect(f1);
            osc2.connect(f2);
            f1.connect(voicedGain);
            f2.connect(voicedGain);
            voicedGain.connect(this.ctx.destination);
            osc1.start(tStart);
            osc2.start(tStart);
            osc1.stop(tStart + syllableDur);
            osc2.stop(tStart + syllableDur);
            const sampleRate = this.ctx.sampleRate;
            const noiseDur = 0.04;
            const noiseSize = sampleRate * noiseDur;
            const noiseBuffer = this.ctx.createBuffer(1, noiseSize, sampleRate);
            const output = noiseBuffer.getChannelData(0);
            for (let j = 0; j < noiseSize; j++) {
                output[j] = Math.random() * 2 - 1;
            }
            const noiseSource = this.ctx.createBufferSource();
            noiseSource.buffer = noiseBuffer;
            const hpf = this.ctx.createBiquadFilter();
            hpf.type = "bandpass";
            hpf.Q.setValueAtTime(6, tStart + syllableDur - 0.015);
            hpf.frequency.setValueAtTime(4500, tStart + syllableDur - 0.015);
            const noiseGain = this.ctx.createGain();
            noiseGain.gain.setValueAtTime(0.001, tStart + syllableDur - 0.015);
            noiseGain.gain.linearRampToValueAtTime(vol * 0.16, tStart + syllableDur);
            noiseGain.gain.exponentialRampToValueAtTime(0.001, tStart + tStart + syllableDur + 0.035);
            noiseSource.connect(hpf);
            hpf.connect(noiseGain);
            noiseGain.connect(this.ctx.destination);
            noiseSource.start(tStart + syllableDur - 0.015);
            noiseSource.stop(tStart + syllableDur + 0.035);
        }
    }

    playBitNo() {
        if (!this.canPlay("bit_no", 800)) return;
        if (this.buffers["bit_no"]) {
            // Play authentic MP3 NO sound once
            this.playBuffer("bit_no", 1.0, 1, 0);
            return;
        }
        
        // Fallback custom synthesis
        const now = this.ctx.currentTime;
        const vol = this.volume;
        const dur = 0.52;
        const osc1 = this.ctx.createOscillator();
        osc1.type = "square";
        osc1.frequency.setValueAtTime(75, now);
        osc1.frequency.linearRampToValueAtTime(60, now + dur);
        const osc2 = this.ctx.createOscillator();
        osc2.type = "square";
        osc2.frequency.setValueAtTime(77, now);
        osc2.frequency.linearRampToValueAtTime(62, now + dur);
        const lfo = this.ctx.createOscillator();
        lfo.type = "square";
        lfo.frequency.setValueAtTime(24, now);
        const lfoGain = this.ctx.createGain();
        lfoGain.gain.setValueAtTime(32, now);
        lfo.connect(lfoGain);
        lfoGain.connect(osc1.frequency);
        lfoGain.connect(osc2.frequency);
        const f1 = this.ctx.createBiquadFilter();
        f1.type = "bandpass";
        f1.Q.setValueAtTime(12, now);
        f1.frequency.setValueAtTime(240, now);
        f1.frequency.linearRampToValueAtTime(450, now + dur * 0.4);
        const f2 = this.ctx.createBiquadFilter();
        f2.type = "bandpass";
        f2.Q.setValueAtTime(12, now);
        f2.frequency.setValueAtTime(650, now);
        f2.frequency.linearRampToValueAtTime(950, now + dur * 0.4);
        const gainNode = this.ctx.createGain();
        gainNode.gain.setValueAtTime(0.001, now);
        gainNode.gain.linearRampToValueAtTime(vol * 0.9, now + 0.05);
        gainNode.gain.linearRampToValueAtTime(vol * 0.7, now + dur * 0.6);
        gainNode.gain.exponentialRampToValueAtTime(0.001, now + dur);
        osc1.connect(f1);
        osc2.connect(f2);
        f1.connect(gainNode);
        f2.connect(gainNode);
        gainNode.connect(this.ctx.destination);
        lfo.start(now);
        osc1.start(now);
        osc2.start(now);
        lfo.stop(now + dur);
        osc1.stop(now + dur);
        osc2.stop(now + dur);
    }
}

// Global instance
const gridAudio = new GridAudio();
window.gridAudio = gridAudio;

// Early user interaction listener to unlock and resume AudioContext immediately
const unlockGridAudio = () => {
    if (gridAudio.ctx) {
        if (gridAudio.ctx.state === "suspended") {
            gridAudio.ctx.resume().then(() => {
                console.log("GridAudio: AudioContext resumed successfully via user interaction.");
            });
        }
    } else {
        gridAudio.init();
    }
    // Once context is running, clean up listeners
    if (gridAudio.ctx && gridAudio.ctx.state === "running") {
        document.removeEventListener("click", unlockGridAudio);
        document.removeEventListener("keydown", unlockGridAudio);
        document.removeEventListener("mousedown", unlockGridAudio);
        document.removeEventListener("touchstart", unlockGridAudio);
    }
};

document.addEventListener("click", unlockGridAudio);
document.addEventListener("keydown", unlockGridAudio);
document.addEventListener("mousedown", unlockGridAudio);
document.addEventListener("touchstart", unlockGridAudio);
