# Cross-platform build script for Onyx (Windows PowerShell)
# Handles Windows-specific dependencies and build configurations

param(
    [Parameter(Position=0)]
    [ValidateSet("deps", "dependencies", "build", "test", "check", "clean", "help")]
    [string]$Command = "build"
)

# Colors for output
$Colors = @{
    Red = "Red"
    Green = "Green"
    Yellow = "Yellow"
    White = "White"
}

function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor $Colors.Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor $Colors.Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor $Colors.Red
}

function Install-Dependencies {
    Write-Status "Windows detected. Checking for Visual Studio Build Tools..."
    
    # Check for Visual Studio Build Tools
    $vsWhere = Get-Command "vswhere.exe" -ErrorAction SilentlyContinue
    if ($vsWhere) {
        $vsInstall = & $vsWhere.Path -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($vsInstall) {
            Write-Status "Visual Studio Build Tools found at: $vsInstall"
        } else {
            Write-Warning "Visual Studio Build Tools with C++ tools not found!"
            Write-Warning "Please install Visual Studio Build Tools 2019 or later with:"
            Write-Warning "  - C++ build tools"
            Write-Warning "  - Windows SDK"
            Write-Warning "Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022"
        }
    } else {
        Write-Warning "vswhere.exe not found. Visual Studio may not be installed."
        Write-Warning "Please install Visual Studio Build Tools 2019 or later."
    }
    
    # Check for CMake
    $cmake = Get-Command "cmake.exe" -ErrorAction SilentlyContinue
    if ($cmake) {
        Write-Status "CMake found at: $($cmake.Source)"
    } else {
        Write-Warning "CMake not found. Please install CMake or add it to PATH."
    }
    
    Write-Warning "RocksDB feature will be disabled on Windows due to compilation issues."
}

function Build-Project {
    Write-Status "Attempting to build with RocksDB on Windows..."
    
    # Set environment variables for RocksDB
    $env:ROCKSDB_SYS_STATIC = "1"
    $env:RUSTFLAGS = "-C target-feature=+crt-static"
    
    # Try to build with RocksDB first
    try {
        Write-Status "Building with RocksDB feature..."
        cargo build --release --features rocksdb-storage
        if ($LASTEXITCODE -eq 0) {
            Write-Status "Successfully built with RocksDB!"
            return
        }
    }
    catch {
        Write-Warning "RocksDB build failed: $($_.Exception.Message)"
    }
    
    # Fallback to build without RocksDB
    Write-Warning "Falling back to build without RocksDB..."
    cargo build --release
}

function Run-Tests {
    Write-Status "Attempting to run tests with RocksDB on Windows..."
    
    # Set environment variables for RocksDB
    $env:ROCKSDB_SYS_STATIC = "1"
    $env:RUSTFLAGS = "-C target-feature=+crt-static"
    
    # Try to run tests with RocksDB first
    try {
        Write-Status "Running tests with RocksDB feature..."
        cargo test --verbose --features rocksdb-storage
        if ($LASTEXITCODE -eq 0) {
            Write-Status "Successfully ran tests with RocksDB!"
            return
        }
    }
    catch {
        Write-Warning "RocksDB test run failed: $($_.Exception.Message)"
    }
    
    # Fallback to run tests without RocksDB
    Write-Warning "Falling back to run tests without RocksDB..."
    cargo test --verbose
}

function Run-Check {
    Write-Status "Running full check (format, lint, test)..."
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Code formatting check failed!"
        exit 1
    }
    
    cargo clippy --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Clippy linting failed!"
        exit 1
    }
    
    Run-Tests
}

function Clean-Build {
    Write-Status "Cleaning build artifacts..."
    cargo clean
}

function Show-Help {
    Write-Host "Cross-platform build script for Onyx (Windows PowerShell)" -ForegroundColor $Colors.White
    Write-Host ""
    Write-Host "Usage: .\scripts\build.ps1 [COMMAND]" -ForegroundColor $Colors.White
    Write-Host ""
    Write-Host "Commands:" -ForegroundColor $Colors.White
    Write-Host "  deps, dependencies  Install platform-specific dependencies" -ForegroundColor $Colors.White
    Write-Host "  build               Build the project" -ForegroundColor $Colors.White
    Write-Host "  test                Run tests" -ForegroundColor $Colors.White
    Write-Host "  check               Run format check, lint, and tests" -ForegroundColor $Colors.White
    Write-Host "  clean               Clean build artifacts" -ForegroundColor $Colors.White
    Write-Host "  help                Show this help message" -ForegroundColor $Colors.White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor $Colors.White
    Write-Host "  .\scripts\build.ps1 build    # Build without RocksDB on Windows" -ForegroundColor $Colors.White
    Write-Host "  .\scripts\build.ps1 test     # Run tests without RocksDB" -ForegroundColor $Colors.White
    Write-Host "  .\scripts\build.ps1 check    # Run full quality check" -ForegroundColor $Colors.White
}

# Main execution
switch ($Command) {
    "deps" { 
        Install-Dependencies 
    }
    "dependencies" { 
        Install-Dependencies 
    }
    "build" { 
        Install-Dependencies
        Build-Project 
    }
    "test" { 
        Install-Dependencies
        Run-Tests 
    }
    "check" { 
        Install-Dependencies
        Run-Check 
    }
    "clean" { 
        Clean-Build 
    }
    "help" { 
        Show-Help 
    }
    default { 
        Write-Error "Unknown command: $Command"
        Write-Host "Run '.\scripts\build.ps1 help' for usage information."
        exit 1
    }
}