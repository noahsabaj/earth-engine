// Hearth Engine Custom Clippy Lints
// Sprint 37: DOP Reality Check

#![feature(rustc_private)]
#![warn(rust_2018_idioms, unused_lifetimes)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use rustc_lint::LintStore;
use rustc_session::Session;

mod dop_enforcement;

#[no_mangle]
pub fn __rustc_plugin_registrar(reg: &mut rustc_lint::LintStore) {
    reg.register_lints(&[
        &dop_enforcement::METHODS_ON_DATA_STRUCTS,
        &dop_enforcement::ARRAY_OF_STRUCTS_PATTERN,
        &dop_enforcement::TRAIT_OBJECTS_FORBIDDEN,
        &dop_enforcement::BUILDER_PATTERNS_FORBIDDEN,
        &dop_enforcement::VEC_PUSH_IN_LOOPS,
    ]);

    reg.register_late_pass(|| Box::new(dop_enforcement::DopEnforcement));
}

// Alternative entry point for newer clippy versions
pub fn register_plugins(store: &mut LintStore, _sess: &Session) {
    store.register_lints(&[
        &dop_enforcement::METHODS_ON_DATA_STRUCTS,
        &dop_enforcement::ARRAY_OF_STRUCTS_PATTERN, 
        &dop_enforcement::TRAIT_OBJECTS_FORBIDDEN,
        &dop_enforcement::BUILDER_PATTERNS_FORBIDDEN,
        &dop_enforcement::VEC_PUSH_IN_LOOPS,
    ]);

    store.register_late_pass(|| Box::new(dop_enforcement::DopEnforcement));
}