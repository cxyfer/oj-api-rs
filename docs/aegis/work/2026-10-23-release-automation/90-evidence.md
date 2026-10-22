# Release automation evidence

- GitHub repository metadata confirms `master` is default, while recent feature
  PRs #39–44 target `dev`; this task therefore targets `dev`.
- Initial checkout is clean at `4536487`; worktree created from updated
  `origin/dev`.
- Remote latest stable release is `v0.4.0`.
- `docker version` reports unavailable WSL integration; use a hosted build.
- `python3 -S scripts/test_release_notes.py -v`: 18 tests pass on Python 3.10;
  temporary repositories and a local GitHub CLI stub cover numeric stable/RC
  ordering, version identity, baselines, contributors, failures and preservation.
- Ruff check and format check pass for both Python files; `git diff --check`
  passes.
- Actionlint 1.7.12 with ShellCheck 0.11.0 passes with one narrow schema
  exception for `concurrency.queue`. GitHub's current official
  `data/reusables/actions/actions-group-concurrency.md` documents `queue: max`;
  successful urlet runs 39833276136 and 39713802247 use it. Hosted acceptance
  remains to be observed on this PR.
- Live authenticated notes generation for historical `v0.4.0` succeeds using
  GitHub PR metadata. The result compares `v0.3.3...v0.4.0`, contains each of
  PRs #19–31 once, and retains direct commits.
- Independent specification review passes after correcting the smoke test to
  require the runtime's expected empty-database HTTP 503 JSON and homepage 200.
- Code-quality review and hosted image build are pending. No production tag,
  registry image or GitHub release has been created by validation.
