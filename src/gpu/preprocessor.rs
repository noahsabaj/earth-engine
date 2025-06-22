use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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
    pub fn process_content(
        &mut self,
        content: &str,
        current_file: &Path,
    ) -> Result<String, std::io::Error> {
        let mut result = String::new();
        let parent_dir = current_file.parent();

        for line in content.lines() {
            if let Some(include_path) = Self::parse_include_directive(line) {
                // First check if this is an embedded include
                if let Some(embedded) =
                    crate::gpu::shader_includes::get_shader_include(&include_path)
                {
                    // Use embedded content directly - no need for circular include checking
                    result.push_str("// Begin include: ");
                    result.push_str(&include_path);
                    result.push_str(" (embedded)\n");
                    result.push_str(embedded);
                    result.push_str("\n// End include: ");
                    result.push_str(&include_path);
                    result.push('\n');
                } else {
                    // Try to resolve the include path from filesystem
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
                Some(
                    after_include
                        .trim_start_matches('<')
                        .trim_end_matches('>')
                        .to_string(),
                )
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Resolve an include path by searching include directories
    fn resolve_include_path(
        &self,
        include_path: &str,
        current_dir: Option<&Path>,
    ) -> Result<PathBuf, std::io::Error> {
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
            format!("Could not find include file: {}", include_path.display()),
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

    // Add shader directories
    preprocessor.add_include_dir("src/shaders");
    preprocessor.add_include_dir("src/shaders/generated");
    preprocessor.add_include_dir("src/shaders/compute");
    preprocessor.add_include_dir("src/shaders/rendering");
    preprocessor.add_include_dir("src/shaders/mesh");

    preprocessor.process_file(shader_path)
}

/// Process shader content at runtime
pub fn preprocess_shader_content(
    content: &str,
    base_path: &Path,
) -> Result<String, std::io::Error> {
    let mut preprocessor = WgslPreprocessor::new();

    // Get the executable directory for cross-platform compatibility
    let exe_path = std::env::current_exe().ok();
    let exe_dir = exe_path.as_ref().and_then(|p| p.parent());

    // Try multiple possible locations for the generated shader files
    // This handles both development (cargo run) and release scenarios
    let possible_roots = vec![
        // Development: relative to current directory
        PathBuf::from("."),
        // Release: relative to executable
        exe_dir.map(|d| d.to_path_buf()).unwrap_or_default(),
        // Cargo workspace root (if running from workspace)
        PathBuf::from(".."),
    ];

    for root in &possible_roots {
        // Add shader directories as include paths
        preprocessor.add_include_dir(root.join("src/shaders"));
        preprocessor.add_include_dir(root.join("src/shaders/generated"));
        preprocessor.add_include_dir(root.join("src/shaders/compute"));
        preprocessor.add_include_dir(root.join("src/shaders/rendering"));
        preprocessor.add_include_dir(root.join("src/shaders/mesh"));

        // Also try without src/ prefix (for installed/deployed scenarios)
        preprocessor.add_include_dir(root.join("shaders"));
        preprocessor.add_include_dir(root.join("shaders/generated"));
        preprocessor.add_include_dir(root.join("shaders/compute"));
        preprocessor.add_include_dir(root.join("shaders/rendering"));
        preprocessor.add_include_dir(root.join("shaders/mesh"));
    }

    // Add parent directory of base_path if it exists
    if let Some(parent) = base_path.parent() {
        preprocessor.add_include_dir(parent);
    }

    preprocessor.process_content(content, base_path)
}
