# Release automation checkpoint

- Plan recorded; implementation starting.
- Worktree: `.worktrees/release-automation`, branch `feat/release-automation`.
- Base: updated `origin/dev`, commit `4536487`.
- Fetch of all tags rejected the moving `nightly` tag; a scoped fetch of base
  branches and `v*` tags succeeded without replacing local `nightly`.
- Release helper and workflow/docs implemented; 18 focused tests pass without
  site-packages. Live `v0.4.0` notes generation succeeded.
- User confirmed RCs must never update `latest`; workflow consumes the helper's
  `is_latest_stable` output. Only exact RC images are pushed.
- Specification and code-quality reviews pass. Local Ruff,
  actionlint with ShellCheck, and diff checks pass. The only actionlint schema
  exception is documented GitHub `concurrency.queue: max` support.
- Implementation committed and pushed as `513f67e`; PR #45 targets `dev`.
- Hosted run `39842825494` succeeds: prepare, actual image build, container
  smoke test, and image export/upload all pass. The image artifact is
  `oj-api-rs-validation-image` (ID `12454271640`, 91,109,421 bytes).
- Drift: none; application and persistent state are outside scope.
- Ready for maintainer review in PR #45. The PR remains open; no release tag,
  version bump, production image push, release publication, or merge occurred.
