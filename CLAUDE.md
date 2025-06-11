# Earth Engine - Claude Instructions

## ENVIRONMENT SETUP
- **Claude (AI)**: Working in WSL Ubuntu at `/home/nsabaj/earth-engine-workspace/earth-engine`
- **Human User**: Working in Windows, pulls changes from main branch
- **Workflow**: Claude makes changes in WSL → pushes to main → User pulls in Windows

## PROJECT OVERVIEW
This is **Earth Engine** - a frontier SOTA voxel game engine being built for the **Earth MMO**, a revolutionary game with:
- **Physical information economy** - no copy/paste, all information must be hand-copied
- **Stone age → space age progression** - intuitive discovery, no recipe books
- **Planetary servers** - each region is its own planet that develops unique culture
- **1m³ voxels** - uniform, perfect for realistic physics and destruction
- **Target**: 10,000+ concurrent players per planet at 144+ FPS

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
After ANY work session, update:
- `/docs/status/CURRENT.md` - current sprint progress, completion percentages
- `/docs/sprints/SPRINT_XX_*.md` - relevant sprint documentation
- Create/update technical docs for new systems
- Keep completion percentages HONEST and ACCURATE

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

## CURRENT PRIORITIES
1. **Sprint 35.1 Emergency** - Replace all 373 unwrap() calls
2. **Zero-panic architecture** - No crashes in production
3. **GPU-first migration** - Move all possible computation to GPU
4. **Documentation accuracy** - Keep all docs updated and honest

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
- Teardown-style voxel destruction
- Thermal dynamics calculated per voxel on GPU

Every line of code should move us toward this vision.

## WHEN IN DOUBT
1. Choose performance over "clean" code
2. Choose data-oriented over object-oriented
3. Choose GPU computation over CPU
4. Choose explicit over abstract
5. Choose measured results over assumptions

Remember: We're building the future of voxel engines. No compromises.