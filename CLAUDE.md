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
- **Everything transformable** - Every voxel can be destroyed/built/changed
- **Local communication only** - Voice carries 50m, no global chat, messages travel physically
- **Player expression** - Dye/sew fabric for flags, write signs, create visual identity
- **Technology matters** - Radio enables long-distance comms (late game)
- **Player-created civilizations** - Cities with smithies, taverns, mayors, kings
- **Or complete anarchy** - Roaming bandits, isolated hermits, anything goes
- **Full physics simulation** - Thermal, fluid, sound, structural per voxel
- **"Real life but fun"** - Complex emergence from simple physics laws
- **Target**: 10,000+ concurrent players per planet at 144+ FPS

Think "Minecraft Civilization videos" but that's just ONE way to play. The engine enables:
- Rust-like survival and raiding
- Garry's Mod sandbox creativity  
- DayZ & Arma Combat and Realism
- EVE Online emergent politics
- SimCity player-made cities
- All in one persistent world

The engine provides the canvas. Players paint civilizations... or chaos.

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

### 2. Git Workflow (MANDATORY AFTER EVERY WORK SESSION)

**CRITICAL**: You MUST commit, merge to main, and push after EVERY work session. No exceptions.

```bash
# 1. Check current status and branch
git status
git branch

# 2. If not on a feature branch, create one
git checkout -b feature/sprint-35-1-emergency  # Example name

# 3. Stage ALL changes
git add -A

# 4. Commit with descriptive message
git commit -m "feat(sprint-35.1): replace all unwrap() calls, add bounds checking, document unsafe blocks"

# 5. Switch to main and merge
git checkout main
git merge feature/sprint-35-1-emergency

# 6. Push to remote (THIS IS MANDATORY)
git push origin main

# 7. Clean up feature branch
git branch -d feature/sprint-35-1-emergency
```

**IMPORTANT REMINDERS**:
- ALWAYS push to main after completing work
- NEVER leave work uncommitted
- Use descriptive commit messages that explain WHAT changed
- If you fixed compilation errors, say so in the commit
- If you completed a sprint, mention it in the commit message

**Example commit messages**:
- `feat(sprint-35.1): complete emergency stability sprint - zero unwrap() calls`
- `fix: resolve 330 compilation errors in error handling refactor`
- `docs: update CURRENT.md with sprint completion status`
- `refactor: add bounds checking to prevent array access panics`

### 3. Terminal Safety (IMPORTANT)
Never run commands that block the terminal:
```bash
# ❌ WRONG - Blocks terminal
npm run dev
docker compose up

# ✅ CORRECT - Runs in background
nohup npm run dev > output.log 2>&1 &
docker compose up -d
```

### 4. Verification Process (REQUIRED)
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

### Engineering Principles (Data-Oriented Style)

#### Single Source of Truth
- Each piece of data lives in exactly ONE buffer
- Each fact documented in exactly ONE place
- No duplicated state between systems
- Reference, don't copy

#### DRY (Don't Repeat Yourself) - DOP Version
- Reuse data transformations, not object hierarchies
- Generic kernels over specialized systems
- Shared buffers over message passing
- If you're copying code, you're missing a pattern

#### KISS (Keep It Simple) - But Fast
- Simple data layouts → complex emergent behavior
- One buffer + one kernel > Ten interacting objects
- Prefer arrays over complex data structures
- The fastest code is code that doesn't exist

#### YAGNI (You Aren't Gonna Need It)
- Don't add buffers "for future use"
- Don't create abstractions without 3+ use cases
- Don't optimize without profiling
- Today's requirements only (but done right)

#### Principle of Least Surprise
- Voxels are always 1m³ (no exceptions)
- Errors always bubble up (no silent failures)
- Data flows one direction (no backchannels)
- Names mean exactly what they say

#### Separation of Concerns - Data Style
- Each buffer has ONE purpose
- Each kernel does ONE transformation
- Physics doesn't know about rendering
- Network doesn't know about gameplay

#### Design by Contract
- Functions document their data requirements
- Preconditions checked in debug (assumed in release)
- Buffer layouts are contracts between systems
- Breaking changes require version bumps

#### Fail Fast, Fail Loud
- Invalid states panic in debug builds
- Errors propagate immediately (no error hiding)
- Assumptions documented and verified
- Better to crash in dev than corrupt in prod

#### Make It Work, Make It Right, Make It Fast
1. **Work**: Get the feature functioning (even if slow)
2. **Right**: Clean up the data flow, remove hacks
3. **Fast**: Profile and optimize based on evidence
- Never skip step 2 to get to step 3

