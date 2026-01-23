#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::path::Path;

#[test]
fn registry_fallback_to_generic() {
    let registry = AdapterRegistry::default();
    let adapter = registry.adapter_for(Path::new("unknown.xyz"));
    assert_eq!(adapter.name(), "generic");
}

#[test]
fn registry_extension_lookup_falls_back() {
    // With no language adapters registered, all files fall back to generic
    let registry = AdapterRegistry::default();
    assert_eq!(registry.adapter_for(Path::new("foo.rs")).name(), "generic");
    assert_eq!(registry.adapter_for(Path::new("bar.py")).name(), "generic");
}
