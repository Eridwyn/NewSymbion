#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Déploiement automatique Symbion Dashboard sur Windows

.DESCRIPTION
    Télécharge le dashboard build depuis GitHub Releases,
    extrait vers répertoire web, et configure serveur HTTP.

.PARAMETER Version
    Version à déployer (ex: dashboard-v2.0.0 ou 'latest')

.PARAMETER GitHubRepo
    Repository GitHub (format: user/repo)

.PARAMETER InstallDir
    Répertoire d'installation (défaut: C:\inetpub\symbion-dashboard)

.PARAMETER Port
    Port serveur HTTP (défaut: 3000)

.EXAMPLE
    .\Deploy-SymbionDashboard.ps1 -Version "dashboard-v2.0.0"

.EXAMPLE
    .\Deploy-SymbionDashboard.ps1 -Version "latest" -InstallDir "C:\Symbion\dashboard"
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$Version = "latest",

    [Parameter(Mandatory=$false)]
    [string]$GitHubRepo = "votre-username/NewSymbion",

    [Parameter(Mandatory=$false)]
    [string]$InstallDir = "C:\Symbion\dashboard",

    [Parameter(Mandatory=$false)]
    [int]$Port = 3000
)

$ErrorActionPreference = "Stop"

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📱 Déploiement Symbion Dashboard $Version (Windows)" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Déterminer URL
if ($Version -eq "latest") {
    $DownloadUrl = "https://github.com/$GitHubRepo/releases/latest/download/symbion-dashboard-latest.tar.gz"
} else {
    $DownloadUrl = "https://github.com/$GitHubRepo/releases/download/$Version/symbion-dashboard-$Version.tar.gz"
}

Write-Host "1️⃣ Téléchargement depuis GitHub Releases..." -ForegroundColor Yellow
Write-Host "   URL: $DownloadUrl" -ForegroundColor Gray

$TempArchive = "$env:TEMP\symbion-dashboard.tar.gz"

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempArchive -UseBasicParsing
    Write-Host "   ✅ Téléchargement réussi" -ForegroundColor Green
} catch {
    Write-Host "   ❌ Échec du téléchargement" -ForegroundColor Red
    Write-Host "   Vérifiez que la version existe: https://github.com/$GitHubRepo/releases" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "2️⃣ Extraction de l'archive..." -ForegroundColor Yellow

# Vérifier si tar est disponible (Windows 10 1903+)
if (Get-Command tar -ErrorAction SilentlyContinue) {
    $TempExtract = "$env:TEMP\symbion-dashboard-extract"
    New-Item -ItemType Directory -Path $TempExtract -Force | Out-Null

    tar -xzf $TempArchive -C $TempExtract
    Write-Host "   ✅ Archive extraite" -ForegroundColor Green
} else {
    Write-Host "   ❌ Commande tar non disponible" -ForegroundColor Red
    Write-Host "   Utilisez Windows 10 1903+ ou installez 7-Zip" -ForegroundColor Yellow
    exit 1
}

# Vérifier contenu
if (-not (Test-Path (Join-Path $TempExtract "index.html"))) {
    Write-Host "   ❌ Archive invalide: index.html manquant" -ForegroundColor Red
    exit 1
}

# Backup ancien déploiement
$BackupDir = "$InstallDir.backup"
if (Test-Path $InstallDir) {
    Write-Host ""
    Write-Host "3️⃣ Sauvegarde du déploiement actuel..." -ForegroundColor Yellow
    if (Test-Path $BackupDir) {
        Remove-Item -Path $BackupDir -Recurse -Force
    }
    Move-Item -Path $InstallDir -Destination $BackupDir
    Write-Host "   ✅ Backup créé: $BackupDir" -ForegroundColor Green
}

# Déployer nouveau build
Write-Host ""
Write-Host "4️⃣ Installation du nouveau build..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Copy-Item -Path "$TempExtract\*" -Destination $InstallDir -Recurse -Force
Write-Host "   ✅ Dashboard déployé: $InstallDir" -ForegroundColor Green

# Cleanup
Remove-Item -Path $TempArchive -Force -ErrorAction SilentlyContinue
Remove-Item -Path $TempExtract -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "✅ Déploiement réussi!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host ""

Write-Host "📊 Informations:" -ForegroundColor Cyan
Write-Host "  - Répertoire:     $InstallDir" -ForegroundColor Gray
Write-Host "  - Backup:         $BackupDir" -ForegroundColor Gray
Write-Host "  - Port suggéré:   $Port" -ForegroundColor Gray
Write-Host ""

Write-Host "🌐 Options de déploiement:" -ForegroundColor Cyan
Write-Host ""
Write-Host "Option 1 : Serveur HTTP Python (Développement)" -ForegroundColor Yellow
Write-Host "  cd $InstallDir" -ForegroundColor Gray
Write-Host "  python -m http.server $Port" -ForegroundColor Gray
Write-Host ""

Write-Host "Option 2 : IIS (Production)" -ForegroundColor Yellow
Write-Host "  1. Activer IIS dans Windows Features" -ForegroundColor Gray
Write-Host "  2. Créer nouveau site web dans IIS Manager" -ForegroundColor Gray
Write-Host "  3. Chemin physique: $InstallDir" -ForegroundColor Gray
Write-Host "  4. Binding: http://*:$Port" -ForegroundColor Gray
Write-Host ""

Write-Host "🔐 Configuration API:" -ForegroundColor Cyan
Write-Host "  Le dashboard utilise config.js pour se connecter au kernel." -ForegroundColor Gray
Write-Host "  Fichier: $InstallDir\config.js" -ForegroundColor Gray
Write-Host ""
Write-Host "  Pour modifier l'URL API:" -ForegroundColor Yellow
Write-Host "  notepad $InstallDir\config.js" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Exemple production:" -ForegroundColor Yellow
Write-Host "    API_BASE: 'https://192.168.1.100:8443'" -ForegroundColor Gray
Write-Host "    API_KEY: 'votre-clé-sécurisée'" -ForegroundColor Gray
Write-Host ""

Write-Host "🔄 Rollback si problème:" -ForegroundColor Cyan
Write-Host "  Remove-Item $InstallDir -Recurse -Force" -ForegroundColor Gray
Write-Host "  Move-Item $BackupDir $InstallDir" -ForegroundColor Gray
Write-Host ""

# Proposer de démarrer serveur Python
Write-Host "Démarrer serveur HTTP Python maintenant? (Y/N): " -ForegroundColor Yellow -NoNewline
$Response = Read-Host

if ($Response -eq "Y" -or $Response -eq "y") {
    # Vérifier Python
    if (Get-Command python -ErrorAction SilentlyContinue) {
        Write-Host ""
        Write-Host "🚀 Démarrage serveur HTTP sur port $Port..." -ForegroundColor Green
        Write-Host "   Accès: http://localhost:$Port" -ForegroundColor Cyan
        Write-Host "   Réseau: http://$(Get-NetIPAddress -AddressFamily IPv4 | Where-Object {$_.InterfaceAlias -notlike '*Loopback*'} | Select-Object -First 1 -ExpandProperty IPAddress):$Port" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "   Press Ctrl+C pour arrêter" -ForegroundColor Gray
        Write-Host ""
        Set-Location $InstallDir
        python -m http.server $Port
    } else {
        Write-Host ""
        Write-Host "⚠️  Python non installé" -ForegroundColor Yellow
        Write-Host "   Installez Python depuis: https://www.python.org/downloads/" -ForegroundColor Cyan
    }
}

Write-Host ""
