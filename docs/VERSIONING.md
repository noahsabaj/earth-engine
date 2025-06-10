# Earth Engine Versioning Strategy

## Current Version: 0.1.0

This document defines our versioning strategy and tracks version milestones objectively.

## Versioning Scheme

We follow **Semantic Versioning** with a twist for pre-1.0:

```
MAJOR.MINOR.PATCH-TAG

Pre-1.0:  0.SPRINT.PATCH-TAG
Post-1.0: MAJOR.MINOR.PATCH
```

### Pre-1.0 Versioning (Current)
- **0**: Indicates pre-release
- **SPRINT**: Sprint number (1-38)
- **PATCH**: Fixes within a sprint
- **TAG**: Optional (alpha, beta, rc)

Examples:
- `0.1.0` - Sprint 1 complete
- `0.26.0` - Sprint 26 complete (current)
- `0.27.0-alpha` - Sprint 27 in progress
- `0.37.0-rc1` - Release candidate

### Post-1.0 Versioning (Future)
- **MAJOR**: Breaking changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes

## Version History

| Version | Date | Description | Status |
|---------|------|-------------|---------|
| 0.1.0 | 2024-01 | Initial foundation | ✅ Released |
| 0.16.0 | 2024-11 | Parallel systems complete | ✅ Released |
| 0.21.0 | 2024-12 | GPU World Architecture | ✅ Released |
| 0.26.0 | 2025-01 | Hot-reload complete | ✅ Released (Current) |
| 0.27.0 | TBD | Memory optimizations | 🚧 Planned |
| 0.37.0 | TBD | Feature complete | 📋 Future |
| 1.0.0-rc1 | TBD | Release candidate | 📋 Future |
| 1.0.0 | TBD | First stable release | 📋 Future |

## Objective Release Criteria

### Version 1.0.0 Requirements

**Core Engine** (MUST HAVE):
- [ ] Single architecture (no OOP remnants)
- [ ] WebGPU support functional
- [ ] Performance within 50% of claims
- [ ] Stable API (no breaking changes planned)
- [ ] Cross-platform builds working

**Basic Gameplay** (MUST HAVE):
- [ ] Complete game loop
- [ ] Inventory system integrated
- [ ] Crafting functional
- [ ] Persistence working
- [ ] Multiplayer validated (100+ players)

**Documentation** (MUST HAVE):
- [ ] API documentation complete
- [ ] Tutorial for developers
- [ ] Example game/mod
- [ ] Performance benchmarks documented

**Quality** (MUST HAVE):
- [ ] Test coverage > 70%
- [ ] No critical bugs
- [ ] Memory leaks eliminated
- [ ] Crash-free for 24 hours

### Version Milestones

#### 0.30.0 - "Architecture Complete"
- All core systems migrated to data-oriented
- GPU-first architecture fully realized
- Instance & metadata systems

#### 0.35.0 - "Performance Realized"
- All optimizations implemented
- Performance claims validated
- Architecture finalized

#### 0.37.0 - "Feature Complete"
- All 1.0 features implemented
- Polish and bug fixes only
- Release candidate preparation

#### 1.0.0-beta - "Public Beta"
- Feature freeze
- Community testing
- Final bug fixes only

#### 1.0.0 - "Stable Release"
- All criteria met
- API stability guaranteed
- Production ready

## How to Update Version

1. **Sprint Completion**:
   ```bash
   # Update VERSION file
   echo "0.27.0" > VERSION
   
   # Update Cargo.toml
   sed -i 's/version = ".*"/version = "0.27.0"/' Cargo.toml
   
   # Tag release
   git tag -a v0.27.0 -m "Sprint 27: Memory Optimizations"
   git push origin v0.27.0
   ```

2. **Create GitHub Release**:
   - Tag: `v0.27.0`
   - Title: "v0.27.0 - Sprint 27: Memory Optimizations"
   - Include sprint summary
   - List completed features
   - Known issues

3. **Update Documentation**:
   - Update this file's version history
   - Update README.md current version
   - Update any version references

## Version Tracking

The version is tracked in multiple places (keep synchronized):
1. `VERSION` file (source of truth)
2. `Cargo.toml` (for builds)
3. `README.md` (for users)
4. Git tags (for history)
5. GitHub releases (for downloads)

## Version Philosophy

- **Honest versioning**: Don't inflate version numbers
- **Meaningful increments**: Each version represents real progress
- **Clear communication**: Version number tells a story
- **No marketing versions**: 1.0 means 1.0, not "good enough"

## FAQ

**Q: Why not call it 1.0 now?**
A: 1.0 implies stability, completeness, and production readiness. We're not there yet.

**Q: When will we reach 1.0?**
A: When all objective criteria are met. Estimated 6-12 months (Sprint 37).

**Q: Can we do 0.x.0 forever?**
A: Yes, until we meet 1.0 criteria. No shame in 0.99.0 if needed.

**Q: What about marketing pressure?**
A: Version integrity > marketing. Use "Technical Preview" or "Early Access" labels.

---

*Version with integrity. Ship when ready.*