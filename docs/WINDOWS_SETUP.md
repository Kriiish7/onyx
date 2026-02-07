# Windows Build Setup for Onyx

This document provides instructions for building Onyx with RocksDB support on Windows.

## Prerequisites

### 1. Visual Studio Build Tools
Install Visual Studio Build Tools 2019 or later with:
- C++ build tools
- Windows SDK (latest)
- MSVC v143 - VS 2022 C++ x64/x86 build tools

Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022

### 2. CMake
Install CMake 3.20 or later:
- Download from: https://cmake.org/download/
- Add to PATH during installation

### 3. Git
Install Git for Windows:
- Download from: https://git-scm.com/download/win

## Build Instructions

### Option 1: Using PowerShell Build Script
```powershell
# Install dependencies
.\scripts\build.ps1 deps

# Build with RocksDB (experimental)
.\scripts\build.ps1 build
```

### Option 2: Manual Build
```cmd
# Set environment variables
set RUSTFLAGS=-C target-feature=+crt-static
set ROCKSDB_SYS_STATIC=1

# Build with RocksDB feature
cargo build --release --features rocksdb-storage
```

## Troubleshooting

### Common Issues

1. **Linker errors with RocksDB**
   ```
   Solution: Set ROCKSDB_SYS_STATIC=1 environment variable
   ```

2. **Missing vcruntime140.dll**
   ```
   Solution: Install Visual Studio C++ Redistributable
   ```

3. **CMake not found**
   ```
   Solution: Add CMake to PATH or restart PowerShell
   ```

4. **Build timeout**
   ```
   Solution: Increase timeout or use fewer parallel jobs:
   cargo build -j 1 --release --features rocksdb-storage
   ```

### Alternative: Docker on Windows
If native build fails, use Docker Desktop:

```cmd
# Build using Docker (Linux container)
docker build -t onyx-windows .
docker run -it onyx-windows
```

## Verification

To verify RocksDB is working:

```rust
// In your code
#[cfg(feature = "rocksdb-storage")]
{
    println!("RocksDB storage enabled!");
}
```

Or check the binary size:
- Without RocksDB: ~8MB
- With RocksDB: ~15MB

## Performance Notes

- RocksDB on Windows may have slower performance than Linux
- Consider using SSD for better I/O performance
- Monitor memory usage as RocksDB can be memory-intensive

## Support

If you encounter issues:
1. Check the troubleshooting section above
2. Open an issue on GitHub with:
   - Windows version
   - Visual Studio Build Tools version
   - Complete error message
   - Output of `cargo --version` and `rustc --version`