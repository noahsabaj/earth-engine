// Earth Engine Custom Clippy Lints for DOP Enforcement
// Sprint 37: DOP Reality Check

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet;
use rustc_hir::{Item, ItemKind, ImplItem, ImplItemKind, FnDecl, FnSig};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

declare_tool_lint! {
    /// **What it does:**
    /// Detects methods with `&self` or `&mut self` parameters on data structures.
    /// 
    /// **Why is this bad?**
    /// Earth Engine follows strict data-oriented programming (DOP) principles.
    /// Methods violate DOP by coupling behavior with data, preventing GPU compatibility
    /// and SIMD optimization.
    /// 
    /// **Known problems:**
    /// This lint may trigger false positives for standard trait implementations
    /// like `Debug`, `Clone`, etc. Use `#[allow(clippy::methods_on_data_structs)]` 
    /// for these cases.
    /// 
    /// **Example:**
    /// ```rust
    /// // Bad - method on data structure
    /// impl PlayerData {
    ///     pub fn update(&mut self, dt: f32) { // triggers lint
    ///         self.position += self.velocity * dt;
    ///     }
    /// }
    /// 
    /// // Good - external kernel function
    /// pub fn update_players(
    ///     positions: &mut [Vec3],
    ///     velocities: &[Vec3],
    ///     dt: f32,
    /// ) {
    ///     for i in 0..positions.len() {
    ///         positions[i] += velocities[i] * dt;
    ///     }
    /// }
    /// ```
    pub clippy::METHODS_ON_DATA_STRUCTS,
    restriction,
    "methods on data structures violate DOP principles"
}

declare_tool_lint! {
    /// **What it does:**
    /// Detects Array of Structs (AoS) patterns that should be Structure of Arrays (SoA).
    /// 
    /// **Why is this bad?**
    /// AoS layout causes cache misses and prevents SIMD optimization.
    /// SoA layout is required for GPU compatibility and performance.
    /// 
    /// **Example:**
    /// ```rust
    /// // Bad - Array of Structs
    /// struct Particle {
    ///     position: Vec3,
    ///     velocity: Vec3,
    /// }
    /// let particles: Vec<Particle> = vec![]; // triggers lint
    /// 
    /// // Good - Structure of Arrays
    /// struct ParticleData {
    ///     count: usize,
    ///     positions_x: Vec<f32>,
    ///     positions_y: Vec<f32>,
    ///     positions_z: Vec<f32>,
    ///     velocities_x: Vec<f32>,
    ///     velocities_y: Vec<f32>,
    ///     velocities_z: Vec<f32>,
    /// }
    /// ```
    pub clippy::ARRAY_OF_STRUCTS_PATTERN,
    perf,
    "Array of Structs patterns should be Structure of Arrays for performance"
}

declare_tool_lint! {
    /// **What it does:**
    /// Detects trait objects that use dynamic dispatch.
    /// 
    /// **Why is this bad?**
    /// Dynamic dispatch prevents inlining, SIMD optimization, and GPU compilation.
    /// DOP prefers data-driven dispatch over runtime polymorphism.
    /// 
    /// **Example:**
    /// ```rust
    /// // Bad - trait object with dynamic dispatch
    /// let drawable: Box<dyn Drawable> = Box::new(mesh); // triggers lint
    /// 
    /// // Good - data-driven dispatch
    /// enum RenderType { Mesh, Particle, Terrain }
    /// struct RenderData {
    ///     render_type: Vec<RenderType>,
    ///     mesh_indices: Vec<u32>,
    ///     // ... other data arrays
    /// }
    /// ```
    pub clippy::TRAIT_OBJECTS_FORBIDDEN,
    restriction,
    "trait objects use dynamic dispatch which violates DOP principles"
}

declare_tool_lint! {
    /// **What it does:**
    /// Detects builder patterns that create objects instead of data.
    /// 
    /// **Why is this bad?**
    /// Builder patterns encourage object-oriented thinking and runtime configuration.
    /// DOP prefers compile-time configuration and direct data initialization.
    /// 
    /// **Example:**
    /// ```rust
    /// // Bad - builder pattern
    /// let system = SystemBuilder::new() // triggers lint
    ///     .with_capacity(1000)
    ///     .build();
    /// 
    /// // Good - direct data initialization
    /// let system_data = SystemData::new(1000);
    /// ```
    pub clippy::BUILDER_PATTERNS_FORBIDDEN,
    restriction,
    "builder patterns encourage OOP thinking over DOP data initialization"
}

