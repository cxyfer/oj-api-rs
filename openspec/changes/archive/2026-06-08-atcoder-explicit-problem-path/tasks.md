## 1. AtCoder Identifier Parsing

- [x] 1.1 Update Rust AtCoder dynamic fetch parsing to accept `contest/problem_id` and `contest/tasks/problem_id` explicit forms.
- [x] 1.2 Update Python `AtCoderClient` parsing and URL derivation to match the Rust explicit-form behavior.
- [x] 1.3 Remove automatic `arc*_abcNNN_* -> abcNNN` contest inference from Rust and Python derivation.
- [x] 1.4 Preserve prefix-based derivation for straightforward IDs and `pastYYYYMM_* -> pastYYYYMM-open` handling.

## 2. Validation and Tests

- [x] 2.1 Add Rust unit tests for `abc042_a`, `arc001_1`, `past201912_a`, `ndpc/ndpc2026_m`, `ndpc/tasks/ndpc2026_m`, `arc058_abc042_a`, and `abc042/arc058_abc042_a`.
- [x] 2.2 Add Python derivation tests for the same accepted AtCoder formats.
- [x] 2.3 Add validation coverage to ensure explicit AtCoder paths are accepted while malformed or traversal-like values are rejected.

## 3. Verification

- [x] 3.1 Run Rust formatting, targeted dynamic-problem tests, and full `cargo test`.
- [x] 3.2 Run Python derivation tests through the scripts project environment.
- [x] 3.3 Run changed-file lint/format checks for Rust and Python.
- [x] 3.4 Confirm OpenSpec status reports the change as apply-ready.
