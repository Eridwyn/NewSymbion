#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Déploiement automatique Symbion Agent sur Windows

.DESCRIPTION
    Télécharge l'agent depuis GitHub Releases, configure les variables
    d'environnement pour se connecter au kernel central, et installe
    le service Windows.

.PARAMETER Version
    Version à déployer (ex: agent-v1.0.0 ou 'latest')

.PARAMETER GitHubRepo
    Repository GitHub (format: user/repo)

.PARAMETER InstallDir
    Répertoire d'installation (défaut: C:\Symbion)

.PARAMETER KernelHost
    Adresse du kernel central (défaut: 192.168.1.14)

.PARAMETER MqttBroker
    Adresse MQTT broker du kernel (défaut: 192.168.1.14:1883)

.EXAMPLE
    .\Deploy-SymbionAgent.ps1 -Version "agent-v1.0.0" -KernelHost "192.168.1.100"

.EXAMPLE
    .\Deploy-SymbionAgent.ps1 -Version "latest"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$Version = "latest",

    [Parameter(Mandatory=$false)]
    [string]$GitHubRepo = "votre-username/NewSymbion",

    [Parameter(Mandatory=$false)]
    [string]$InstallDir = "C:\Symbion",

    [Parameter(Mandatory=$false)]
    [string]$KernelHost = "192.168.1.14",

    [Parameter(Mandatory=$false)]
    [string]$MqttBroker = "192.168.1.14:1883"
)

$ErrorActionPreference = "Stop"

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🤖 Déploiement Symbion Agent $Version (Windows)" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "   Kernel Central: $KernelHost" -ForegroundColor Gray
Write-Host "   MQTT Broker:    $MqttBroker" -ForegroundColor Gray
Write-Host ""

# Déterminer URL de téléchargement
if ($Version -eq "latest") {
    $DownloadUrl = "https://github.com/$GitHubRepo/releases/latest/download/symbion-agent-windows-x64-latest.exe"
} else {
    $DownloadUrl = "https://github.com/$GitHubRepo/releases/download/$Version/symbion-agent-windows-x64-$Version.exe"
}

Write-Host "1️⃣ Téléchargement depuis GitHub Releases..." -ForegroundColor Yellow
Write-Host "   URL: $DownloadUrl" -ForegroundColor Gray

# Créer répertoire temporaire
$TempFile = "$env:TEMP\symbion-agent-new.exe"

