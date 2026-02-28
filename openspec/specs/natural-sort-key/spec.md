# Spec: Natural Sort Key Function

## Overview

A pure Rust function `natural_sort_key(s: &str) -> String` splits an ID string into alternating non-digit/digit segments, zero-pads numeric segments to 20 chars, and lowercases non-digit segments. The result is a string that compares lexicographically equivalent to the natural ordering of the original ID.

The function is registered as a SQLite scalar function on every RO pool connection, making `natural_sort_key(id)` callable in SQL.

## Requirements

### REQ-1.1: Algorithm correctness

**Scenario**: Numeric segment comparison

Given IDs `"P2000"` and `"P10000"`:
- `natural_sort_key("P2000")` = `"p00000000000000002000"`
- `natural_sort_key("P10000")` = `"p00000000000000010000"`
- `natural_sort_key("P2000") < natural_sort_key("P10000")` ✓

**Scenario**: Multi-segment with special characters

Given `"abc001_a"`:
- Segments: `"abc"`, `"001"`, `"_a"`
- Key: `"abc00000000000000000001_a"`

**Scenario**: Pure numeric ID

Given `"1000"` and `"999"`:
- `natural_sort_key("999")` = `"00000000000000000999"`
- `natural_sort_key("1000")` = `"00000000000000001000"`
- Result: `"999"` < `"1000"` ✓

### REQ-1.2: Determinism constraints

- `natural_sort_key` is deterministic: same input always yields same output
- NULL input returns `""` (empty string)
- Empty string input returns `""` (empty string)
- Case-insensitive for alpha segments: `natural_sort_key("P1") == natural_sort_key("p1")`

### REQ-1.3: Registration

- Registered via `conn.create_scalar_function("natural_sort_key", 1, FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_INNOCUOUS, ...)`
- Called in `create_ro_pool`'s `with_init` closure
- `rusqlite` feature `"functions"` added to `Cargo.toml`

## PBT Properties

| Property | Invariant | Falsification |
|----------|-----------|---------------|
| **Numeric ordering** | `a.parse::<u128>() < b.parse::<u128>()` → `natural_sort_key(prefix+a) < natural_sort_key(prefix+b)` for any common prefix | Generate random numeric-only IDs, compare natural key vs numeric value |
| **Idempotency** | `natural_sort_key(natural_sort_key(s)) != natural_sort_key(s)` unless input is already a key | Apply function twice; if result differs from single application, function is not pure-key |
| **Case insensitivity** | `natural_sort_key(s.to_uppercase()) == natural_sort_key(s.to_ascii_lowercase())` | Generate random ASCII strings, compare upper vs lower |
| **Transitivity** | `key(a) < key(b) && key(b) < key(c)` → `key(a) < key(c)` | Generate triples, check transitivity of key ordering |
| **Segment count** | All-digit input has exactly one 20-char segment; all-alpha input has no padding | Generate 1..=20 digit strings; verify key length == 20 |
