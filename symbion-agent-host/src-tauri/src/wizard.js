// Symbion Agent Setup Wizard
// Frontend JavaScript integration with Tauri backend

const { invoke } = window.__TAURI__.core;

class SetupWizard {
    constructor() {
        this.currentStep = 0;
        this.totalSteps = 5;
        this.steps = ['welcome', 'mqtt', 'elevation', 'updates', 'summary'];
        this.config = {};
        this.systemInfo = {};
        this.updateInfo = null;
        
        this.init();
    }
    
    async init() {
        // Load initial state
        try {
            const state = await invoke('get_setup_state');
            this.config = state.config;
            this.currentStep = state.current_step;
            
            // Load system info
            this.systemInfo = await invoke('get_system_info');
            this.populateSystemInfo();
            
            // Setup event listeners
            this.setupEventListeners();
            
            // Update UI
            this.updateStepDisplay();
            this.updateProgress();
            
            console.log('Setup wizard initialized', { state, systemInfo: this.systemInfo });
        } catch (error) {
            console.error('Failed to initialize wizard:', error);
            this.showError('Erreur d\'initialisation: ' + error);
        }
    }
    
    setupEventListeners() {
        // Navigation buttons
        document.getElementById('next-btn').addEventListener('click', () => this.nextStep());
        document.getElementById('prev-btn').addEventListener('click', () => this.prevStep());
        
        // MQTT test connection
        document.getElementById('test-mqtt-btn').addEventListener('click', () => this.testMqttConnection());
        
        // Elevation checkboxes
        document.getElementById('store-credentials').addEventListener('change', (e) => {
            const passwordGroup = document.getElementById('password-group');
            passwordGroup.style.display = e.target.checked ? 'block' : 'none';
        });
        
        // Load current values
        this.loadCurrentValues();
    }
    
    loadCurrentValues() {
        // MQTT
        document.getElementById('mqtt-host').value = this.config.mqtt.broker_host;
        document.getElementById('mqtt-port').value = this.config.mqtt.broker_port;
        document.getElementById('mqtt-client-id').value = this.config.mqtt.client_id || '';
        
        // Elevation
        document.getElementById('store-credentials').checked = this.config.elevation.store_credentials;
        document.getElementById('auto-elevate').checked = this.config.elevation.auto_elevate;
        
        // Updates
        document.getElementById('auto-update').checked = this.config.update.auto_update;
        document.getElementById('update-channel').value = this.config.update.channel.toLowerCase();
        document.getElementById('check-interval').value = this.config.update.check_interval_hours;
        document.getElementById('github-repo').value = this.config.update.github_repo;
    }
    
    populateSystemInfo() {
        document.getElementById('system-os').textContent = this.systemInfo.os || 'Unknown';
        document.getElementById('system-arch').textContent = this.systemInfo.arch || 'Unknown';
        document.getElementById('system-hostname').textContent = this.systemInfo.hostname || 'Unknown';
        document.getElementById('system-version').textContent = this.systemInfo.version || 'Unknown';
    }
    
    async nextStep() {
        if (await this.validateCurrentStep()) {
            if (this.currentStep < this.totalSteps - 1) {
                this.currentStep++;
                this.updateStepDisplay();
                this.updateProgress();
                
                // Special handling for steps that need data loading
                if (this.steps[this.currentStep] === 'updates') {
                    await this.checkForUpdates();
                } else if (this.steps[this.currentStep] === 'summary') {
                    this.populateSummary();
                }
            } else {
                // Finalize setup
                await this.finalizeSetup();
            }
        }
    }
    
    prevStep() {
        if (this.currentStep > 0) {
            this.currentStep--;
            this.updateStepDisplay();
            this.updateProgress();
        }
    }
    
    updateStepDisplay() {
        // Hide all steps
        document.querySelectorAll('.step').forEach(step => {
            step.classList.remove('active');
        });
        
        // Show current step
        const currentStepElement = document.querySelector(`[data-step="${this.steps[this.currentStep]}"]`);
        if (currentStepElement) {
            currentStepElement.classList.add('active');
        }
        
        // Update navigation buttons
        const prevBtn = document.getElementById('prev-btn');
        const nextBtn = document.getElementById('next-btn');
        
        prevBtn.style.display = this.currentStep > 0 ? 'block' : 'none';
        
        if (this.currentStep === this.totalSteps - 1) {
            nextBtn.textContent = '✅ Finaliser la configuration';
            nextBtn.className = 'btn btn-primary';
        } else {
            nextBtn.textContent = 'Suivant ➡️';
            nextBtn.className = 'btn btn-primary';
        }
        
        // Update step indicator
        document.getElementById('step-indicator').textContent = `Étape ${this.currentStep + 1} sur ${this.totalSteps}`;
    }
    