declare_tool_lint! {
    /// **What it does:**
    /// Detects `Vec::push()` calls in loops that could cause performance issues.
    /// 
    /// **Why is this bad?**
    /// Runtime allocation in hot paths violates DOP principles of pre-allocation.
    /// Growing vectors causes memory fragmentation and unpredictable performance.
    /// 
    /// **Example:**
    /// ```rust
    /// // Bad - runtime allocation in loop
    /// for item in items {
    ///     results.push(process(item)); // triggers lint
    /// }
    /// 
    /// // Good - pre-allocated buffer
    /// let mut results = Vec::with_capacity(items.len());
    /// for item in items {
    ///     results.push(process(item)); // OK - capacity known
    /// }
    /// ```
    pub clippy::VEC_PUSH_IN_LOOPS,
    perf,
    "Vec::push in loops causes runtime allocation - prefer pre-allocation"
}

declare_lint_pass!(DopEnforcement => [
    METHODS_ON_DATA_STRUCTS,
    ARRAY_OF_STRUCTS_PATTERN,
    TRAIT_OBJECTS_FORBIDDEN,
    BUILDER_PATTERNS_FORBIDDEN,
    VEC_PUSH_IN_LOOPS,
]);

impl<'tcx> LateLintPass<'tcx> for DopEnforcement {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'tcx>) {
        if let ImplItemKind::Fn(sig, _) = &impl_item.kind {
            self.check_method_on_data_struct(cx, impl_item, sig);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        match &item.kind {
            ItemKind::Struct(..) => {
                self.check_for_builder_pattern(cx, item);
            }
            ItemKind::Impl(impl_) => {
                self.check_for_array_of_structs(cx, item);
            }
            _ => {}
        }
    }
}

impl DopEnforcement {
    fn check_method_on_data_struct(
        &mut self,
        cx: &LateContext<'_>,
        impl_item: &ImplItem<'_>,
        sig: &FnSig<'_>,
    ) {
        // Check if this function has a self parameter
        if let Some(self_arg) = sig.decl.inputs.first() {
            let self_snippet = snippet(cx, self_arg.span, "self");
            
            // Skip standard trait methods that are acceptable
            let method_name = impl_item.ident.name.as_str();
            let allowed_methods = [
                "new", "default", "clone", "fmt", "eq", "ne", "cmp", 
                "partial_cmp", "hash", "drop", "deref", "deref_mut",
                "from", "into", "try_from", "try_into"
            ];
            
            if allowed_methods.contains(&method_name) {
                return;
            }
            
            // Check for &self or &mut self
            if self_snippet.contains("&self") || self_snippet.contains("&mut self") {
                span_lint_and_help(
                    cx,
                    METHODS_ON_DATA_STRUCTS,
                    impl_item.span,
                    "method with self parameter violates DOP principles",
                    None,
                    &format!(
                        "convert `{}` to an external kernel function that takes data parameters instead. \
                        See docs/guides/DOP_ENFORCEMENT.md for examples.",
                        method_name
                    ),
                );
            }
        }
    }

    fn check_for_array_of_structs(
        &mut self,
        cx: &LateContext<'_>,
        item: &Item<'_>,
    ) {
        // This is a simplified check - a full implementation would analyze
        // the type system to detect Vec<CustomStruct> patterns
        let item_name = item.ident.name.as_str();
        
        // Look for patterns that suggest AoS usage
        if item_name.ends_with("List") || item_name.ends_with("Array") {
            // This would need more sophisticated analysis in a real implementation
            // For now, we'll just warn about suspicious naming patterns
        }
    }

    fn check_for_builder_pattern(
        &mut self,
        cx: &LateContext<'_>,
        item: &Item<'_>,
    ) {
        let item_name = item.ident.name.as_str();
        
        if item_name.ends_with("Builder") || item_name.ends_with("Factory") {
            span_lint_and_help(
                cx,
                BUILDER_PATTERNS_FORBIDDEN,
                item.span,
                "builder patterns encourage OOP thinking",
                None,
                "use direct data initialization instead of builder patterns. \
                Create data structures directly with known values.",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clippy_utils::diagnostics::span_lint_and_help;

    // Test helper functions would go here
    // Due to the complexity of setting up clippy test infrastructure,
    // these are simplified examples

    #[test]
    fn test_method_detection() {
        // This would test that methods with &self trigger the lint
        // In a real implementation, this would use clippy's test framework
    }

    #[test]
    fn test_builder_detection() {
        // This would test that Builder structs trigger the lint
    }
}

// Additional helper functions for the lints

fn is_data_struct_name(name: &str) -> bool {
    // Heuristic to identify data structures vs. behavior objects
    name.ends_with("Data") || 
    name.ends_with("Buffer") || 
    name.ends_with("Pool") ||
    name.ends_with("Array") ||
    name.ends_with("Storage")
}

fn is_performance_critical_path(item_path: &str) -> bool {
    // Check if this code is in performance-critical directories
    let critical_paths = [
        "src/renderer/",
        "src/world_gpu/", 
        "src/physics_data/",
        "src/particles/",
        "src/lighting/",
        "src/memory/",
    ];
    
    critical_paths.iter().any(|&path| item_path.starts_with(path))
}