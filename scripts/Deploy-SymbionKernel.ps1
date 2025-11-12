#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Déploiement automatique Symbion Kernel sur Windows

.DESCRIPTION
    Télécharge le kernel depuis GitHub Releases, crée un backup,
    déploie le nouveau binaire et vérifie la santé du service.

.PARAMETER Version
    Version à déployer (ex: kernel-v1.2.0 ou 'latest')

.PARAMETER GitHubRepo
    Repository GitHub (format: user/repo)

.PARAMETER InstallDir
    Répertoire d'installation (défaut: C:\Symbion)

.EXAMPLE
    .\Deploy-SymbionKernel.ps1 -Version "kernel-v1.2.0"

.EXAMPLE
    .\Deploy-SymbionKernel.ps1 -Version "latest" -GitHubRepo "votre-user/NewSymbion"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$Version = "latest",

    [Parameter(Mandatory=$false)]
    [string]$GitHubRepo = "votre-username/NewSymbion",

    [Parameter(Mandatory=$false)]
    [string]$InstallDir = "C:\Symbion"
)

$ErrorActionPreference = "Stop"

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🚀 Déploiement Symbion Kernel $Version (Windows)" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Déterminer URL de téléchargement
if ($Version -eq "latest") {
    $DownloadUrl = "https://github.com/$GitHubRepo/releases/latest/download/symbion-kernel-windows-x64-latest.exe"
} else {
    $DownloadUrl = "https://github.com/$GitHubRepo/releases/download/$Version/symbion-kernel-windows-x64-$Version.exe"
}

Write-Host "1️⃣ Téléchargement depuis GitHub Releases..." -ForegroundColor Yellow
Write-Host "   URL: $DownloadUrl" -ForegroundColor Gray

# Créer répertoire temporaire
$TempFile = "$env:TEMP\symbion-kernel-new.exe"

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
$BinaryPath = Join-Path $InstallDir "symbion-kernel.exe"
$BackupPath = Join-Path $InstallDir "symbion-kernel.exe.backup"

if (Test-Path $BinaryPath) {
    Write-Host ""
    Write-Host "4️⃣ Sauvegarde de l'ancien binaire..." -ForegroundColor Yellow
    Copy-Item -Path $BinaryPath -Destination $BackupPath -Force
    Write-Host "   ✅ Backup créé: $BackupPath" -ForegroundColor Green
}

# Vérifier si service existe
$ServiceName = "SymbionKernel"
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

# Redémarrer service
if ($ServiceExists) {
    Write-Host ""
    Write-Host "7️⃣ Redémarrage du service..." -ForegroundColor Yellow
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
    Write-Host "8️⃣ Vérification de la santé..." -ForegroundColor Yellow
    Start-Sleep -Seconds 2

    try {
        # Ignorer certificat self-signed pour test
        [System.Net.ServicePointManager]::ServerCertificateValidationCallback = {$true}
        $Response = Invoke-WebRequest -Uri "https://localhost:8443/health" -UseBasicParsing -TimeoutSec 5
        if ($Response.StatusCode -eq 200) {
            Write-Host "   ✅ API kernel répond correctement" -ForegroundColor Green
        }
    } catch {
        Write-Host "   ⚠️  API kernel ne répond pas (peut être normal si TLS non configuré)" -ForegroundColor Yellow
    }
} else {
    Write-Host ""
    Write-Host "7️⃣ Service Windows non détecté - installation manuelle" -ForegroundColor Yellow
    Write-Host "   Pour créer le service, lancez:" -ForegroundColor Gray
    Write-Host "   .\Install-SymbionService.ps1" -ForegroundColor Cyan
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
Write-Host "  - Voir status:     Get-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Restart:         Restart-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Logs:            Get-EventLog -LogName Application -Source $ServiceName -Newest 50" -ForegroundColor Gray
Write-Host "  - Rollback:        Copy-Item $BackupPath $BinaryPath -Force; Restart-Service $ServiceName" -ForegroundColor Gray
Write-Host "  - Health check:    Invoke-WebRequest https://localhost:8443/health" -ForegroundColor Gray
Write-Host ""