try {
    # Télécharger nouveau binaire
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile -UseBasicParsing
    Write-Host "   ✅ Téléchargement réussi" -ForegroundColor Green
} catch {
    Write-Host "   ❌ Échec du téléchargement" -ForegroundColor Red
    Write-Host "   Vérifiez que la version existe: https://github.com/$GitHubRepo/releases" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "2️⃣ Vérification du binaire..." -ForegroundColor Yellow

# Calculer hash
$FileHash = (Get-FileHash -Path $TempFile -Algorithm SHA256).Hash
Write-Host "   SHA256: $FileHash" -ForegroundColor Gray

# Créer répertoire d'installation si nécessaire
if (-not (Test-Path $InstallDir)) {
    Write-Host ""
    Write-Host "3️⃣ Création du répertoire d'installation..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Write-Host "   ✅ Répertoire créé: $InstallDir" -ForegroundColor Green
}

# Backup ancien binaire si existe
$BinaryPath = Join-Path $InstallDir "symbion-agent-host.exe"
$BackupPath = Join-Path $InstallDir "symbion-agent-host.exe.backup"

if (Test-Path $BinaryPath) {
    Write-Host ""
    Write-Host "4️⃣ Sauvegarde de l'ancien binaire..." -ForegroundColor Yellow
    Copy-Item -Path $BinaryPath -Destination $BackupPath -Force
    Write-Host "   ✅ Backup créé: $BackupPath" -ForegroundColor Green
}

# Vérifier si service existe
$ServiceName = "SymbionAgent"
$ServiceExists = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($ServiceExists) {
    Write-Host ""
    Write-Host "5️⃣ Service Windows détecté, arrêt en cours..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    Write-Host "   ✅ Service arrêté" -ForegroundColor Green
}

# Déployer nouveau binaire
Write-Host ""
Write-Host "6️⃣ Installation du nouveau binaire..." -ForegroundColor Yellow
Move-Item -Path $TempFile -Destination $BinaryPath -Force
Write-Host "   ✅ Binaire installé: $BinaryPath" -ForegroundColor Green

# Créer fichier de configuration
Write-Host ""
Write-Host "7️⃣ Configuration de connexion au kernel central..." -ForegroundColor Yellow

$ConfigContent = @"
# Configuration Symbion Agent
# Ce fichier est chargé automatiquement par le service

SYMBION_MQTT_BROKER=$MqttBroker
SYMBION_KERNEL_HOST=$KernelHost
RUST_LOG=info

# ID Agent (auto-généré au premier lancement)
# SYMBION_AGENT_ID=
"@

$ConfigPath = Join-Path $InstallDir "agent-config.env"
Set-Content -Path $ConfigPath -Value $ConfigContent
Write-Host "   ✅ Configuration créée: $ConfigPath" -ForegroundColor Green

# Redémarrer service
if ($ServiceExists) {
    Write-Host ""
    Write-Host "8️⃣ Redémarrage du service..." -ForegroundColor Yellow

    # Mettre à jour variables d'environnement du service si NSSM
    $NssmPath = "C:\Tools\nssm\nssm.exe"
    if (Test-Path $NssmPath) {
        & $NssmPath set $ServiceName AppEnvironmentExtra "SYMBION_MQTT_BROKER=$MqttBroker" "RUST_LOG=info" | Out-Null
        Write-Host "   ✅ Configuration service mise à jour" -ForegroundColor Green
    }

    Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 3

    # Vérifier status
    $Service = Get-Service -Name $ServiceName
    if ($Service.Status -eq "Running") {
        Write-Host "   ✅ Service actif" -ForegroundColor Green
    } else {
        Write-Host "   ❌ Service failed to start" -ForegroundColor Red
        Write-Host ""
        Write-Host "   📋 Logs (Event Viewer):" -ForegroundColor Yellow
        Get-EventLog -LogName Application -Source $ServiceName -Newest 10 -ErrorAction SilentlyContinue | Format-Table -AutoSize

        Write-Host ""
        Write-Host "   🔄 Rollback automatique..." -ForegroundColor Yellow
        if (Test-Path $BackupPath) {
            Copy-Item -Path $BackupPath -Destination $BinaryPath -Force
            Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
            Write-Host "   ✅ Rollback effectué vers ancienne version" -ForegroundColor Green
        }
        exit 1
    }

    Write-Host ""
    Write-Host "9️⃣ Vérification de l'enregistrement au kernel..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5

    try {
        # Vérifier que l'agent est enregistré auprès du kernel
        [System.Net.ServicePointManager]::ServerCertificateValidationCallback = {$true}

        # Récupérer nom de la machine (agent ID)
        $AgentId = $env:COMPUTERNAME

        $Headers = @{
            "x-api-key" = "s3cr3t-42"
        }

        $Response = Invoke-WebRequest -Uri "https://${KernelHost}:8443/agents" -Headers $Headers -UseBasicParsing -TimeoutSec 10
        $Agents = $Response.Content | ConvertFrom-Json

        $ThisAgent = $Agents | Where-Object { $_.agent_id -eq $AgentId }

        if ($ThisAgent) {
            Write-Host "   ✅ Agent enregistré sur kernel central" -ForegroundColor Green
            Write-Host "   Agent ID: $($ThisAgent.agent_id)" -ForegroundColor Gray
            Write-Host "   Status: $($ThisAgent.status.status)" -ForegroundColor Gray
        } else {
            Write-Host "   ⚠️  Agent pas encore visible sur kernel (peut prendre 30s)" -ForegroundColor Yellow
            Write-Host "   Vérifiez dans 1 minute avec: Invoke-WebRequest https://${KernelHost}:8443/agents" -ForegroundColor Cyan
        }
    } catch {
        Write-Host "   ⚠️  Impossible de vérifier enregistrement (kernel inaccessible)" -ForegroundColor Yellow
        Write-Host "   Vérifiez que le kernel est accessible: https://${KernelHost}:8443/health" -ForegroundColor Cyan
    }
} else {
    Write-Host ""
    Write-Host "8️⃣ Service Windows non détecté - installation manuelle" -ForegroundColor Yellow
    Write-Host "   Pour créer le service, lancez:" -ForegroundColor Gray
    Write-Host "   .\Install-SymbionAgentService.ps1 -KernelHost '$KernelHost' -MqttBroker '$MqttBroker'" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "✅ Déploiement réussi!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host ""

if ($ServiceExists) {
    Write-Host "📊 Status service:" -ForegroundColor Cyan
    Get-Service -Name $ServiceName | Format-Table -AutoSize
}

Write-Host ""
Write-Host "📝 Commandes utiles:" -ForegroundColor Cyan
Write-Host "  - Voir status:       Get-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Restart:           Restart-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Logs agent:        Get-Content $InstallDir\logs\agent-stdout.log -Tail 50" -ForegroundColor Gray
Write-Host "  - Rollback:          Copy-Item $BackupPath $BinaryPath -Force; Restart-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Test connexion:    Invoke-WebRequest https://${KernelHost}:8443/agents" -ForegroundColor Gray
Write-Host ""

Write-Host "🌐 Prochaines étapes:" -ForegroundColor Cyan
Write-Host "  1. Vérifier que l'agent apparaît sur le dashboard: http://${KernelHost}:3000" -ForegroundColor Gray
Write-Host "  2. Vérifier les métriques temps réel (CPU/RAM)" -ForegroundColor Gray
Write-Host "  3. Tester contrôles système (shutdown/hibernate)" -ForegroundColor Gray
Write-Host ""