    updateProgress() {
        const progress = ((this.currentStep) / (this.totalSteps - 1)) * 100;
        document.getElementById('progress-fill').style.width = progress + '%';
    }
    
    async validateCurrentStep() {
        const stepName = this.steps[this.currentStep];
        
        switch (stepName) {
            case 'welcome':
                return true; // No validation needed
                
            case 'mqtt':
                return await this.validateMqttStep();
                
            case 'elevation':
                return this.validateElevationStep();
                
            case 'updates':
                return this.validateUpdatesStep();
                
            case 'summary':
                return true; // No validation needed
                
            default:
                return true;
        }
    }
    
    async validateMqttStep() {
        const host = document.getElementById('mqtt-host').value.trim();
        const port = parseInt(document.getElementById('mqtt-port').value);
        const clientId = document.getElementById('mqtt-client-id').value.trim() || null;
        
        if (!host) {
            this.showError('L\'adresse du broker MQTT est requise');
            return false;
        }
        
        if (!port || port < 1 || port > 65535) {
            this.showError('Le port MQTT doit être entre 1 et 65535');
            return false;
        }
        
        // Save MQTT config
        try {
            await invoke('save_mqtt_config', { 
                brokerHost: host, 
                brokerPort: port, 
                clientId: clientId 
            });
            console.log('MQTT config saved');
            return true;
        } catch (error) {
            this.showError('Erreur lors de la sauvegarde: ' + error);
            return false;
        }
    }
    
    validateElevationStep() {
        const storeCredentials = document.getElementById('store-credentials').checked;
        const autoElevate = document.getElementById('auto-elevate').checked;
        const password = document.getElementById('elevation-password').value;
        
        if (storeCredentials && !password) {
            this.showError('Mot de passe requis si stockage des credentials activé');
            return false;
        }
        
        // Save elevation config
        invoke('save_elevation_config', {
            storeCredentials: storeCredentials,
            autoElevate: autoElevate,
            password: storeCredentials ? password : null
        }).then(() => {
            console.log('Elevation config saved');
        }).catch(error => {
            console.error('Failed to save elevation config:', error);
        });
        
        return true;
    }
    
    validateUpdatesStep() {
        const autoUpdate = document.getElementById('auto-update').checked;
        const channel = document.getElementById('update-channel').value;
        const checkInterval = parseInt(document.getElementById('check-interval').value);
        const githubRepo = document.getElementById('github-repo').value.trim();
        
        if (!githubRepo.includes('/')) {
            this.showError('Repository GitHub doit être au format "owner/repository"');
            return false;
        }
        
        // Save update config
        invoke('save_update_config', {
            autoUpdate: autoUpdate,
            channel: channel,
            checkIntervalHours: checkInterval,
            githubRepo: githubRepo
        }).then(() => {
            console.log('Update config saved');
        }).catch(error => {
            console.error('Failed to save update config:', error);
        });
        
        return true;
    }
    
    async testMqttConnection() {
        const btn = document.getElementById('test-mqtt-btn');
        const result = document.getElementById('mqtt-test-result');
        const host = document.getElementById('mqtt-host').value.trim();
        const port = parseInt(document.getElementById('mqtt-port').value);
        
        btn.classList.add('loading');
        btn.disabled = true;
        result.style.display = 'none';
        
        try {
            const success = await invoke('test_mqtt_connection', { 
                brokerHost: host, 
                brokerPort: port 
            });
            
            if (success) {
                result.className = 'test-result success';
                result.textContent = '✅ Connexion réussie au broker MQTT';
            } else {
                result.className = 'test-result error';
                result.textContent = '❌ Connexion échouée';
            }
        } catch (error) {
            result.className = 'test-result error';
            result.textContent = '❌ Erreur: ' + error;
        }
        
        result.style.display = 'block';
        btn.classList.remove('loading');
        btn.disabled = false;
    }
    
    async checkForUpdates() {
        const infoDiv = document.getElementById('update-check-info');
        
        try {
            this.updateInfo = await invoke('check_for_updates');
            
            if (this.updateInfo.is_update_available) {
                infoDiv.className = 'update-info available';
                infoDiv.innerHTML = `
                    <strong>🎉 Mise à jour disponible!</strong><br>
                    Version actuelle: ${this.updateInfo.current_version}<br>
                    Nouvelle version: ${this.updateInfo.latest_version}<br>
                    ${this.updateInfo.is_critical ? '<span style="color: #d63031;">⚠️ Mise à jour critique</span>' : ''}
                `;
            } else {
                infoDiv.className = 'update-info';
                infoDiv.innerHTML = `
                    <strong>✅ Agent à jour</strong><br>
                    Version actuelle: ${this.updateInfo.current_version}
                `;
            }
        } catch (error) {
            infoDiv.className = 'update-info error';
            infoDiv.innerHTML = `<strong>❌ Erreur de vérification:</strong> ${error}`;
        }
    }
    
