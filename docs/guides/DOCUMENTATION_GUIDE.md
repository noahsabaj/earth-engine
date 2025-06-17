# Hearth Engine Documentation Guide

## Purpose
This guide ensures documentation coherence and prevents redundancy as the project evolves.

## Documentation Structure

```
hearth-engine/
├── README.md                           # Project overview and quick start
├── MASTER_ROADMAP.md                  # Complete sprint timeline (source of truth)
├── ENGINE_VISION.md                   # High-level vision and USPs
├── HEARTH_ENGINE_VISION_2025.md        # Revolutionary game design vision
├── DATA_ORIENTED_TRANSITION_PLAN.md   # Architecture transition strategy
├── ENVIRONMENT_COHERENCE.md           # Dev/test environment guide
├── DUPLICATE_FILES_ANALYSIS.md        # Code cleanup tracking
└── docs/
    ├── DOCUMENTATION_GUIDE.md         # This file
    ├── SPRINT_XX_SUMMARY.md          # Completed sprint summaries
    └── ROADMAP_ARCHIVED.md           # Historical reference

Future structure:
├── vision/                            # Vision documents
├── technical/                         # Technical guides
└── sprints/                          # Sprint summaries
```

## Document Purposes

### Root Level Documents

1. **README.md**
   - First point of contact
   - Project overview
   - Quick start guide
   - Links to other docs

2. **MASTER_ROADMAP.md**
   - THE source of truth for sprints
   - Complete timeline (Sprints 1-34)
   - Performance metrics
   - Technical evolution

3. **ENGINE_VISION.md**
   - Marketing-oriented vision
   - Unique selling points
   - Why this matters

4. **EARTH_ENGINE_VISION_2025.md**
   - Detailed game design
   - Revolutionary mechanics
   - Server architecture
   - Physical information economy

5. **DATA_ORIENTED_TRANSITION_PLAN.md**
   - Technical strategy
   - Sprint 21 as pivot point
   - Migration approach

6. **ENVIRONMENT_COHERENCE.md**
   - WSL/Windows sync
   - Development workflow
   - GPU testing guide

7. **DUPLICATE_FILES_ANALYSIS.md**
   - Code cleanup decisions
   - File consolidation tracking

### Sprint Documentation

- **SPRINT_XX_SUMMARY.md**: Created at sprint completion
- Follow SPRINT_12_SUMMARY.md format
- Include: Overview, Achievements, Performance, Technical Details, Lessons

## Documentation Rules

### 1. Single Source of Truth
- Sprint planning: MASTER_ROADMAP.md ONLY
- Never create duplicate roadmaps
- Reference, don't duplicate

### 2. Update Triggers
- Sprint completion → Create SPRINT_XX_SUMMARY.md
- Sprint completion → Update MASTER_ROADMAP.md status
- Architecture change → Update relevant vision docs
- New sprint → Update MASTER_ROADMAP.md

### 3. Cross-References
- Use relative links: `[MASTER_ROADMAP.md](../MASTER_ROADMAP.md)`
- Link to specific sections when relevant
- Keep links updated during refactoring

### 4. Naming Conventions
- UPPERCASE.md for major documents
- SPRINT_XX_SUMMARY.md for sprint summaries
- Descriptive names that indicate scope

### 5. Content Guidelines
- Be concise but complete
- Include code examples where helpful
- Add performance metrics
- Document rationale for major decisions

## Maintenance Checklist

### At Sprint Start
- [ ] Review sprint goals in MASTER_ROADMAP.md
- [ ] Update README.md "Next Sprint" section
- [ ] Check ENVIRONMENT_COHERENCE.md is current

### During Sprint
- [ ] Document major decisions in sprint work
- [ ] Update technical docs if architecture changes
- [ ] Keep performance metrics

### At Sprint Completion
- [ ] Create SPRINT_XX_SUMMARY.md
- [ ] Update MASTER_ROADMAP.md status to ✅
- [ ] Update README.md current status
- [ ] Sync all docs to Windows
- [ ] Archive any outdated documents

### Quarterly
- [ ] Review all docs for outdated information
- [ ] Consolidate redundant documents
- [ ] Update vision docs if direction changes
- [ ] Clean up broken links

## Common Pitfalls to Avoid

1. **Creating new roadmap documents** - Use MASTER_ROADMAP.md
2. **Duplicating information** - Reference instead
3. **Forgetting to update status** - Use the checklist
4. **Letting docs drift** - Regular reviews
5. **Over-documenting** - Keep it focused

## Future Improvements

When the project grows, consider:
1. Moving to directory structure (vision/, technical/, sprints/)
2. Automated link checking
3. Documentation generation from code
4. Version-specific documentation

---

By following this guide, Hearth Engine documentation will remain coherent, up-to-date, and useful throughout the project's evolution.