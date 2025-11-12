#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Installation services Windows Symbion avec NSSM

.DESCRIPTION
    Installe symbion-kernel et symbion-agent comme services Windows
    utilisant NSSM (Non-Sucking Service Manager) pour supervision.

.PARAMETER InstallDir
    Répertoire d'installation des binaires (défaut: C:\Symbion)

.PARAMETER ApiKey
    Clé API Symbion (défaut: s3cr3t-42)

.PARAMETER MqttBroker
    Adresse MQTT broker (défaut: 127.0.0.1:1883)

.EXAMPLE
    .\Install-SymbionService.ps1

.EXAMPLE
    .\Install-SymbionService.ps1 -InstallDir "D:\Services\Symbion" -ApiKey "prod-key-123"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$InstallDir = "C:\Symbion",

    [Parameter(Mandatory=$false)]
    [string]$ApiKey = "s3cr3t-42",

    [Parameter(Mandatory=$false)]
    [string]$MqttBroker = "127.0.0.1:1883",

    [Parameter(Mandatory=$false)]
    [string]$JwtSecret = "test-secret-1234567890123456789012345678901234567890123456789012345678901234"
)

$ErrorActionPreference = "Stop"

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🔧 Installation Services Symbion Windows" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Vérifier si NSSM est installé
$NssmPath = "C:\Tools\nssm\nssm.exe"

if (-not (Test-Path $NssmPath)) {
    Write-Host "📥 NSSM non trouvé, téléchargement..." -ForegroundColor Yellow

    # Télécharger NSSM
    $NssmUrl = "https://nssm.cc/release/nssm-2.24.zip"
    $NssmZip = "$env:TEMP\nssm.zip"
    $NssmExtract = "$env:TEMP\nssm"

    try {
        Invoke-WebRequest -Uri $NssmUrl -OutFile $NssmZip -UseBasicParsing
        Expand-Archive -Path $NssmZip -DestinationPath $NssmExtract -Force

        # Copier nssm.exe vers C:\Tools
        New-Item -ItemType Directory -Path "C:\Tools\nssm" -Force | Out-Null
        Copy-Item -Path "$NssmExtract\nssm-2.24\win64\nssm.exe" -Destination $NssmPath -Force

        Write-Host "   ✅ NSSM installé: $NssmPath" -ForegroundColor Green
    } catch {
        Write-Host "   ❌ Échec téléchargement NSSM" -ForegroundColor Red
        Write-Host "   Installez manuellement depuis: https://nssm.cc/download" -ForegroundColor Yellow
        exit 1
    }
}

Write-Host ""
Write-Host "1️⃣ Arrêt des services existants..." -ForegroundColor Yellow

# Arrêter services s'ils existent
$Services = @("SymbionKernel", "SymbionAgent")
foreach ($ServiceName in $Services) {
    $Service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($Service) {
        if ($Service.Status -eq "Running") {
            Stop-Service -Name $ServiceName -Force
            Write-Host "   ✅ Service $ServiceName arrêté" -ForegroundColor Green
        }

        # Supprimer ancien service
        & $NssmPath remove $ServiceName confirm | Out-Null
        Write-Host "   ✅ Ancien service $ServiceName supprimé" -ForegroundColor Green
    }
}

Start-Sleep -Seconds 2

# Vérifier binaires
$KernelBinary = Join-Path $InstallDir "symbion-kernel.exe"
$AgentBinary = Join-Path $InstallDir "symbion-agent-host.exe"

if (-not (Test-Path $KernelBinary)) {
    Write-Host ""
    Write-Host "⚠️  Binaire kernel non trouvé: $KernelBinary" -ForegroundColor Yellow
    Write-Host "   Téléchargez d'abord avec: .\Deploy-SymbionKernel.ps1" -ForegroundColor Cyan
    $SkipKernel = $true
} else {
    $SkipKernel = $false
}

if (-not (Test-Path $AgentBinary)) {
    Write-Host ""
    Write-Host "⚠️  Binaire agent non trouvé: $AgentBinary" -ForegroundColor Yellow
    Write-Host "   Compilez d'abord: cargo build --release -p symbion-agent-host" -ForegroundColor Cyan
    $SkipAgent = $true
} else {
    $SkipAgent = $false
}

# Installer service Kernel
if (-not $SkipKernel) {
    Write-Host ""
    Write-Host "2️⃣ Installation service SymbionKernel..." -ForegroundColor Yellow

    & $NssmPath install SymbionKernel $KernelBinary | Out-Null
    & $NssmPath set SymbionKernel AppDirectory $InstallDir | Out-Null
    & $NssmPath set SymbionKernel DisplayName "Symbion Kernel - IoT Hub" | Out-Null
    & $NssmPath set SymbionKernel Description "Symbion IoT Home Automation Kernel - Central Hub" | Out-Null
    & $NssmPath set SymbionKernel Start SERVICE_AUTO_START | Out-Null

    # Variables d'environnement
    & $NssmPath set SymbionKernel AppEnvironmentExtra "SYMBION_API_KEY=$ApiKey" "SYMBION_MQTT_BROKER=$MqttBroker" "SYMBION_JWT_SECRET=$JwtSecret" "RUST_LOG=info" | Out-Null

    # Restart automatique
    & $NssmPath set SymbionKernel AppExit Default Restart | Out-Null
    & $NssmPath set SymbionKernel AppThrottle 10000 | Out-Null  # 10 secondes avant restart

    # Logging
    $LogDir = Join-Path $InstallDir "logs"
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
    & $NssmPath set SymbionKernel AppStdout (Join-Path $LogDir "kernel-stdout.log") | Out-Null
    & $NssmPath set SymbionKernel AppStderr (Join-Path $LogDir "kernel-stderr.log") | Out-Null
    & $NssmPath set SymbionKernel AppRotateFiles 1 | Out-Null
    & $NssmPath set SymbionKernel AppRotateBytes 10485760 | Out-Null  # 10MB rotation

    Write-Host "   ✅ Service SymbionKernel configuré" -ForegroundColor Green
}

