# Windows Git Sync Guide

## Setting up your Windows Rider project to sync with GitHub

### 1. Open Git Bash or Terminal in your Windows project directory
Navigate to your earth-engine project folder in Windows (where your Cargo.toml is located).

### 2. Check current Git status
```bash
git status
```

If it shows "not a git repository", initialize it:
```bash
git init
```

### 3. Add the GitHub remote
```bash
git remote add origin https://github.com/noahsabaj/earth-engine.git
```

If you already have an origin, update it:
```bash
git remote set-url origin https://github.com/noahsabaj/earth-engine.git
```

### 4. Fetch the latest changes from GitHub
```bash
git fetch origin
```

### 5. Set up tracking and pull latest
```bash
# First, stash any local changes you want to keep
git stash

# Set main branch to track origin/main
git branch --set-upstream-to=origin/main main

# Pull the latest changes
git pull origin main

# If you stashed changes, apply them back
git stash pop
```

### 6. Configure Git credentials (one-time setup)
```bash
git config --global user.name "noahsabaj"
git config --global user.email "your-email@example.com"
```

## Daily Workflow

### Before starting work (pull latest changes):
```bash
git pull origin main
```

### After making changes (push your work):
```bash
git add .
git commit -m "Your commit message"
git push origin main
```

### If you get authentication prompts:
Use your GitHub username and Personal Access Token (PAT) as the password.

## Syncing Between Windows and Linux

### On Windows (after making changes):
```bash
git add .
git commit -m "Work from Windows"
git push origin main
```

### On Linux (to get Windows changes):
```bash
git pull origin main
```

### Vice versa - same process!

## Handling Conflicts

If you get merge conflicts:
1. `git status` - see conflicted files
2. Open conflicted files in Rider
3. Look for `<<<<<<<`, `=======`, `>>>>>>>` markers
4. Resolve conflicts manually
5. `git add .`
6. `git commit -m "Resolved conflicts"`
7. `git push origin main`

## Rider Integration

Rider has built-in Git support:
- VCS menu → Git → Pull/Push
- Bottom right corner shows branch name
- Commit window: Ctrl+K
- Update project: Ctrl+T

## Tips
- Always pull before starting work
- Commit frequently with clear messages
- Push at end of each work session
- Use `.gitignore` for files that shouldn't be synced