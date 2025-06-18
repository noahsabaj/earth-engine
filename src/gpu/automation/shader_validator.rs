//! Compile-time shader validation with enhanced error reporting
//! 
//! This module provides build-time validation of WGSL shaders with clear,
//! actionable error messages.

use std::collections::HashMap;
use wgpu::naga;

/// Shader validation result with enhanced error information
#[derive(Debug)]
pub enum ValidationResult {
    Ok,
    Error(ValidationError),
}

/// Enhanced shader validation error
#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
    pub location: Option<ErrorLocation>,
    pub suggestions: Vec<String>,
}

/// Error location information
#[derive(Debug)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub snippet: String,
}

/// Shader validator with enhanced error reporting
pub struct ShaderValidator {
    /// Naga module for validation
    module: Option<naga::Module>,
    /// Source mapping for error reporting
    source_map: HashMap<String, String>,
}

impl ShaderValidator {
    pub fn new() -> Self {
        Self {
            module: None,
            source_map: HashMap::new(),
        }
    }
    
    /// Validate a WGSL shader with enhanced error reporting
    pub fn validate_wgsl(&mut self, name: &str, source: &str) -> ValidationResult {
        // Store source for error reporting
        self.source_map.insert(name.to_string(), source.to_string());
        
        // Parse with naga
        let module = match naga::front::wgsl::parse_str(source) {
            Ok(module) => module,
            Err(error) => {
                return ValidationResult::Error(self.create_parse_error(name, source, error));
            }
        };
        
        // Validate module
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        
        match validator.validate(&module) {
            Ok(_) => {
                self.module = Some(module);
                ValidationResult::Ok
            }
            Err(error) => {
                // Extract the inner ValidationError from WithSpan
                let inner_error = error.into_inner();
                ValidationResult::Error(self.create_validation_error(name, source, inner_error))
            }
        }
    }
    
    /// Create enhanced parse error
    fn create_parse_error(
        &self,
        name: &str,
        source: &str,
        error: naga::front::wgsl::ParseError,
    ) -> ValidationError {
        // Extract location if available
        let (line_number, line_pos, snippet) = if let Some(location) = error.location(source) {
            let line_num = location.line_number as u32;
            let line_pos = location.line_position as u32;
            let snippet = self.create_error_snippet(source, line_num, line_pos);
            (line_num, line_pos, snippet)
        } else {
            (1, 1, "Unable to determine error location".to_string())
        };
        
        // Get the error line
        let lines: Vec<&str> = source.lines().collect();
        let error_line = lines.get(line_number.saturating_sub(1) as usize)
            .copied()
            .unwrap_or("");
        
        // Generate suggestions based on error
        let suggestions = self.generate_parse_suggestions(&error, error_line);
        
        ValidationError {
            message: format!("Shader parse error in {}: {}", name, error),
            location: Some(ErrorLocation {
                file: name.to_string(),
                line: line_number,
                column: line_pos,
                snippet,
            }),
            suggestions,
        }
    }
    
    /// Create enhanced validation error
    fn create_validation_error(
        &self,
        name: &str,
        _source: &str,
        error: naga::valid::ValidationError,
    ) -> ValidationError {
        let suggestions = self.generate_validation_suggestions(&error);
        
        ValidationError {
            message: format!("Shader validation error in {}: {:?}", name, error),
            location: None, // Validation errors don't have specific locations
            suggestions,
        }
    }
    
    /// Create error snippet with context
    fn create_error_snippet(&self, source: &str, line: u32, column: u32) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut snippet = String::new();
        
        // Show 2 lines before and after
        let start = line.saturating_sub(3) as usize;
        let end = (line + 2).min(lines.len() as u32) as usize;
        
        for i in start..end {
            let line_num = i + 1;
            let line_str = lines.get(i).copied().unwrap_or("");
            
            // Highlight the error line
            if line_num == line as usize {
                snippet.push_str(&format!("{:>4} | {}\n", line_num, line_str));
                snippet.push_str(&format!("     | {}^\n", " ".repeat(column as usize - 1)));
                snippet.push_str("     | Error occurs here\n");
            } else {
                snippet.push_str(&format!("{:>4} | {}\n", line_num, line_str));
            }
        }
        
