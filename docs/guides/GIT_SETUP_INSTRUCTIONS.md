# Git Setup Instructions for Hearth Engine

## Current Status
- Git repository initialized locally in Linux Mint
- Initial commit made with all project files
- Ready to connect to remote repository

## Setting Up Remote Repository

### Step 1: Create GitHub/GitLab/Bitbucket Repository
1. Create a new repository on your preferred platform
2. Name it `hearth-engine` or similar
3. Do NOT initialize with README, .gitignore, or license (we already have these)

### Step 2: Connect Local to Remote
```bash
# Add remote origin (replace URL with your repository URL)
git remote add origin https://github.com/YOUR_USERNAME/hearth-engine.git

# Verify remote was added
git remote -v

# Push to remote
git push -u origin master
```

## Development Workflow

### Working with Git
```bash
# Make changes in code...

# Check status
git status

# Add changes
git add .

# Commit
git commit -m "Description of changes"

# Push to remote
git push
```


## Best Practices

1. **Always pull before starting work**
   ```bash
   git pull
   ```

2. **Commit frequently with clear messages**
   ```bash
   git commit -m "feat: Add GPU-driven rendering pipeline"
   git commit -m "fix: Resolve culling shader compilation"
   git commit -m "perf: Optimize instance buffer updates"
   ```

3. **Work directly in Linux Mint**
   - Full native performance
   - Direct GPU access
   - No synchronization needed

4. **Use branches for major features**
   ```bash
   git checkout -b feature/sprint-20-gpu-rendering
   # Work on feature...
   git push -u origin feature/sprint-20-gpu-rendering
   ```

## Current Repository State

- **Total files**: 181
- **Lines of code**: ~38,000
- **Completed sprints**: 1-19
- **Current sprint**: 20 (GPU-Driven Rendering)

## Troubleshooting

### If you see merge conflicts:
```bash
# Stash your changes
git stash

# Pull latest
git pull

# Apply your changes
git stash pop

# Resolve conflicts manually, then:
git add .
git commit -m "Resolve merge conflicts"
```

### If Windows line endings cause issues:
```bash
# Configure git to handle line endings
git config --global core.autocrlf true  # On Windows
git config --global core.autocrlf input  # On Linux/WSL
```

## Next Steps

1. Create remote repository
2. Add remote origin using the commands above
3. Push initial commit
4. Clone on Windows side
5. Continue development with proper version control!

This approach is much safer than sync scripts and gives you:
- Full history of all changes
- Ability to revert if needed
- Collaboration capabilities
- Backup of your work
- Clear separation between development (Linux) and testing (Windows)