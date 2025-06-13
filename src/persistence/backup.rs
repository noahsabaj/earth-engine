use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, Duration};
use chrono::Local;

use crate::persistence::{
    PersistenceResult, PersistenceError,
    WorldMetadata, Compressor, CompressionType, CompressionLevel,
    atomic_write,
};

/// Backup policy configuration
#[derive(Debug, Clone)]
pub struct BackupPolicy {
    /// Maximum number of backups to keep
    pub max_backups: usize,
    /// Minimum time between backups
    pub min_interval: Duration,
    /// Whether to compress backups
    pub compress: bool,
    /// Compression type for backups
    pub compression_type: CompressionType,
    /// Automatic backup triggers
    pub triggers: BackupTriggers,
    /// Backup retention policy
    pub retention: RetentionPolicy,
}

/// Events that trigger automatic backups
#[derive(Debug, Clone)]
pub struct BackupTriggers {
    /// Backup before world upgrade
    pub before_upgrade: bool,
    /// Backup on server shutdown
    pub on_shutdown: bool,
    /// Backup on manual save
    pub on_manual_save: bool,
    /// Periodic backup interval (None = disabled)
    pub periodic_interval: Option<Duration>,
}

/// Backup retention policy
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Keep all backups from last N hours
    pub keep_hourly: usize,
    /// Keep daily backups for N days
    pub keep_daily: usize,
    /// Keep weekly backups for N weeks
    pub keep_weekly: usize,
    /// Keep monthly backups for N months
    pub keep_monthly: usize,
}

impl Default for BackupPolicy {
    fn default() -> Self {
        Self {
            max_backups: 10,
            min_interval: Duration::from_secs(3600), // 1 hour
            compress: true,
            compression_type: CompressionType::Zstd,
            triggers: BackupTriggers::default(),
            retention: RetentionPolicy::default(),
        }
    }
}

impl Default for BackupTriggers {
    fn default() -> Self {
        Self {
            before_upgrade: true,
            on_shutdown: true,
            on_manual_save: false,
            periodic_interval: Some(Duration::from_secs(86400)), // Daily
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_hourly: 24,   // Last 24 hours
            keep_daily: 7,     // Last 7 days
            keep_weekly: 4,    // Last 4 weeks
            keep_monthly: 3,   // Last 3 months
        }
    }
}

/// Manages world backups
pub struct BackupManager {
    /// Backup directory
    backup_dir: PathBuf,
    /// Backup policy
    policy: BackupPolicy,
    /// Last backup time
    last_backup: Option<SystemTime>,
}

/// Information about a backup
#[derive(Debug)]
pub struct BackupInfo {
    /// Backup name
    pub name: String,
    /// Full path to backup
    pub path: PathBuf,
    /// When the backup was created
    pub created_at: SystemTime,
    /// Size of backup in bytes
    pub size: u64,
    /// Whether backup is compressed
    pub compressed: bool,
    /// World metadata if available
    pub metadata: Option<WorldMetadata>,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new<P: AsRef<Path>>(save_dir: P, policy: BackupPolicy) -> Self {
        let backup_dir = save_dir.as_ref().join("backups");
        
        Self {
            backup_dir,
            policy,
            last_backup: None,
        }
    }
    
    /// Create a backup of the world
    pub fn create_backup<P: AsRef<Path>>(
        &mut self,
        world_dir: P,
        reason: BackupReason,
    ) -> PersistenceResult<BackupInfo> {
        // Check minimum interval
        if let Some(last) = self.last_backup {
            let elapsed = SystemTime::now().duration_since(last)
                .unwrap_or(Duration::ZERO);
            
            if elapsed < self.policy.min_interval && !reason.is_critical() {
                return Err(PersistenceError::BackupError(
                    format!("Backup too soon, please wait {} seconds",
                        (self.policy.min_interval - elapsed).as_secs())
                ));
            }
        }
        
        // Create backup directory
        fs::create_dir_all(&self.backup_dir)?;
        
        // Generate backup name
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("backup_{}_{}", timestamp, reason.suffix());
        
        let backup_path = if self.policy.compress {
            self.backup_dir.join(format!("{}.tar.zst", backup_name))
        } else {
            self.backup_dir.join(&backup_name)
        };
        
        // Create backup
        let size = if self.policy.compress {
            self.create_compressed_backup(world_dir.as_ref(), &backup_path)?
        } else {
            self.create_uncompressed_backup(world_dir.as_ref(), &backup_path)?
        };
        
        // Update last backup time
        self.last_backup = Some(SystemTime::now());
        
        // Clean old backups
        self.cleanup_old_backups()?;
        
        Ok(BackupInfo {
            name: backup_name,
            path: backup_path,
            created_at: SystemTime::now(),
            size,
            compressed: self.policy.compress,
            metadata: None, // TODO: Load metadata
        })
    }
    
