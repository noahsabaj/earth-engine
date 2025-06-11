# Earth Engine - Claude Instructions

**PURPOSE**: This document contains TIMELESS instructions that don't change with sprints.
**For current sprint/priorities**: See CURRENT.md
**For sprint planning**: See MASTER_ROADMAP.md

## ENVIRONMENT SETUP
- **Claude (AI)**: Working in WSL Ubuntu at `/home/nsabaj/earth-engine-workspace/earth-engine`
- **Human User**: Working in Windows, pulls changes from main branch
- **Workflow**: Claude makes changes in WSL → pushes to main → User pulls in Windows

## PROJECT OVERVIEW
This is **Earth Engine** - a frontier SOTA voxel game engine. We are building in this order:
1. **ENGINE** (Current Phase) - Game-agnostic voxel engine with cutting-edge performance
2. **GAME** (Next Phase) - Earth MMO implementation using the engine
3. **FRAMEWORK** (Final Phase) - Tools enabling others to build ANY voxel game

### Earth MMO Vision (Future Game)
- **Physical information economy** - no copy/paste, all information must be hand-copied
- **Stone age → space age progression** - intuitive discovery, no recipe books
- **Planetary servers** - each region is its own planet that develops unique culture
- **1m³ voxels** - uniform, perfect for realistic physics and material properties
- **Realistic voxel physics** - materials break/burn/flow based on real properties
- **Target**: 10,000+ concurrent players per planet at 144+ FPS

The engine will eventually be released for others to create their own voxel experiences.

## CRITICAL PHILOSOPHY
**DATA-ORIENTED PROGRAMMING ONLY** - This codebase follows strict DOP principles:
- ❌ NO classes, objects, or OOP patterns
- ❌ NO methods - only functions that transform data
- ✅ Data lives in shared buffers (WorldBuffer, RenderBuffer, etc.)
- ✅ Systems are stateless kernels that read/write buffers
- ✅ GPU-first architecture - data lives where it's processed
- ✅ If you're writing `self.method()`, you're doing it wrong

## WORKFLOW REQUIREMENTS

### 1. Documentation Updates (MANDATORY)

#### Living Documents (Update After EVERY Work Session)
1. **MASTER_ROADMAP.md** - Sprint planning and tracking
2. **CURRENT.md** - Active sprint, completion percentages, recent work
3. **Active Sprint file** - `/docs/sprints/SPRINT_XX_*.md` for current sprint

#### Sprint Completion Checklist
When finishing a sprint:
- [ ] Mark sprint complete in MASTER_ROADMAP.md
- [ ] Update CURRENT.md with new sprint number
- [ ] Create new sprint file if needed
- [ ] Archive completed sprint docs if necessary
- [ ] Review ALL docs for stale sprint references

#### Documentation Principles
- **Single Source of Truth** - Each fact lives in exactly ONE document
- **No Duplication** - Reference other docs, don't copy content
- **Timeless vs Temporal** - CLAUDE.md has timeless info, CURRENT.md has temporal
- **Regular Reviews** - Check for stale info, consolidation opportunities
- **Honest Metrics** - Real percentages, not optimistic guesses

### 2. Git Workflow (ALWAYS)
```bash
# 1. Create feature branch
git checkout -b feature/description

# 2. Make changes, commit with descriptive messages
git add -A
git commit -m "feat: implement thermal dynamics on GPU"

# 3. Push branch
git push -u origin feature/description

# 4. Create PR, merge to main
gh pr create --title "Add thermal dynamics" --body "..."
gh pr merge

# 5. Update main and clean up
git checkout main
git pull
git branch -d feature/description
```

### 3. Verification Process (REQUIRED)
Before considering ANY task complete:
1. Run `cargo check` - must pass
2. Run `cargo test` - must pass
3. Run `cargo clippy` - address warnings
4. Verify the feature actually works as intended
5. Update all relevant documentation
6. Check that no unwrap() calls were added
7. Ensure no OOP patterns were introduced

