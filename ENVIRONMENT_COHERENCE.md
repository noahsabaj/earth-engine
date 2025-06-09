# Earth Engine Environment Coherence Guide

## Overview
This document ensures we maintain **full coherence** between the Linux (WSL) development environment and the Windows testing environment. 

**CRITICAL**: The Linux/WSL environment is the SOURCE OF TRUTH. Windows is ONLY for GPU testing.

## Directory Structure

### Linux/WSL (PRIMARY - Development Environment)
```
/home/nsabaj/earth-engine-workspace/earth-engine/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── bin/           # All test binaries
│   ├── world/         # World management
│   ├── renderer/      # Rendering systems
│   ├── lighting/      # Lighting systems
│   ├── physics/       # Physics
│   ├── network/       # Networking
│   └── ...
├── target/            # Build artifacts
└── *.md              # Documentation
```

### Windows (SECONDARY - Testing Only)
```
C:\earth-engine-project\
├── Cargo.toml
├── Cargo.lock  
├── earth-engine\      # Source code directory
│   └── src/          # Mirror of Linux src/
└── target/           # Windows build artifacts
```

## Development Workflow

### 1. ALWAYS Develop in Linux/WSL
```bash
cd /home/nsabaj/earth-engine-workspace/earth-engine
# Do all development here
cargo build
cargo test
```

### 2. Sync to Windows After Changes
After ANY development work in Linux, run this sync script:

```bash
# Full sync from Linux to Windows (overwrites everything)
rsync -av --delete \
    /home/nsabaj/earth-engine-workspace/earth-engine/src/ \
    /mnt/c/earth-engine-project/earth-engine/src/

# Copy Cargo files
cp /home/nsabaj/earth-engine-workspace/earth-engine/Cargo.toml \
   /mnt/c/earth-engine-project/Cargo.toml

# Copy documentation
cp /home/nsabaj/earth-engine-workspace/earth-engine/*.md \
   /mnt/c/earth-engine-project/
```

### 3. Windows Testing
```powershell
cd C:\earth-engine-project
cargo run --bin gpu_test       # Test GPU detection
cargo run --release            # Test with optimizations
```

## GPU Test Results (RTX 4060 Ti - Confirmed Working)
```
GPU #1: NVIDIA GeForce RTX 4060 Ti
  Backend: Vulkan
  Device Type: DiscreteGpu
  Driver: NVIDIA
  Driver Info: 572.16
  Supports Compute: true  ← Best option

GPU #2: NVIDIA GeForce RTX 4060 Ti
  Backend: Dx12
  Device Type: DiscreteGpu
  Supports Compute: false

GPU #3: Microsoft Basic Render Driver
  Backend: Dx12
  Device Type: Cpu

GPU #4: NVIDIA GeForce RTX 4060 Ti/PCIe/SSE2
  Backend: Gl
  Device Type: Other
```
- **Preferred Backend**: Vulkan (supports compute shaders)
- **4 adapters detected**: Multiple API options available

## Sync Script

Create this as `/home/nsabaj/earth-engine-workspace/earth-engine/sync_to_windows.sh`:

```bash
#!/bin/bash
echo "=== Syncing Earth Engine to Windows ==="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Source and destination paths
SRC_ROOT="/home/nsabaj/earth-engine-workspace/earth-engine"
DEST_ROOT="/mnt/c/earth-engine-project"

# Ensure destination exists
mkdir -p "$DEST_ROOT/earth-engine/src"

# Sync source files (with delete to ensure exact mirror)
echo "Syncing source files..."
if rsync -av --delete "$SRC_ROOT/src/" "$DEST_ROOT/earth-engine/src/"; then
    echo -e "${GREEN}✓ Source files synced${NC}"
else
    echo -e "${RED}✗ Failed to sync source files${NC}"
    exit 1
fi

# Copy Cargo.toml (project level)
echo "Copying Cargo.toml..."
if cp "$SRC_ROOT/Cargo.toml" "$DEST_ROOT/Cargo.toml"; then
    echo -e "${GREEN}✓ Cargo.toml synced${NC}"
else
    echo -e "${RED}✗ Failed to copy Cargo.toml${NC}"
fi

# Copy all documentation files
echo "Copying documentation..."
cp "$SRC_ROOT"/*.md "$DEST_ROOT/" 2>/dev/null
echo -e "${GREEN}✓ Documentation synced${NC}"

# List what was synced
echo -e "\n${GREEN}=== Sync Complete ===${NC}"
echo "Windows project ready at: C:\\earth-engine-project"
echo "Run GPU tests with: cargo run --bin gpu_test"
```

## Important Files to Track

### Always in Both Environments:
- All `.rs` source files
- `Cargo.toml`
- Documentation (`.md` files)

### May Differ:
- `Cargo.lock` (platform-specific dependencies)
- `target/` directory (build artifacts)
- `.gitignore` (if using git)

## Common Issues & Solutions

### 1. Missing Files in Windows
**Solution**: Run full sync from Linux
```bash
bash sync_to_windows.sh
```

### 2. Compilation Errors in Windows
**Check**:
- All files synced properly
- Windows-specific features in Cargo.toml
- DirectX/Vulkan SDK installed

### 3. Performance Differences
**Expected**: Windows may perform differently due to:
- Different GPU drivers
- DirectX vs Vulkan backends
- Debug vs Release builds

## Sprint Development Process

1. **Start Sprint in Linux**
   ```bash
   cd /home/nsabaj/earth-engine-workspace/earth-engine
   # Create new features/files
   ```

2. **Test in Linux**
   ```bash
   cargo test
   cargo run --bin <test_name>
   ```

3. **Sync to Windows**
   ```bash
   bash sync_to_windows.sh
   ```

4. **GPU Test in Windows**
   ```powershell
   cargo run --bin gpu_test
   cargo run --release
   ```

5. **Document Results**
   - Update sprint documentation in Linux
   - Sync again to Windows

## NEVER DO:
- ❌ Edit files directly in Windows (except for quick fixes)
- ❌ Create new files in Windows
- ❌ Commit from Windows (if using git)
- ❌ Have different versions of the same file

## ALWAYS DO:
- ✅ Develop in Linux/WSL
- ✅ Sync after EVERY change
- ✅ Test GPU features in Windows
- ✅ Keep documentation updated
- ✅ Run `sync_to_windows.sh` before Windows testing

## Quick Reference Commands

```bash
# In Linux - Check file count
find /home/nsabaj/earth-engine-workspace/earth-engine/src -name "*.rs" | wc -l

# In Linux - Full sync to Windows
rsync -av --delete /home/nsabaj/earth-engine-workspace/earth-engine/src/ /mnt/c/earth-engine-project/earth-engine/src/

# In Windows - GPU test
cd C:\earth-engine-project && cargo run --bin gpu_test

# In Windows - Release build test
cd C:\earth-engine-project && cargo run --release
```

---
Last Updated: Sprint 16 (Parallel Lighting System) ✅
Next Sprint: Sprint 17 (Performance & Data Layout Analysis)
See [MASTER_ROADMAP.md](MASTER_ROADMAP.md) for complete sprint details.