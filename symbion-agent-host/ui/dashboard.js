// Dashboard local de l'agent Symbion
class AgentDashboard {
    constructor() {
        this.updateInterval = null;
        this.connected = false;
        this.init();
    }

    async init() {
        console.log('[Dashboard] Initializing...');
        this.startPeriodicUpdate();
        await this.updateMetrics();
    }

    startPeriodicUpdate() {
        // Update métriques toutes les 3 secondes
        this.updateInterval = setInterval(() => {
            this.updateMetrics();
        }, 3000);
    }

    async updateMetrics() {
        try {
            const response = await fetch('http://localhost:9899/status');
            const data = await response.json();
            
            this.updateUI(data);
            this.setConnectionStatus(true);
        } catch (error) {
            console.warn('[Dashboard] Failed to fetch metrics:', error);
            this.setConnectionStatus(false);
        }
    }

    updateUI(data) {
        // Hostname
        document.getElementById('hostname').textContent = data.hostname || 'Unknown';
        
        // Status
        document.getElementById('status').textContent = data.mqtt_connected ? 'Connected' : 'Disconnected';
        
        // Métriques système
        if (data.system) {
            document.getElementById('cpu').textContent = data.system.cpu_percent?.toFixed(1) + '%' || '---%';
            document.getElementById('memory').textContent = data.system.memory_used_mb || '--- MB';
            document.getElementById('processes').textContent = data.system.process_count || '---';
        }
        
        // Uptime
        if (data.uptime_seconds) {
            document.getElementById('uptime').textContent = this.formatUptime(data.uptime_seconds);
        }
    }

    setConnectionStatus(connected) {
        this.connected = connected;
        const statusDot = document.getElementById('status-dot');
        const statusText = document.getElementById('status');
        
        if (connected) {
            statusDot.className = 'status-dot';
            statusText.textContent = 'Connected';
        } else {
            statusDot.className = 'status-dot offline';
            statusText.textContent = 'Offline';
        }
    }

    formatUptime(seconds) {
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        
        if (hours > 0) {
            return `${hours}h ${minutes}m`;
        } else {
            return `${minutes}m`;
        }
    }

    cleanup() {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
        }
    }
}

// Actions disponibles
async function openMainPWA() {
    try {
        // Essayer de détecter l'IP du kernel automatiquement
        const kernelUrl = await detectKernelUrl();
        window.__TAURI__?.shell.open(kernelUrl);
    } catch (error) {
        // Fallback localhost
        window.__TAURI__?.shell.open('http://localhost:3000');
    }
}

async function detectKernelUrl() {
    // Essayer quelques IPs communes pour le kernel
    const candidates = [
        'http://localhost:3000',
        'http://192.168.1.100:3000', // IP commune du kernel
        'http://192.168.1.10:3000'
    ];
    
    for (const url of candidates) {
        try {
            const response = await fetch(url, { method: 'HEAD', timeout: 1000 });
            if (response.ok) {
                return url;
            }
        } catch (e) {
            continue;
        }
    }
    
    return 'http://localhost:3000'; // Fallback
}

function showLogs() {
    // Ouvrir les logs dans l'éditeur par défaut
    window.__TAURI__?.shell.open('file:///var/log/symbion/agent.log');
}

async function reconnect() {
    document.getElementById('status').textContent = 'Reconnecting...';
    
    try {
        const response = await fetch('http://localhost:9899/reconnect', { method: 'POST' });
        if (response.ok) {
            setTimeout(() => {
                dashboard.updateMetrics();
            }, 2000);
        }
    } catch (error) {
        console.warn('[Dashboard] Reconnect failed:', error);
    }
}

// Initialize dashboard when page loads
let dashboard;
document.addEventListener('DOMContentLoaded', () => {
    dashboard = new AgentDashboard();
});

// Cleanup on unload
window.addEventListener('beforeunload', () => {
    if (dashboard) {
        dashboard.cleanup();
    }
});