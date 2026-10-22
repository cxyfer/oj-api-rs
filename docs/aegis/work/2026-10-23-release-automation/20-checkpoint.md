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
- Specification review passes; code-quality review is running. Local Ruff,
  actionlint with ShellCheck, and diff checks pass. The only actionlint schema
  exception is documented GitHub `concurrency.queue: max` support.
- Tasks: finish code review, commit/push/PR, and observe the hosted image build.
- Drift: none; application and persistent state are outside scope.
- Next: finish independent review and open the PR to run the hosted image build.
