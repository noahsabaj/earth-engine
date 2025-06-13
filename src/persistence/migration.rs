use std::path::Path;
use std::collections::HashMap;

use crate::persistence::{
    PersistenceResult, PersistenceError,
    SaveVersion, WorldMetadata, WorldSave,
};

/// Trait for save file migrations
pub trait Migration: Send + Sync {
    /// Version this migration upgrades from
    fn from_version(&self) -> SaveVersion;
    
    /// Version this migration upgrades to
    fn to_version(&self) -> SaveVersion;
    
    /// Description of what this migration does
    fn description(&self) -> &str;
    
    /// Check if this migration can be applied
    fn can_apply(&self, metadata: &WorldMetadata) -> bool {
        metadata.version == self.from_version()
    }
    
    /// Apply the migration
    fn apply(&self, save_dir: &Path, metadata: &mut WorldMetadata) -> PersistenceResult<()>;
    
    /// Estimate time to complete migration (in seconds)
    fn estimate_duration(&self, save_dir: &Path) -> u64 {
        60 // Default: 1 minute
    }
}

/// Manages save file migrations
pub struct MigrationManager {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new() -> Self {
        let mut manager = Self {
            migrations: Vec::new(),
        };
        
        // Register built-in migrations
        manager.register_builtin_migrations();
        
        manager
    }
    
    /// Register a migration
    pub fn register(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }
    
    /// Register built-in migrations
    fn register_builtin_migrations(&mut self) {
        // Example migrations would go here
        // self.register(Box::new(MigrationV0ToV1));
    }
    
    /// Find migration path from one version to another
    pub fn find_migration_path(&self, from: SaveVersion, to: SaveVersion) -> Option<Vec<&dyn Migration>> {
        if from == to {
            return Some(Vec::new());
        }
        
        // Simple linear search for now
        // In production, use graph-based pathfinding
        let mut path = Vec::new();
        let mut current = from;
        
        while current != to {
            let migration = self.migrations.iter()
                .find(|m| m.from_version() == current && m.to_version() <= to)?;
            
            current = migration.to_version();
            path.push(migration.as_ref());
        }
        
        Some(path)
    }
    
    /// Check if migration is possible
    pub fn can_migrate(&self, from: SaveVersion, to: SaveVersion) -> bool {
        self.find_migration_path(from, to).is_some()
    }
    
    /// Migrate a world save
    pub fn migrate_world(&self, save_dir: &Path, target_version: SaveVersion) -> PersistenceResult<()> {
        // Load metadata
        let world_save = WorldSave::load(save_dir)?;
        let mut metadata = world_save.load_metadata()
            .map_err(|e| PersistenceError::MigrationError(format!("Failed to load metadata: {}", e)))?;
        
        // Find migration path
        let path = self.find_migration_path(metadata.version, target_version)
            .ok_or_else(|| PersistenceError::MigrationError(
                format!("No migration path from {} to {}", metadata.version, target_version)
            ))?;
        
        if path.is_empty() {
            return Ok(()); // Already at target version
        }
        
        // Create backup before migration
        let backup_dir = save_dir.with_extension("pre_migration_backup");
        self.backup_world(save_dir, &backup_dir)?;
        
        // Apply migrations
        println!("Starting migration from {} to {}", metadata.version, target_version);
        
        for migration in path {
            println!("Applying migration: {}", migration.description());
            
            migration.apply(save_dir, &mut metadata)?;
            
            // Update version and save metadata after each step
            metadata.version = migration.to_version();
            world_save.save_metadata(&metadata)?;
            
            println!("Migration step completed: now at version {}", metadata.version);
        }
        
        println!("Migration completed successfully!");
        Ok(())
    }
    
    /// Create a backup of the world
    fn backup_world(&self, source: &Path, destination: &Path) -> PersistenceResult<()> {
        use std::fs;
        
        if destination.exists() {
            fs::remove_dir_all(destination)?;
        }
        
        fs::create_dir_all(destination)?;
        
        // Copy all files recursively
        copy_dir_recursive(source, destination)?;
        
        Ok(())
    }
    
    /// Get migration summary
    pub fn get_migration_summary(&self, from: SaveVersion, to: SaveVersion) -> Option<MigrationSummary> {
        let path = self.find_migration_path(from, to)?;
        
        let steps: Vec<MigrationStep> = path.iter()
            .map(|m| MigrationStep {
                from: m.from_version(),
                to: m.to_version(),
                description: m.description().to_string(),
                estimated_duration: 60, // Default estimate
            })
            .collect();
        
        let total_duration = steps.iter().map(|s| s.estimated_duration).sum();
        
        Some(MigrationSummary {
            from_version: from,
            to_version: to,
            steps,
            total_steps: path.len(),
            estimated_duration: total_duration,
        })
    }
}

