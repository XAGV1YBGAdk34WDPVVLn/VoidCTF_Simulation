// game_bit.js: Three.js renderer and animator for the Tron Bit Grid Assistant.

class GridBit {
    constructor() {
        this.canvas = document.getElementById("bit-canvas");
        if (!this.canvas) {
            console.error("GridBit: Canvas element #bit-canvas not found.");
            return;
        }
        
        this.statusEl = document.getElementById("bit-status");
        this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, alpha: true, antialias: true });
        this.renderer.setSize(100, 100);
        this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
        
        this.scene = new THREE.Scene();
        this.camera = new THREE.PerspectiveCamera(45, 1, 0.1, 10);
        this.camera.position.z = 2.5;
        
        // Add subtle lights for materials
        const ambient = new THREE.AmbientLight(0xffffff, 0.4);
        this.scene.add(ambient);
        const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
        dirLight.position.set(1, 2, 3);
        this.scene.add(dirLight);
        
        this.currentState = "idle"; // "idle", "yes", "no"
        this.stateTimer = 0;
        
        this.createGeometries();
        this.createMeshes();
        
        // Start animation loop
        this.animate();
        
        // Interactive click triggers a local YES / NO test for user fun
        this.canvas.addEventListener("click", () => {
            if (this.currentState === "idle") {
                if (Math.random() > 0.5) {
                    this.triggerYes();
                } else {
                    this.triggerNo();
                }
            }
        });
    }
    
    createGeometries() {
        // Idle: Dodecahedron
        this.geoIdle = new THREE.DodecahedronGeometry(0.5, 0);
        
        // YES: Octahedron
        this.geoYes = new THREE.OctahedronGeometry(0.48, 0);
        
        // NO: Icosahedron (vibrated in render loop)
        this.geoNo = new THREE.IcosahedronGeometry(0.42, 1);
    }
    
    createMeshes() {
        // Material for Idle: Yellow-Green glowing wireframe + solid semi-transparent
        this.matIdleSolid = new THREE.MeshBasicMaterial({
            color: 0x99ff00,
            transparent: true,
            opacity: 0.15,
            wireframe: false
        });
        this.matIdleWire = new THREE.LineBasicMaterial({
            color: 0xccff00,
            linewidth: 1.5
        });
        
        // Material for YES: Light blue/cyan solid + wireframe
        this.matYesSolid = new THREE.MeshBasicMaterial({
            color: 0x00f0ff,
            transparent: true,
            opacity: 0.35,
            wireframe: false
        });
        this.matYesWire = new THREE.LineBasicMaterial({
            color: 0x00ffff,
            linewidth: 2.0
        });
        
        // Material for NO: Red/Orange solid + wireframe
        this.matNoSolid = new THREE.MeshBasicMaterial({
            color: 0xff3300,
            transparent: true,
            opacity: 0.3,
            wireframe: false
        });
        this.matNoWire = new THREE.LineBasicMaterial({
            color: 0xff0000,
            linewidth: 2.0
        });
        
        // Create root group
        this.group = new THREE.Group();
        this.scene.add(this.group);
        
        // Build mesh states
        this.meshIdle = new THREE.Mesh(this.geoIdle, this.matIdleSolid);
        const edgesIdle = new THREE.EdgesGeometry(this.geoIdle);
        this.wireIdle = new THREE.LineSegments(edgesIdle, this.matIdleWire);
        this.meshIdle.add(this.wireIdle);
        this.group.add(this.meshIdle);
        
        this.meshYes = new THREE.Mesh(this.geoYes, this.matYesSolid);
        const edgesYes = new THREE.EdgesGeometry(this.geoYes);
        this.wireYes = new THREE.LineSegments(edgesYes, this.matYesWire);
        this.meshYes.add(this.wireYes);
        this.group.add(this.meshYes);
        
        this.meshNo = new THREE.Mesh(this.geoNo, this.matNoSolid);
        const edgesNo = new THREE.EdgesGeometry(this.geoNo);
        this.wireNo = new THREE.LineSegments(edgesNo, this.matNoWire);
        this.meshNo.add(this.wireNo);
        this.group.add(this.meshNo);
        
        this.updateVisibilities();
    }
    
    updateVisibilities() {
        this.meshIdle.visible = (this.currentState === "idle");
        this.meshYes.visible = (this.currentState === "yes");
        this.meshNo.visible = (this.currentState === "no");
    }
    
    setStatus(text, color) {
        if (this.statusEl) {
            this.statusEl.innerText = text;
            this.statusEl.style.color = color;
        }
    }
    
    triggerYes() {
        this.currentState = "yes";
        this.stateTimer = 1.5; // Stay in YES state for 1.5 seconds
        this.updateVisibilities();
        this.setStatus("YES YES YES", "#00ffff");
        if (window.gridAudio) {
            window.gridAudio.playBitYes();
        }
    }
    
    triggerNo() {
        this.currentState = "no";
        this.stateTimer = 1.5; // Stay in NO state for 1.5 seconds
        this.updateVisibilities();
        this.setStatus("NO NO NO", "#ff3300");
        if (window.gridAudio) {
            window.gridAudio.playBitNo();
        }
    }
    
    triggerPulse() {
        // Simple scale pulse, only on Idle to not disrupt other active states
        if (this.currentState === "idle") {
            this.group.scale.set(1.3, 1.3, 1.3);
        }
    }
    
    animate() {
        requestAnimationFrame(() => this.animate());
        
        const time = Date.now() * 0.002;
        
        // State timer count down
        if (this.stateTimer > 0) {
            this.stateTimer -= 0.016; // approx 1 frame at 60Hz
            if (this.stateTimer <= 0) {
                this.currentState = "idle";
                this.updateVisibilities();
                this.setStatus("READY", "#00f0ff");
            }
        }
        
        // Manage rotations and structural animations per state
        if (this.currentState === "idle") {
            this.meshIdle.rotation.x = time * 0.4;
            this.meshIdle.rotation.y = time * 0.6;
            
            // Revert group scale back to 1.0 smoothly
            this.group.scale.x += (1.0 - this.group.scale.x) * 0.15;
            this.group.scale.y += (1.0 - this.group.scale.y) * 0.15;
            this.group.scale.z += (1.0 - this.group.scale.z) * 0.15;
            
            // Soft opacity breath pulse
            this.matIdleSolid.opacity = 0.12 + Math.sin(time * 2.5) * 0.04;
        } 
        else if (this.currentState === "yes") {
            this.meshYes.rotation.x = time * 1.5;
            this.meshYes.rotation.y = time * 2.2;
            
            // Fast bounce scale pulse
            const s = 1.0 + Math.sin(time * 12) * 0.12;
            this.group.scale.set(s, s, s);
            
            this.matYesSolid.opacity = 0.35 + Math.sin(time * 8) * 0.1;
        } 
        else if (this.currentState === "no") {
            this.meshNo.rotation.x = time * 3.5;
            this.meshNo.rotation.y = time * 4.5;
            
            // High frequency vibration scale jitter
            const s = 1.0 + (Math.random() - 0.5) * 0.15;
            this.group.scale.set(s, s, s);
            
            // Position jitter to look highly unstable
            this.group.position.set(
                (Math.random() - 0.5) * 0.04,
                (Math.random() - 0.5) * 0.04,
                (Math.random() - 0.5) * 0.04
            );
            
            this.matNoSolid.opacity = 0.45 + (Math.random() - 0.5) * 0.15;
        }
        
        // Reset base position if not vibrating
        if (this.currentState !== "no") {
            this.group.position.set(0, 0, 0);
        }
        
        this.renderer.render(this.scene, this.camera);
    }
}

// Initialize on DOM load
window.addEventListener("DOMContentLoaded", () => {
    window.gridBit = new GridBit();
});