## CODE STANDARDS

### Long-Term Code Philosophy
- **NO BANDAIDS** - No temporary solutions, hacks, or "fix it later" code
- **Build for decades** - This code should decrease technical debt over time
- **Extensibility without breaking** - New features must not break existing functionality  
- **Clean code when possible** - Readability matters, but not at the cost of performance
- **Kaizen over revolution** - Continuous small improvements over big rewrites
- **Think in systems** - How will this code interact with features we haven't built yet?
- **Measure twice, code once** - Design decisions should be deliberate and documented

### Error Handling
- NEVER use `.unwrap()` - use `?` operator or proper error handling
- Create module-specific error types (NetworkError, RenderError, etc.)
- Every fallible operation must return Result<T, E>

### Data Layout
```rust
// ❌ WRONG - OOP style
struct Chunk {
    fn generate(&mut self) { } // NO METHODS!
}

// ✅ CORRECT - DOP style
struct ChunkData {
    voxels: Buffer<Voxel>,
    position: ChunkPos,
}

fn generate_chunk(data: &mut ChunkData, gen_params: &GenParams) {
    // Transform data, no self
}
```

### Performance
- Profile before optimizing
- Data locality matters more than "clean" code
- Prefer SOA (Structure of Arrays) over AOS
- Batch operations for GPU

## WHERE TO FIND CURRENT PRIORITIES
- **Active Sprint**: Check `/docs/status/CURRENT.md` for current sprint number and focus
- **Sprint Details**: Check `/docs/docs/MASTER_ROADMAP.md` for full sprint planning
- **Progress Tracking**: Check relevant `/docs/sprints/SPRINT_XX_*.md` files

## EVERGREEN PRIORITIES (Always True)
1. **Zero-panic architecture** - No unwrap(), no crashes in production
2. **Data-oriented design** - Everything is data + kernels, no OOP
3. **GPU-first computation** - Always ask "can GPU do this?"
4. **Documentation accuracy** - Keep all docs in sync with reality
5. **Long-term thinking** - No bandaids, build for decades

## COMMON PITFALLS TO AVOID
1. **Creating unnecessary documents** - Fix code first, document after
2. **OOP creep** - Watch for methods, traits with behavior, unnecessary abstractions
3. **CPU-thinking** - Always ask "can GPU do this in parallel?"
4. **Forgetting the vision** - This enables physical information economy
5. **Not updating CURRENT.md** - Track progress honestly

## TESTING REMINDERS
- Test with missing files (no unwrap crashes)
- Test network disconnections (graceful handling)
- Test malformed data (proper errors, not panics)
- Benchmark everything - we need 1000x performance

## VISION REMINDERS
This engine enables:
- Books that must be hand-copied (no copy/paste)
- Knowledge that can be lost forever
- Technologies discovered through experimentation
- Civilizations that develop uniquely per planet
- Realistic material physics - wood splinters, metal bends, stone crumbles
- Thermal dynamics calculated per voxel on GPU
- Voxel-based destruction with real material properties

Every line of code should move us toward this vision, but remember: we're building a game-agnostic ENGINE first.

## WHEN IN DOUBT
1. Choose performance over "clean" code
2. Choose data-oriented over object-oriented
3. Choose GPU computation over CPU
4. Choose explicit over abstract
5. Choose measured results over assumptions
6. Choose boring stability over exciting features
7. Choose proven patterns over novel experiments
8. Choose cache-friendly over "logical" organization
9. Choose batch operations over individual updates
10. Choose predictable behavior over clever tricks
11. Choose documentation over self-documenting code
12. Choose tomorrow's maintainability over today's convenience
13. Choose real benchmarks over theoretical benefits
14. Choose user trust over feature count
15. Choose physics accuracy over gameplay shortcuts

Remember: We're building the future of voxel engines. This is a 10-year project, not a 10-week sprint. No compromises on fundamentals.