/// Summary of a migration process
#[derive(Debug)]
pub struct MigrationSummary {
    pub from_version: SaveVersion,
    pub to_version: SaveVersion,
    pub steps: Vec<MigrationStep>,
    pub total_steps: usize,
    pub estimated_duration: u64,
}

/// Individual migration step
#[derive(Debug)]
pub struct MigrationStep {
    pub from: SaveVersion,
    pub to: SaveVersion,
    pub description: String,
    pub estimated_duration: u64,
}

/// Example migration from v0.1.0 to v0.2.0
struct MigrationV0_1_0ToV0_2_0;

impl Migration for MigrationV0_1_0ToV0_2_0 {
    fn from_version(&self) -> SaveVersion {
        SaveVersion::new(0, 1, 0)
    }
    
    fn to_version(&self) -> SaveVersion {
        SaveVersion::new(0, 2, 0)
    }
    
    fn description(&self) -> &str {
        "Add item IDs to blocks and update inventory format"
    }
    
    fn apply(&self, save_dir: &Path, metadata: &mut WorldMetadata) -> PersistenceResult<()> {
        // Example migration logic
        println!("Migrating chunks to new format...");
        
        // In a real migration:
        // 1. Load all chunks
        // 2. Convert block format
        // 3. Save with new format
        // 4. Update player inventories
        
        // For now, just update metadata
        metadata.set_property("migration_v0_2_0".to_string(), "completed".to_string());
        
        Ok(())
    }
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> PersistenceResult<()> {
    use std::fs;
    
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// Migration validator to check if migrations are safe
pub struct MigrationValidator {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationValidator {
    pub fn new(migrations: Vec<Box<dyn Migration>>) -> Self {
        Self { migrations }
    }
    
    /// Validate all migrations
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check for version conflicts
        let mut version_map: HashMap<SaveVersion, Vec<&dyn Migration>> = HashMap::new();
        
        for migration in &self.migrations {
            version_map.entry(migration.from_version())
                .or_insert_with(Vec::new)
                .push(migration.as_ref());
        }
        
        // Check for multiple migrations from same version
        for (version, migrations) in &version_map {
            if migrations.len() > 1 {
                errors.push(format!(
                    "Multiple migrations from version {}: {}",
                    version,
                    migrations.iter()
                        .map(|m| m.description())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        
        // Check for circular dependencies
        for migration in &self.migrations {
            if migration.from_version() == migration.to_version() {
                errors.push(format!(
                    "Migration has same from and to version: {}",
                    migration.description()
                ));
            }
            
            if migration.from_version() > migration.to_version() {
                errors.push(format!(
                    "Migration downgrades version: {}",
                    migration.description()
                ));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestMigration {
        from: SaveVersion,
        to: SaveVersion,
        desc: String,
    }
    
    impl Migration for TestMigration {
        fn from_version(&self) -> SaveVersion { self.from }
        fn to_version(&self) -> SaveVersion { self.to }
        fn description(&self) -> &str { &self.desc }
        fn apply(&self, _: &Path, _: &mut WorldMetadata) -> PersistenceResult<()> {
            Ok(())
        }
    }
    
    #[test]
    fn test_migration_path() {
        let mut manager = MigrationManager::new();
        
        manager.register(Box::new(TestMigration {
            from: SaveVersion::new(1, 0, 0),
            to: SaveVersion::new(1, 1, 0),
            desc: "1.0 to 1.1".to_string(),
        }));
        
        manager.register(Box::new(TestMigration {
            from: SaveVersion::new(1, 1, 0),
            to: SaveVersion::new(1, 2, 0),
            desc: "1.1 to 1.2".to_string(),
        }));
        
        let path = manager.find_migration_path(
            SaveVersion::new(1, 0, 0),
            SaveVersion::new(1, 2, 0)
        ).expect("Migration path should exist");
        
        assert_eq!(path.len(), 2);
    }
    
    #[test]
    fn test_migration_validator() {
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(TestMigration {
                from: SaveVersion::new(1, 0, 0),
                to: SaveVersion::new(1, 0, 0),
                desc: "Invalid same version".to_string(),
            }),
        ];
        
        let validator = MigrationValidator::new(migrations);
        assert!(validator.validate().is_err());
    }
}