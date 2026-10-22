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
- Independent code-quality review passes for `origin/dev...513f67e`.
- PR #45 targets `dev`. Hosted run `39842825494` for `513f67e` succeeds and
  accepts `queue: max`. Prepare passes the same 18 helper tests. Job
  `120048226989` builds the actual Docker image, starts its container, verifies
  the expected empty-database health JSON plus homepage, and exports the image.
  The build job takes 5m04s; image compilation/build takes 4m01s.
- Image artifact `oj-api-rs-validation-image`, ID `12454271640`, is available
  with size 91,109,421 bytes and three-day retention. It contains a gzip Docker
  archive for `docker load`. Hosted run and artifact metadata were read back
  after completion; every validation step reports success.
- No production tag, registry image or GitHub release has been created by
  validation.
- Boundary: actual tag-triggered GHCR push and GitHub draft creation are
  implemented and reviewed but were deliberately not executed without a chosen
  release version. PR validation skips both publishing jobs as intended.