        snippet
    }
    
    /// Generate suggestions for parse errors
    fn generate_parse_suggestions(&self, _error: &naga::front::wgsl::ParseError, line: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Since ParseError variants may change between naga versions,
        // we'll analyze the error string and line content
        let error_str = format!("{:?}", _error);
        
        // Check for common mistakes based on line content
        if line.contains("@binding") && !line.contains("@group") {
            suggestions.push("Add @group annotation before @binding".to_string());
        }
        
        if line.contains("var") && !line.contains(":") {
            suggestions.push("Add type annotation after variable name (e.g., var name: type)".to_string());
        }
        
        if line.contains("array<") && !line.contains(">") {
            suggestions.push("Close array type with '>'".to_string());
        }
        
        if error_str.contains("Unexpected") || error_str.contains("unexpected") {
            suggestions.push("Check for missing semicolons or brackets".to_string());
            suggestions.push("Verify correct WGSL syntax".to_string());
        }
        
        if error_str.contains("UnknownType") || error_str.contains("unknown type") {
            suggestions.push("Check type spelling and ensure all custom types are defined".to_string());
            suggestions.push("Common WGSL types: u32, i32, f32, vec2<f32>, vec3<f32>, vec4<f32>".to_string());
        }
        
        if error_str.contains("binding") {
            suggestions.push("Ensure binding syntax is: @group(N) @binding(M)".to_string());
        }
        
        // Always provide a generic suggestion
        suggestions.push("Refer to WGSL specification for correct syntax".to_string());
        
        suggestions
    }
    
    /// Generate suggestions for validation errors
    fn generate_validation_suggestions(&self, error: &naga::valid::ValidationError) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Since naga's ValidationError structure may vary between versions,
        // we provide generic suggestions based on the error string
        let error_str = format!("{:?}", error);
        
        if error_str.contains("Function") || error_str.contains("function") {
            suggestions.push("Check function signatures for correct parameter and return types".to_string());
            suggestions.push("Ensure all functions have proper stage annotations (@vertex, @fragment, @compute)".to_string());
        }
        
        if error_str.contains("Type") || error_str.contains("type") {
            suggestions.push("Ensure all types are properly defined and aligned".to_string());
            suggestions.push("Check that array sizes match between Rust and WGSL".to_string());
        }
        
        if error_str.contains("GlobalVariable") || error_str.contains("binding") {
            suggestions.push("Check global variable declarations and bindings".to_string());
            suggestions.push("Ensure @group and @binding annotations are correct".to_string());
        }
        
        if error_str.contains("entry point") {
            suggestions.push("Ensure shader has at least one entry point function".to_string());
            suggestions.push("Check that entry points have correct stage annotations".to_string());
        }
        
        // Always add a generic suggestion
        suggestions.push("Review shader code for WGSL syntax compliance".to_string());
        
        suggestions
    }
}

/// Macro for build-time shader validation
#[macro_export]
macro_rules! validate_shader {
    ($name:expr, $source:expr) => {{
        use $crate::gpu::automation::shader_validator::{ShaderValidator, ValidationResult};
        
        let mut validator = ShaderValidator::new();
        match validator.validate_wgsl($name, $source) {
            ValidationResult::Ok => Ok(()),
            ValidationResult::Error(error) => {
                let mut msg = format!("\n{}\n", error.message);
                
                if let Some(loc) = error.location {
                    msg.push_str(&format!("\nLocation: {}:{}:{}\n", loc.file, loc.line, loc.column));
                    msg.push_str(&format!("\n{}\n", loc.snippet));
                }
                
                if !error.suggestions.is_empty() {
                    msg.push_str("\nSuggestions:\n");
                    for suggestion in &error.suggestions {
                        msg.push_str(&format!("  - {}\n", suggestion));
                    }
                }
                
                Err(msg)
            }
        }
    }};
}

/// Validate all shaders in a directory at build time
pub fn validate_shader_directory(dir: &str) -> Result<(), String> {
    use std::fs;
    use std::path::Path;
    
    let path = Path::new(dir);
    if !path.exists() {
        return Ok(()); // Skip if directory doesn't exist
    }
    
    let mut errors = Vec::new();
    
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("wgsl") {
            let name = path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            
            if let Err(e) = validate_shader!(name, &source) {
                errors.push(e);
            }
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_shader() {
        let valid_shader = r#"
            @group(0) @binding(0) var<uniform> camera: mat4x4<f32>;
            
            @vertex
            fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
                return camera * vec4<f32>(position, 1.0);
            }
        "#;
        
        let mut validator = ShaderValidator::new();
        let result = validator.validate_wgsl("test.wgsl", valid_shader);
        
        match result {
            ValidationResult::Ok => (),
            ValidationResult::Error(e) => panic!("Valid shader failed: {:?}", e),
        }
    }
    
    #[test]
    fn test_invalid_shader_with_suggestions() {
        let invalid_shader = r#"
            @binding(0) var<uniform> camera: mat4x4<f32>;
            
            fn main() {
                let x: unknowntype = 1;
            }
        "#;
        
        let mut validator = ShaderValidator::new();
        let result = validator.validate_wgsl("test.wgsl", invalid_shader);
        
        match result {
            ValidationResult::Ok => panic!("Invalid shader passed validation"),
            ValidationResult::Error(e) => {
                assert!(!e.suggestions.is_empty());
                // Should suggest adding @group
                assert!(e.suggestions.iter().any(|s| s.contains("@group")));
            }
        }
    }
}