#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Installation service Symbion Agent Windows (NSSM)

.DESCRIPTION
    Installe symbion-agent-host comme service Windows
    en utilisant NSSM (Non-Sucking Service Manager).
    Configure la connexion au kernel central via MQTT.

.PARAMETER InstallDir
    Répertoire contenant le binaire agent (défaut: C:\Symbion)

.PARAMETER KernelHost
    Adresse du kernel central (défaut: 192.168.1.14)

.PARAMETER MqttBroker
    Adresse MQTT broker du kernel (défaut: 192.168.1.14:1883)

.EXAMPLE
    .\Install-SymbionAgentService.ps1 -KernelHost "192.168.1.100"

.EXAMPLE
    .\Install-SymbionAgentService.ps1 -InstallDir "D:\Symbion" -KernelHost "192.168.1.50"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$InstallDir = "C:\Symbion",

    [Parameter(Mandatory=$false)]
    [string]$KernelHost = "192.168.1.14",

    [Parameter(Mandatory=$false)]
    [string]$MqttBroker = "192.168.1.14:1883"
)

$ErrorActionPreference = "Stop"

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🔧 Installation Service Symbion Agent (Windows)" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "   Kernel Central: $KernelHost" -ForegroundColor Gray
Write-Host "   MQTT Broker:    $MqttBroker" -ForegroundColor Gray
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
Write-Host "1️⃣ Vérification du binaire agent..." -ForegroundColor Yellow

$AgentBinary = Join-Path $InstallDir "symbion-agent-host.exe"

if (-not (Test-Path $AgentBinary)) {
    Write-Host "   ❌ Binaire agent non trouvé: $AgentBinary" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Déployez d'abord l'agent avec:" -ForegroundColor Yellow
    Write-Host "   .\Deploy-SymbionAgent.ps1 -KernelHost '$KernelHost'" -ForegroundColor Cyan
    exit 1
}

Write-Host "   ✅ Binaire trouvé: $AgentBinary" -ForegroundColor Green

# Arrêter service existant
$ServiceName = "SymbionAgent"
$Service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($Service) {
    Write-Host ""
    Write-Host "2️⃣ Suppression de l'ancien service..." -ForegroundColor Yellow
    if ($Service.Status -eq "Running") {
        Stop-Service -Name $ServiceName -Force
    }
    & $NssmPath remove $ServiceName confirm | Out-Null
    Write-Host "   ✅ Ancien service supprimé" -ForegroundColor Green
    Start-Sleep -Seconds 2
}

# Installer nouveau service
Write-Host ""
Write-Host "3️⃣ Installation du service Windows..." -ForegroundColor Yellow

& $NssmPath install $ServiceName $AgentBinary | Out-Null
& $NssmPath set $ServiceName AppDirectory $InstallDir | Out-Null
& $NssmPath set $ServiceName DisplayName "Symbion Agent - System Monitoring" | Out-Null
& $NssmPath set $ServiceName Description "Symbion Agent connecté au kernel $KernelHost" | Out-Null
& $NssmPath set $ServiceName Start SERVICE_AUTO_START | Out-Null

# Variables d'environnement (connexion au kernel central)
& $NssmPath set $ServiceName AppEnvironmentExtra "SYMBION_MQTT_BROKER=$MqttBroker" "SYMBION_KERNEL_HOST=$KernelHost" "RUST_LOG=info" | Out-Null

# Restart automatique
& $NssmPath set $ServiceName AppExit Default Restart | Out-Null
& $NssmPath set $ServiceName AppThrottle 15000 | Out-Null  # 15 secondes avant restart

# Logging
$LogDir = Join-Path $InstallDir "logs"
New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
& $NssmPath set $ServiceName AppStdout (Join-Path $LogDir "agent-stdout.log") | Out-Null
& $NssmPath set $ServiceName AppStderr (Join-Path $LogDir "agent-stderr.log") | Out-Null
& $NssmPath set $ServiceName AppRotateFiles 1 | Out-Null
& $NssmPath set $ServiceName AppRotateBytes 10485760 | Out-Null  # 10MB rotation

Write-Host "   ✅ Service configuré" -ForegroundColor Green

# Démarrer service
Write-Host ""
Write-Host "4️⃣ Démarrage du service..." -ForegroundColor Yellow
Start-Service -Name $ServiceName
Start-Sleep -Seconds 3