# Installer service Agent
if (-not $SkipAgent) {
    Write-Host ""
    Write-Host "3️⃣ Installation service SymbionAgent..." -ForegroundColor Yellow

    & $NssmPath install SymbionAgent $AgentBinary | Out-Null
    & $NssmPath set SymbionAgent AppDirectory $InstallDir | Out-Null
    & $NssmPath set SymbionAgent DisplayName "Symbion Agent - System Monitoring" | Out-Null
    & $NssmPath set SymbionAgent Description "Symbion Agent Host - Local System Monitoring" | Out-Null
    & $NssmPath set SymbionAgent Start SERVICE_AUTO_START | Out-Null

    # Variables d'environnement
    & $NssmPath set SymbionAgent AppEnvironmentExtra "SYMBION_MQTT_BROKER=$MqttBroker" "RUST_LOG=info" | Out-Null

    # Dépendance: agent démarre après kernel
    & $NssmPath set SymbionAgent DependOnService SymbionKernel | Out-Null

    # Restart automatique
    & $NssmPath set SymbionAgent AppExit Default Restart | Out-Null
    & $NssmPath set SymbionAgent AppThrottle 15000 | Out-Null  # 15 secondes avant restart

    # Logging
    & $NssmPath set SymbionAgent AppStdout (Join-Path $LogDir "agent-stdout.log") | Out-Null
    & $NssmPath set SymbionAgent AppStderr (Join-Path $LogDir "agent-stderr.log") | Out-Null
    & $NssmPath set SymbionAgent AppRotateFiles 1 | Out-Null
    & $NssmPath set SymbionAgent AppRotateBytes 10485760 | Out-Null

    Write-Host "   ✅ Service SymbionAgent configuré" -ForegroundColor Green
}

# Démarrer services
Write-Host ""
Write-Host "4️⃣ Démarrage des services..." -ForegroundColor Yellow

if (-not $SkipKernel) {
    Start-Service -Name SymbionKernel
    Start-Sleep -Seconds 3
    Write-Host "   ✅ SymbionKernel démarré" -ForegroundColor Green
}

if (-not $SkipAgent) {
    Start-Service -Name SymbionAgent
    Start-Sleep -Seconds 2
    Write-Host "   ✅ SymbionAgent démarré" -ForegroundColor Green
}

# Vérifier status
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "✅ Installation terminée!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host ""

Write-Host "📊 Status des services:" -ForegroundColor Cyan
Get-Service -Name SymbionKernel,SymbionAgent -ErrorAction SilentlyContinue | Format-Table -AutoSize

Write-Host ""
Write-Host "📝 Commandes utiles:" -ForegroundColor Cyan
Write-Host "  - Voir status:       Get-Service SymbionKernel,SymbionAgent" -ForegroundColor Gray
Write-Host "  - Restart kernel:    Restart-Service SymbionKernel" -ForegroundColor Gray
Write-Host "  - Restart agent:     Restart-Service SymbionAgent" -ForegroundColor Gray
Write-Host "  - Stop tout:         Stop-Service SymbionKernel,SymbionAgent" -ForegroundColor Gray
Write-Host "  - Désactiver auto:   Set-Service SymbionKernel -StartupType Manual" -ForegroundColor Gray
Write-Host "  - Voir logs:         Get-Content $LogDir\kernel-stdout.log -Tail 50" -ForegroundColor Gray
Write-Host "  - Config NSSM:       & '$NssmPath' edit SymbionKernel" -ForegroundColor Gray
Write-Host ""

# Configurer firewall
Write-Host "🔥 Configuration firewall Windows..." -ForegroundColor Yellow
Write-Host "   Souhaitez-vous ouvrir les ports nécessaires? (Y/N): " -ForegroundColor Cyan -NoNewline
$Response = Read-Host

if ($Response -eq "Y" -or $Response -eq "y") {
    try {
        New-NetFirewallRule -DisplayName "Symbion Kernel HTTPS" -Direction Inbound -LocalPort 8443 -Protocol TCP -Action Allow -ErrorAction SilentlyContinue | Out-Null
        New-NetFirewallRule -DisplayName "Symbion MQTT" -Direction Inbound -LocalPort 1883 -Protocol TCP -Action Allow -ErrorAction SilentlyContinue | Out-Null
        Write-Host "   ✅ Règles firewall créées (ports 8443, 1883)" -ForegroundColor Green
    } catch {
        Write-Host "   ⚠️  Erreur création règles firewall" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "🌐 Health check:" -ForegroundColor Cyan
Start-Sleep -Seconds 2

try {
    [System.Net.ServicePointManager]::ServerCertificateValidationCallback = {$true}
    $Response = Invoke-WebRequest -Uri "https://localhost:8443/health" -UseBasicParsing -TimeoutSec 5
    if ($Response.StatusCode -eq 200) {
        Write-Host "   ✅ Kernel API accessible: https://localhost:8443" -ForegroundColor Green
    }
} catch {
    Write-Host "   ⚠️  Kernel API non accessible (peut prendre quelques secondes au démarrage)" -ForegroundColor Yellow
}

Write-Host ""
