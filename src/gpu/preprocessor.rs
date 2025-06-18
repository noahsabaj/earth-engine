use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fs;

/// Simple WGSL preprocessor that handles #include directives
pub struct WgslPreprocessor {
    include_dirs: Vec<PathBuf>,
    processed_files: HashSet<PathBuf>,
}

impl WgslPreprocessor {
    pub fn new() -> Self {
        Self {
            include_dirs: vec![],
            processed_files: HashSet::new(),
        }
    }

    /// Add a directory to search for include files
    pub fn add_include_dir<P: AsRef<Path>>(&mut self, path: P) {
        self.include_dirs.push(path.as_ref().to_path_buf());
    }

    /// Process a WGSL file, resolving all #include directives
    pub fn process_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String, std::io::Error> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        self.process_content(&content, path)
    }

    /// Process WGSL content, resolving all #include directives
    pub fn process_content(&mut self, content: &str, current_file: &Path) -> Result<String, std::io::Error> {
        let mut result = String::new();
        let parent_dir = current_file.parent();

        for line in content.lines() {
            if let Some(include_path) = Self::parse_include_directive(line) {
                // Try to resolve the include path
                let resolved_path = self.resolve_include_path(&include_path, parent_dir)?;
                
                // Prevent circular includes
                if !self.processed_files.contains(&resolved_path) {
                    self.processed_files.insert(resolved_path.clone());
                    
                    // Recursively process the included file
                    let included_content = fs::read_to_string(&resolved_path)?;
                    let processed = self.process_content(&included_content, &resolved_path)?;
                    
                    result.push_str("// Begin include: ");
                    result.push_str(&include_path);
                    result.push('\n');
                    result.push_str(&processed);
                    result.push_str("\n// End include: ");
                    result.push_str(&include_path);
                    result.push('\n');
                } else {
                    // Skip circular include
                    result.push_str("// Skipped circular include: ");
                    result.push_str(&include_path);
                    result.push('\n');
                }
            } else {
                // Regular line, just append
                result.push_str(line);
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// Parse an #include directive from a line
    fn parse_include_directive(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.starts_with("#include") {
            // Support both #include "file.wgsl" and #include <file.wgsl>
            let after_include = trimmed.trim_start_matches("#include").trim();
            
            if after_include.starts_with('"') && after_include.ends_with('"') {
                Some(after_include.trim_matches('"').to_string())
            } else if after_include.starts_with('<') && after_include.ends_with('>') {
                Some(after_include.trim_start_matches('<').trim_end_matches('>').to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Resolve an include path by searching include directories
    fn resolve_include_path(&self, include_path: &str, current_dir: Option<&Path>) -> Result<PathBuf, std::io::Error> {
        let include_path = Path::new(include_path);
        
        // First try relative to current file
        if let Some(dir) = current_dir {
            let candidate = dir.join(include_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        
        // Then try each include directory
        for include_dir in &self.include_dirs {
            let candidate = include_dir.join(include_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        
        // If not found anywhere, return error
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Could not find include file: {}", include_path.display())
        ))
    }
}

/// Process a shader at runtime, resolving includes
pub fn preprocess_shader(shader_path: &Path) -> Result<String, std::io::Error> {
    let mut preprocessor = WgslPreprocessor::new();
    
    // Add GPU shaders directory as include path
    if let Some(parent) = shader_path.parent() {
        preprocessor.add_include_dir(parent);
    }
    
    // Add generated shaders directory
    preprocessor.add_include_dir("src/gpu/shaders/generated");
    
    preprocessor.process_file(shader_path)
}

/// Process shader content at runtime
pub fn preprocess_shader_content(content: &str, base_path: &Path) -> Result<String, std::io::Error> {
    let mut preprocessor = WgslPreprocessor::new();
    
    // Add GPU shaders directories as include paths
    preprocessor.add_include_dir("src/gpu/shaders");
    preprocessor.add_include_dir("src/gpu/shaders/generated");
    preprocessor.add_include_dir("src/world_gpu/shaders");
    
    preprocessor.process_content(content, base_path)
}