$Service = Get-Service -Name $ServiceName
if ($Service.Status -eq "Running") {
    Write-Host "   ✅ Service démarré avec succès" -ForegroundColor Green
} else {
    Write-Host "   ❌ Le service n'a pas démarré" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Logs:" -ForegroundColor Yellow
    Get-Content (Join-Path $LogDir "agent-stderr.log") -Tail 20 -ErrorAction SilentlyContinue
    exit 1
}

# Vérifier enregistrement au kernel
Write-Host ""
Write-Host "5️⃣ Vérification de l'enregistrement au kernel..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

try {
    [System.Net.ServicePointManager]::ServerCertificateValidationCallback = {$true}

    $AgentId = $env:COMPUTERNAME
    $Headers = @{
        "x-api-key" = "s3cr3t-42"
    }

    $Response = Invoke-WebRequest -Uri "https://${KernelHost}:8443/agents" -Headers $Headers -UseBasicParsing -TimeoutSec 10
    $Agents = $Response.Content | ConvertFrom-Json

    $ThisAgent = $Agents | Where-Object { $_.agent_id -eq $AgentId }

    if ($ThisAgent) {
        Write-Host "   ✅ Agent enregistré sur kernel central" -ForegroundColor Green
        Write-Host ""
        Write-Host "   📊 Informations agent:" -ForegroundColor Cyan
        Write-Host "      Agent ID:     $($ThisAgent.agent_id)" -ForegroundColor Gray
        Write-Host "      Status:       $($ThisAgent.status.status)" -ForegroundColor Gray
        Write-Host "      Platform:     $($ThisAgent.platform)" -ForegroundColor Gray
        Write-Host "      CPU Usage:    $($ThisAgent.status.cpu_usage)%" -ForegroundColor Gray
        Write-Host "      RAM Usage:    $($ThisAgent.status.memory_usage_mb) MB" -ForegroundColor Gray
    } else {
        Write-Host "   ⚠️  Agent pas encore visible sur kernel (peut prendre 30s)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "   ⚠️  Impossible de vérifier enregistrement" -ForegroundColor Yellow
    Write-Host "   Vérifiez que le kernel est accessible: https://${KernelHost}:8443/health" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "✅ Installation terminée!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host ""

Write-Host "📊 Status service:" -ForegroundColor Cyan
Get-Service -Name $ServiceName | Format-Table -AutoSize

Write-Host ""
Write-Host "📝 Commandes utiles:" -ForegroundColor Cyan
Write-Host "  - Voir status:       Get-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Restart:           Restart-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Stop:              Stop-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Désactiver auto:   Set-Service $ServiceName -StartupType Manual" -ForegroundColor Gray
Write-Host "  - Voir logs:         Get-Content $LogDir\agent-stdout.log -Tail 50 -Wait" -ForegroundColor Gray
Write-Host "  - Config NSSM:       & '$NssmPath' edit $ServiceName" -ForegroundColor Gray
Write-Host ""

Write-Host "🌐 Vérifier le dashboard:" -ForegroundColor Cyan
Write-Host "  http://${KernelHost}:3000" -ForegroundColor Gray
Write-Host ""

# Configurer firewall (optionnel)
Write-Host "🔥 Configurer firewall Windows pour communication kernel? (Y/N): " -ForegroundColor Yellow -NoNewline
$Response = Read-Host

if ($Response -eq "Y" -or $Response -eq "y") {
    try {
        # Permettre connexions sortantes vers kernel HTTPS et MQTT
        New-NetFirewallRule -DisplayName "Symbion Agent → Kernel HTTPS" -Direction Outbound -RemoteAddress $KernelHost -RemotePort 8443 -Protocol TCP -Action Allow -ErrorAction SilentlyContinue | Out-Null
        New-NetFirewallRule -DisplayName "Symbion Agent → MQTT" -Direction Outbound -RemoteAddress $KernelHost -RemotePort 1883 -Protocol TCP -Action Allow -ErrorAction SilentlyContinue | Out-Null
        Write-Host "   ✅ Règles firewall créées" -ForegroundColor Green
    } catch {
        Write-Host "   ⚠️  Erreur création règles firewall" -ForegroundColor Yellow
    }
}

Write-Host ""
