
## Rust `cfg_test_split` Modes

The `[rust].cfg_test_split` option controls how `#[cfg(test)]` blocks are handled for LOC counting:

```toml
[rust]
cfg_test_split = "count"  # default
```

| Mode | Behavior |
|------|----------|
| `"count"` | Split `#[cfg(test)]` blocks into test LOC (current behavior) |
| `"require"` | Fail if source files contain inline `#[cfg(test)]` blocks; require separate `_tests.rs` files |
| `"off"` | Count all lines as source LOC, don't parse for `#[cfg(test)]` |

### `require` Mode

Projects using `require` mode enforce the sibling test file convention:

```
src/parser.rs       # source only, no #[cfg(test)]
src/parser_tests.rs # all tests here
```

Violations would report:
```
src/parser.rs:150: inline_cfg_test
  Move tests to a sibling _tests.rs file.
```

This pairs with the existing convention documented in CLAUDE.md for using `#[path]` attributes.