    populateSummary() {
        const summaryContent = document.getElementById('summary-content');
        
        // Get current form values
        const mqttHost = document.getElementById('mqtt-host').value;
        const mqttPort = document.getElementById('mqtt-port').value;
        const storeCredentials = document.getElementById('store-credentials').checked;
        const autoElevate = document.getElementById('auto-elevate').checked;
        const autoUpdate = document.getElementById('auto-update').checked;
        const updateChannel = document.getElementById('update-channel').value;
        
        summaryContent.innerHTML = `
            <div class="summary-section">
                <h3>📡 Configuration MQTT</h3>
                <div class="detail">Broker: ${mqttHost}:${mqttPort}</div>
                <div class="detail">Client ID: ${document.getElementById('mqtt-client-id').value || 'Auto-généré'}</div>
            </div>
            
            <div class="summary-section">
                <h3>🔐 Privilèges système</h3>
                <div class="detail">Stockage credentials: ${storeCredentials ? '✅ Activé' : '❌ Désactivé'}</div>
                <div class="detail">Élévation auto: ${autoElevate ? '✅ Activé' : '❌ Désactivé'}</div>
            </div>
            
            <div class="summary-section">
                <h3>🔄 Mises à jour</h3>
                <div class="detail">Auto-update: ${autoUpdate ? '✅ Activé' : '❌ Désactivé'}</div>
                <div class="detail">Canal: ${updateChannel}</div>
                <div class="detail">Fréquence: ${document.getElementById('check-interval').value}h</div>
            </div>
            
            <div class="summary-section">
                <h3>🖥️ Système</h3>
                <div class="detail">OS: ${this.systemInfo.os} (${this.systemInfo.arch})</div>
                <div class="detail">Hostname: ${this.systemInfo.hostname}</div>
                <div class="detail">Version: ${this.systemInfo.version}</div>
            </div>
        `;
    }
    
    async finalizeSetup() {
        const nextBtn = document.getElementById('next-btn');
        nextBtn.classList.add('loading');
        nextBtn.disabled = true;
        
        try {
            // All configurations have already been saved in individual steps
            console.log('Setup completed successfully');
            
            // Show success message
            document.querySelector('.wizard-body').innerHTML = `
                <div style="text-align: center; padding: 40px;">
                    <h2 style="color: #00b894; margin-bottom: 20px;">✅ Configuration terminée!</h2>
                    <p style="margin-bottom: 30px;">L'agent Symbion est maintenant configuré et prêt à fonctionner.</p>
                    
                    <div style="background: #d1f2eb; padding: 20px; border-radius: 8px; margin-bottom: 20px;">
                        <strong>🚀 Prochaines étapes:</strong><br>
                        • L'agent va se connecter automatiquement au broker MQTT<br>
                        • Il apparaîtra dans le dashboard PWA Symbion<br>
                        • Les mises à jour automatiques sont ${document.getElementById('auto-update').checked ? 'activées' : 'désactivées'}
                    </div>
                    
                    <button class="btn btn-primary" onclick="window.close()">🏁 Fermer le wizard</button>
                </div>
            `;
            
            // Hide footer
            document.querySelector('.wizard-footer').style.display = 'none';
            
        } catch (error) {
            this.showError('Erreur lors de la finalisation: ' + error);
            nextBtn.classList.remove('loading');
            nextBtn.disabled = false;
        }
    }
    
    showError(message) {
        // Create or update error display
        let errorDiv = document.getElementById('wizard-error');
        if (!errorDiv) {
            errorDiv = document.createElement('div');
            errorDiv.id = 'wizard-error';
            errorDiv.style.cssText = `
                background: #f8d7da;
                color: #721c24;
                border: 1px solid #f5c6cb;
                padding: 15px;
                border-radius: 6px;
                margin-bottom: 20px;
            `;
            document.querySelector('.wizard-body').insertBefore(errorDiv, document.querySelector('.step.active'));
        }
        
        errorDiv.innerHTML = `<strong>❌ Erreur:</strong> ${message}`;
        errorDiv.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        
        // Auto-hide after 5 seconds
        setTimeout(() => {
            if (errorDiv && errorDiv.parentNode) {
                errorDiv.remove();
            }
        }, 5000);
    }
}

// Initialize wizard when page loads
document.addEventListener('DOMContentLoaded', () => {
    new SetupWizard();
});