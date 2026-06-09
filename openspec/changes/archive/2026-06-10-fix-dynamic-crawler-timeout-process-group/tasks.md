## 1. Dynamic Crawler Process Management

- [x] 1.1 Refactor `run_single_problem_crawler` to spawn dynamic crawler commands with `crate::utils::spawn_with_pgid`.
- [x] 1.2 Capture the spawned child PID and wait for completion through an explicit wait task while preserving stderr logging behavior.
- [x] 1.3 On timeout, call `crate::utils::kill_pgid(pid)` before completing the dynamic fetch failure path.
- [x] 1.4 Preserve existing successful completion, non-zero exit, spawn error, and database re-read behavior.

## 2. Regression Coverage

- [x] 2.1 Add focused test coverage for the dynamic crawler timeout branch without external judge network calls.
- [x] 2.2 Verify the timeout path returns not found while terminating the spawned process group.
- [x] 2.3 Keep existing dynamic fetch success/failure and AtCoder explicit path tests passing.

## 3. Validation

- [x] 3.1 Run targeted Rust tests for `dynamic_problem` and problem detail dynamic fetch behavior.
- [x] 3.2 Run formatting or lint checks if the implementation changes Rust formatting-sensitive code.
- [x] 3.3 Confirm OpenSpec status reports the change as ready for apply.
