# Windows Build Environment Setup Script
# This script helps set up the Windows build environment for Onyx

param(
    [Parameter(Position=0)]
    [ValidateSet("check", "install", "help")]
    [string]$Action = "check"
)

function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Check-Prerequisites {
    Write-Status "Checking Windows build prerequisites..."
    
    $issues = @()
    
    # Check Visual Studio Build Tools
    $vsWhere = Get-Command "vswhere.exe" -ErrorAction SilentlyContinue
    if ($vsWhere) {
        try {
            $vsInstall = & $vsWhere.Path -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
            if ($vsInstall) {
                Write-Status "✓ Visual Studio Build Tools found at: $vsInstall"
            } else {
                $issues += "Visual Studio Build Tools with C++ tools not found"
            }
        } catch {
            $issues += "Error checking Visual Studio installation: $($_.Exception.Message)"
        }
    } else {
        $issues += "vswhere.exe not found - Visual Studio may not be installed"
    }
    
    # Check CMake
    $cmake = Get-Command "cmake.exe" -ErrorAction SilentlyContinue
    if ($cmake) {
        try {
            $cmakeVersion = & $cmake --version | Select-Object -First 1
            Write-Status "✓ CMake found: $cmakeVersion"
        } catch {
            $issues += "CMake found but failed to get version"
        }
    } else {
        $issues += "CMake not found in PATH"
    }
    
    # Check Git
    $git = Get-Command "git.exe" -ErrorAction SilentlyContinue
    if ($git) {
        try {
            $gitVersion = & $git --version
            Write-Status "✓ Git found: $gitVersion"
        } catch {
            $issues += "Git found but failed to get version"
        }
    } else {
        $issues += "Git not found in PATH"
    }
    
    # Check Rust
    $cargo = Get-Command "cargo.exe" -ErrorAction SilentlyContinue
    if ($cargo) {
        try {
            $rustVersion = & $cargo --version
            Write-Status "✓ Rust/Cargo found: $rustVersion"
        } catch {
            $issues += "Cargo found but failed to get version"
        }
    } else {
        $issues += "Rust/Cargo not found in PATH"
    }
    
    # Report results
    if ($issues.Count -eq 0) {
        Write-Status "✓ All prerequisites satisfied!"
        return $true
    } else {
        Write-Error "Found $($issues.Count) issue(s):"
        foreach ($issue in $issues) {
            Write-Error "  - $issue"
        }
        return $false
    }
}

function Show-InstallationGuide {
    Write-Host "Windows Build Environment Setup Guide" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "1. Install Visual Studio Build Tools" -ForegroundColor Yellow
    Write-Host "   Download: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022" -ForegroundColor White
    Write-Host "   Required components:" -ForegroundColor White
    Write-Host "   - C++ build tools" -ForegroundColor Gray
    Write-Host "   - Windows SDK (latest)" -ForegroundColor Gray
    Write-Host "   - MSVC v143 - VS 2022 C++ x64/x86 build tools" -ForegroundColor Gray
    Write-Host ""
    
    Write-Host "2. Install CMake" -ForegroundColor Yellow
    Write-Host "   Download: https://cmake.org/download/" -ForegroundColor White
    Write-Host "   Version: 3.20 or later" -ForegroundColor White
    Write-Host "   Important: Add to PATH during installation" -ForegroundColor White
    Write-Host ""
    
    Write-Host "3. Install Git for Windows" -ForegroundColor Yellow
    Write-Host "   Download: https://git-scm.com/download/win" -ForegroundColor White
    Write-Host "   Important: Use Git from the Windows Command Prompt" -ForegroundColor White
    Write-Host ""
    
    Write-Host "4. Install Rust" -ForegroundColor Yellow
    Write-Host "   Download: https://rustup.rs/" -ForegroundColor White
    Write-Host "   Command: rustup-init.exe" -ForegroundColor White
    Write-Host ""
    
    Write-Host "5. Restart PowerShell" -ForegroundColor Yellow
    Write-Host "   Close and reopen PowerShell to refresh PATH" -ForegroundColor White
    Write-Host ""
    
    Write-Host "6. Verify Installation" -ForegroundColor Yellow
    Write-Host "   Run: .\scripts\setup-windows.ps1 check" -ForegroundColor White
    Write-Host ""
    
    Write-Host "Alternative: Use Docker Desktop" -ForegroundColor Cyan
    Write-Host "If native build fails, you can use Docker:" -ForegroundColor White
    Write-Host "1. Install Docker Desktop" -ForegroundColor White
    Write-Host "2. docker build -t onyx-windows ." -ForegroundColor White
    Write-Host "3. docker run -it onyx-windows" -ForegroundColor White
}

function Show-Help {
    Write-Host "Windows Setup Script for Onyx" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\scripts\setup-windows.ps1 [ACTION]" -ForegroundColor White
    Write-Host ""
    Write-Host "Actions:" -ForegroundColor White
    Write-Host "  check    Check if all prerequisites are installed" -ForegroundColor White
    Write-Host "  install Show installation guide" -ForegroundColor White
    Write-Host "  help     Show this help message" -ForegroundColor White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor White
    Write-Host "  .\scripts\setup-windows.ps1 check     # Check prerequisites" -ForegroundColor White
    Write-Host "  .\scripts\setup-windows.ps1 install   # Show installation guide" -ForegroundColor White
}

# Main execution
switch ($Action) {
    "check" {
        $success = Check-Prerequisites
        if (-not $success) {
            Write-Host ""
            Write-Warning "Run '.\scripts\setup-windows.ps1 install' for setup instructions"
            exit 1
        }
    }
    "install" {
        Show-InstallationGuide
    }
    "help" {
        Show-Help
    }
    default {
        Write-Error "Unknown action: $Action"
        Show-Help
        exit 1
    }
}