#### The Boy Scout Rule
- Leave code better than you found it
- Fix neighboring unwraps when touching a file
- Update stale comments as you go
- Small improvements compound over time

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
- **Sprint Details**: Check `/docs/MASTER_ROADMAP.md` for full sprint planning
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

## 🚨 CRITICAL: CLAIMS VS REALITY PREVENTION 🚨
**Earth Engine Specific**: This project has experienced severe "claims vs reality" gaps where agents report success without verification.

### Four Root Causes of False Claims (Identified in Sprint 36):
1. **Overconfidence Bias** - Assuming unwrap() elimination worked without checking rg counts
2. **Lack of Verification** - Not running `cargo check`, `cargo test` to prove claims
3. **Text Generation vs Reality** - Generating "mission accomplished" text without evidence
4. **Speed over Accuracy** - Rushing to claim sprint completion rather than verify

### Earth Engine Verification Requirements:
**MANDATORY FOR ALL WORK:**
- `cargo check --lib` MUST pass before claiming compilation success
- `cargo test --lib` MUST be attempted before claiming functionality
- `rg "\.unwrap\(\)" src --type rust -c` MUST be run to verify unwrap elimination
- `cargo run --example engine_testbed` MUST be tested for user experience
- All metrics must be measured, never estimated

### Sprint Completion Requirements:
- **NO sprint can be marked complete without QA verification**
- **Agent reports of "success" mean nothing without command evidence**
- **Every deliverable requires independent verification with actual commands**
- **"Attempted" vs "Accomplished" language until verified**

### Earth Engine Specific Checks:
- **Player Movement**: Verify WASD keys work with actual testing
- **Spawn System**: Verify safe spawn with actual gameplay
- **Save/Load**: Verify no corruption with actual save/load cycles
- **Performance**: Verify claims with actual benchmarks
- **DOP Conversion**: Verify allocation counts with profiling

### Language for Earth Engine:
- ❌ "Zero unwraps achieved" → ✅ "Targeted unwraps, running rg count..."
- ❌ "Sprint 36 complete" → ✅ "Sprint 36 changes made, running QA..."
- ❌ "Player movement fixed" → ✅ "Modified movement code, testing WASD..."
- ❌ "Engine ready for production" → ✅ "Engine compiles, testing functionality..."

**Earth Engine Principle**: Show, don't tell. Prove, don't claim.

## VISION REMINDERS
This engine enables the ultimate MMO - Earth:
- **"Real life but fun"** - Complex societies emerge from simple physics
- **Every voxel transformable** - Build cities, destroy mountains, redirect rivers
- **No forced gameplay** - Those "1000 players simulate civilization" videos? Just ONE way to play
- **Player-created everything**:
  - Cities with actual smithies, taverns, mayors (not NPCs - real players)
  - Confederacies of villages with complex politics
  - Roaming bandit clans with hideouts
  - Hermit scholars preserving ancient knowledge
  - Trade empires spanning continents
- **Communication mirrors reality**:
  - Voice propagates through actual physics - not arbitrary distance
  - Sound waves blocked by walls, carried by tunnels, echo in caves
  - Material acoustics matter - concrete muffles, wood transmits
  - Stealth is physical - footsteps on stone vs grass
  - No global chat - messages travel physically
  - Write signs, books, letters - all hand-copied
  - Dye and sew fabric for banners, flags, uniforms
  - Late-game radio tech enables long-distance comms
- **Visual identity emerges**:
  - Nations design their own flags from dyed fabric
  - Guilds create banners to mark territory
  - Shops hang signs with hand-written text
  - All player-created, no presets
- **Blend of the best**:
  - Minecraft's building and exploration
  - Rust's survival and raiding
  - Garry's Mod's creative emergence
  - EVE Online's player politics
  - SimCity's urban planning
- **Full physics simulation per voxel**:
  - Thermal dynamics - heat spreads, fire propagates
  - Fluid dynamics - water flows, pressure matters  
  - Sound physics - waves propagate, materials absorb/reflect
  - Structural integrity - buildings can collapse
  - Material properties - density, conductivity, acoustics
- **Physics enables stories**:
  - Build soundproof rooms for secret meetings
  - Castle walls block both arrows AND eavesdroppers
  - Footsteps on stone alert guards
  - Caves carry whispers for kilometers
  - Thunder echoes off mountains
- **Knowledge has weight** - Technologies spread through teaching, not wikis

The engine is the laws of physics. Players are the force of history.

Every line of code should enable player creativity, not constrain it. Remember: we're building a game-agnostic ENGINE first.

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