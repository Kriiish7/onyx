#!/bin/bash
# Cross-platform build script for Onyx
# Handles platform-specific dependencies and build configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect platform
detect_platform() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
        echo "windows"
    else
        echo "unknown"
    fi
}

# Install platform-specific dependencies
install_dependencies() {
    local platform=$(detect_platform)
    print_status "Detected platform: $platform"
    
    case $platform in
        "linux")
            print_status "Installing Linux dependencies..."
            if command -v apt-get &> /dev/null; then
                sudo apt-get update
                sudo apt-get install -y build-essential pkg-config libssl-dev clang cmake
            elif command -v yum &> /dev/null; then
                sudo yum groupinstall -y "Development Tools"
                sudo yum install -y openssl-devel pkgconfig clang cmake
            else
                print_warning "Package manager not detected. Please install build-essential, pkg-config, libssl-dev, clang, cmake manually."
            fi
            ;;
        "macos")
            print_status "Installing macOS dependencies..."
            if command -v brew &> /dev/null; then
                brew install cmake
            else
                print_warning "Homebrew not found. Please install cmake manually."
            fi
            ;;
        "windows")
            print_status "Windows detected. Please ensure Visual Studio Build Tools are installed:"
            echo "  - Visual Studio Build Tools 2019 or later"
            echo "  - C++ build tools"
            echo "  - Windows SDK"
            echo ""
            print_warning "RocksDB feature will be disabled on Windows due to compilation issues."
            ;;
        *)
            print_error "Unsupported platform: $OSTYPE"
            exit 1
            ;;
    esac
}

# Build with appropriate features
build_project() {
    local platform=$(detect_platform)
    local features=""
    
    # Disable RocksDB on Windows due to compilation issues
    if [[ "$platform" == "windows" ]]; then
        print_warning "Building without RocksDB on Windows"
        features=""
    else
        features="--features rocksdb-storage"
    fi
    
    print_status "Building project with features: $features"
    cargo build --release $features
}

# Run tests with appropriate features
run_tests() {
    local platform=$(detect_platform)
    local features=""
    
    if [[ "$platform" == "windows" ]]; then
        print_warning "Running tests without RocksDB on Windows"
        features=""
    else
        features="--features rocksdb-storage"
    fi
    
    print_status "Running tests with features: $features"
    cargo test --verbose $features
}

# Main function
main() {
    local command=${1:-"build"}
    
    case $command in
        "deps"|"dependencies")
            install_dependencies
            ;;
        "build")
            install_dependencies
            build_project
            ;;
        "test")
            install_dependencies
            run_tests
            ;;
        "check")
            install_dependencies
            print_status "Running full check (format, lint, test)..."
            cargo fmt --all -- --check
            cargo clippy --all-targets --all-features -- -D warnings
            run_tests
            ;;
        "clean")
            print_status "Cleaning build artifacts..."
            cargo clean
            ;;
        "help"|"-h"|"--help")
            echo "Cross-platform build script for Onyx"
            echo ""
            echo "Usage: $0 [COMMAND]"
            echo ""
            echo "Commands:"
            echo "  deps, dependencies  Install platform-specific dependencies"
            echo "  build               Build the project"
            echo "  test                Run tests"
            echo "  check               Run format check, lint, and tests"
            echo "  clean               Clean build artifacts"
            echo "  help                Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 build    # Build with platform-specific features"
            echo "  $0 test     # Run tests with platform-specific features"
            echo "  $0 check    # Run full quality check"
            ;;
        *)
            print_error "Unknown command: $command"
            echo "Run '$0 help' for usage information."
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"