    /// Create uncompressed backup (copy directory)
    fn create_uncompressed_backup(&self, source: &Path, dest: &Path) -> PersistenceResult<u64> {
        copy_dir_recursive(source, dest)?;
        
        // Calculate total size
        let size = calculate_dir_size(dest)?;
        Ok(size)
    }
    
    /// Create compressed backup
    fn create_compressed_backup(&self, source: &Path, dest: &Path) -> PersistenceResult<u64> {
        use tar::Builder;
        use std::fs::File;
        
        // Create tar archive
        let _tar_file = File::create(dest)?;
        let compressor = Compressor::new(
            self.policy.compression_type,
            CompressionLevel::Default,
        );
        
        // For now, use a simple approach - tar then compress
        let temp_tar = dest.with_extension("tar");
        {
            let tar_file = File::create(&temp_tar)?;
            let mut tar = Builder::new(tar_file);
            
            // Add all files to tar
            tar.append_dir_all(".", source)?;
            tar.finish()?;
        }
        
        // Compress the tar file
        let tar_data = fs::read(&temp_tar)?;
        let compressed = compressor.compress(&tar_data)?;
        atomic_write(dest, &compressed)?;
        
        // Clean up temp tar
        fs::remove_file(temp_tar)?;
        
        // Get final size
        let metadata = fs::metadata(dest)?;
        Ok(metadata.len())
    }
    
    /// Restore a backup
    pub fn restore_backup<P: AsRef<Path>>(
        &self,
        backup_path: P,
        destination: P,
    ) -> PersistenceResult<()> {
        let backup_path = backup_path.as_ref();
        let destination = destination.as_ref();
        
        // Check if backup exists
        if !backup_path.exists() {
            return Err(PersistenceError::BackupError(
                "Backup not found".to_string()
            ));
        }
        
        // Clear destination
        if destination.exists() {
            fs::remove_dir_all(destination)?;
        }
        
        // Restore based on type
        if backup_path.extension() == Some("zst".as_ref()) {
            self.restore_compressed_backup(backup_path, destination)?;
        } else {
            copy_dir_recursive(backup_path, destination)?;
        }
        
        Ok(())
    }
    
    /// Restore compressed backup
    fn restore_compressed_backup(&self, backup_path: &Path, dest: &Path) -> PersistenceResult<()> {
        use tar::Archive;
        
        // Read and decompress
        let compressed_data = fs::read(backup_path)?;
        let compressor = Compressor::new(
            self.policy.compression_type,
            CompressionLevel::Default,
        );
        let tar_data = compressor.decompress(&compressed_data)?;
        
        // Extract tar
        let mut archive = Archive::new(tar_data.as_slice());
        fs::create_dir_all(dest)?;
        archive.unpack(dest)?;
        
        Ok(())
    }
    
    /// List all backups
    pub fn list_backups(&self) -> PersistenceResult<Vec<BackupInfo>> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() || metadata.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let compressed = path.extension() == Some("zst".as_ref());
                    
