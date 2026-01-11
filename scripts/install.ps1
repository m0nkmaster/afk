#Requires -Version 5.1
<#
.SYNOPSIS
    afk installer for Windows

.DESCRIPTION
    Downloads and installs afk to %LOCALAPPDATA%\afk\bin

.PARAMETER Beta
    Install from beta channel (pre-releases)

.EXAMPLE
    irm https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.ps1 | iex

.EXAMPLE
    & { param($Beta) irm https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.ps1 | iex } -Beta
#>

param(
    [switch]$Beta
)

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "m0nkmaster/afk"
$InstallDir = "$env:LOCALAPPDATA\afk\bin"
$BinaryName = "afk.exe"

function Write-Info {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Yellow
}

function Get-LatestRelease {
    $Url = "https://api.github.com/repos/$Repo/releases"
    
    try {
        if ($Beta) {
            # Get latest release including pre-releases
            $Releases = Invoke-RestMethod -Uri $Url -UseBasicParsing
            return $Releases[0].tag_name
        } else {
            # Get latest stable release only
            $Release = Invoke-RestMethod -Uri "$Url/latest" -UseBasicParsing
            return $Release.tag_name
        }
    } catch {
        throw "Failed to get latest release: $_"
    }
}

function Get-FileHash256 {
    param([string]$Path)
    $Hash = Get-FileHash -Path $Path -Algorithm SHA256
    return $Hash.Hash.ToLower()
}

function Install-Afk {
    Write-Host ""
    Write-Info "afk installer"
    Write-Host ""
    
    $Platform = "windows-x86_64"
    $BinaryDownloadName = "afk-$Platform.exe"
    
    Write-Info "Detected: $Platform"
    
    $Version = Get-LatestRelease
    if (-not $Version) {
        throw "Could not determine latest version"
    }
    
    if ($Beta) {
        Write-Info "Channel: beta"
    }
    
    $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$BinaryDownloadName"
    $ChecksumUrl = "https://github.com/$Repo/releases/download/$Version/checksums.sha256"
    
    # Create temp directory
    $TempDir = Join-Path $env:TEMP "afk-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
    
    try {
        $TempBinary = Join-Path $TempDir $BinaryDownloadName
        $TempChecksum = Join-Path $TempDir "checksums.sha256"
        
        Write-Info "Downloading afk $Version..."
        
        # Download binary
        try {
            Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempBinary -UseBasicParsing
        } catch {
            throw "Failed to download binary: $_"
        }
        
        # Download and verify checksum
        try {
            Invoke-WebRequest -Uri $ChecksumUrl -OutFile $TempChecksum -UseBasicParsing
            
            Write-Info "Verifying checksum..."
            
            $ExpectedHash = (Get-Content $TempChecksum | 
                Where-Object { $_ -match $BinaryDownloadName } |
                ForEach-Object { ($_ -split '\s+')[0] }).ToLower()
            
            $ActualHash = (Get-FileHash256 -Path $TempBinary)
            
            if ($ExpectedHash -ne $ActualHash) {
                throw "Checksum verification failed!`nExpected: $ExpectedHash`nActual: $ActualHash"
            }
        } catch [System.Net.WebException] {
            Write-Warn "Warning: Could not download checksums for verification"
        }
        
        # Create install directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        
        $FinalPath = Join-Path $InstallDir $BinaryName
        
        Write-Info "Installing to $FinalPath..."
        
        # Remove old binary if exists
        if (Test-Path $FinalPath) {
            Remove-Item $FinalPath -Force
        }
        
        # Move binary to install location
        Move-Item $TempBinary $FinalPath -Force
        
    } finally {
        # Cleanup
        if (Test-Path $TempDir) {
            Remove-Item $TempDir -Recurse -Force
        }
    }
    
    # Add to PATH if needed
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Host ""
        Write-Info "Adding $InstallDir to PATH..."
        
        $NewPath = "$InstallDir;$UserPath"
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        
        # Update current session
        $env:Path = "$InstallDir;$env:Path"
        
        Write-Success "PATH updated. You may need to restart your terminal."
    }
    
    Write-Host ""
    Write-Success "afk $Version installed successfully!"
    Write-Host ""
    Write-Host "Get started:"
    Write-Host "  afk go            # Zero-config: auto-detect and run"
    Write-Host "  afk --help        # Show all commands"
    Write-Host ""
}

Install-Afk