                    backups.push(BackupInfo {
                        name,
                        path,
                        created_at: metadata.modified()?,
                        size: if metadata.is_file() {
                            metadata.len()
                        } else {
                            calculate_dir_size(&entry.path())?
                        },
                        compressed,
                        metadata: None,
                    });
                }
            }
        }
        
        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(backups)
    }
    
    /// Delete a backup
    pub fn delete_backup<P: AsRef<Path>>(&self, backup_path: P) -> PersistenceResult<()> {
        let path = backup_path.as_ref();
        
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }
    
    /// Clean up old backups according to policy
    fn cleanup_old_backups(&self) -> PersistenceResult<()> {
        let backups = self.list_backups()?;
        
        // Simple policy: keep only max_backups newest
        if backups.len() > self.policy.max_backups {
            for backup in &backups[self.policy.max_backups..] {
                self.delete_backup(&backup.path)?;
            }
        }
        
        // TODO: Implement retention policy (hourly/daily/weekly/monthly)
        
        Ok(())
    }
    
    /// Get total size of all backups
    pub fn get_total_backup_size(&self) -> PersistenceResult<u64> {
        let backups = self.list_backups()?;
        Ok(backups.iter().map(|b| b.size).sum())
    }
    
    /// Verify backup integrity
    pub fn verify_backup<P: AsRef<Path>>(&self, backup_path: P) -> PersistenceResult<bool> {
        let path = backup_path.as_ref();
        
        if !path.exists() {
            return Ok(false);
        }
        
        // For compressed backups, try to decompress
        if path.extension() == Some("zst".as_ref()) {
            let data = fs::read(path)?;
            let compressor = Compressor::new(
                self.policy.compression_type,
                CompressionLevel::Default,
            );
            
            match compressor.decompress(&data) {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            // For directory backups, check if metadata exists
            Ok(path.join("world.meta").exists())
        }
    }
}

/// Reason for creating a backup
#[derive(Debug, Clone)]
pub enum BackupReason {
    Manual,
    Automatic,
    BeforeUpgrade,
    Shutdown,
    Error,
}

impl BackupReason {
    fn suffix(&self) -> &str {
        match self {
            BackupReason::Manual => "manual",
            BackupReason::Automatic => "auto",
            BackupReason::BeforeUpgrade => "upgrade",
            BackupReason::Shutdown => "shutdown",
            BackupReason::Error => "error",
        }
    }
    
    fn is_critical(&self) -> bool {
        matches!(self, BackupReason::BeforeUpgrade | BackupReason::Error)
    }
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> PersistenceResult<()> {
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

/// Calculate total size of directory
fn calculate_dir_size(path: &Path) -> PersistenceResult<u64> {
    let mut size = 0;
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        if metadata.is_file() {
            size += metadata.len();
        } else if metadata.is_dir() {
            size += calculate_dir_size(&entry.path())?;
        }
    }
    
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_backup_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let world_dir = temp_dir.path().join("world");
        fs::create_dir_all(&world_dir).expect("Failed to create world directory");
        
        // Create some test data
        fs::write(world_dir.join("test.txt"), "test data").expect("Failed to write test data");
        
        let mut manager = BackupManager::new(temp_dir.path(), BackupPolicy::default());
        
        let backup = manager.create_backup(&world_dir, BackupReason::Manual).expect("Failed to create backup");
        assert!(backup.path.exists());
    }
    
    #[test]
    fn test_backup_restore() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let world_dir = temp_dir.path().join("world");
        fs::create_dir_all(&world_dir).expect("Failed to create world directory");
        fs::write(world_dir.join("test.txt"), "test data").expect("Failed to write test data");
        
        let mut manager = BackupManager::new(temp_dir.path(), BackupPolicy::default());
        
        // Create backup
        let backup = manager.create_backup(&world_dir, BackupReason::Manual).expect("Failed to create backup");
        
        // Delete original
        fs::remove_dir_all(&world_dir).expect("Failed to delete original world directory");
        
        // Restore
        let restore_dir = temp_dir.path().join("restored");
        manager.restore_backup(&backup.path, &restore_dir).expect("Failed to restore backup");
        
        // Verify
        assert!(restore_dir.join("test.txt").exists());
        let content = fs::read_to_string(restore_dir.join("test.txt")).expect("Failed to read restored test file");
        assert_eq!(content, "test data");
    }
}