## Why

Codeforces contest fetching currently calls `contest.standings` with `from=1&count=1`, but that API request now errors for affected contests. The crawler needs to use the accepted standings request shape so `--fetch-contest` and single-contest fetches can discover contest problems reliably.

## What Changes

- Update the Codeforces contest standings fetch to request standings with only `contestId`.
- Preserve existing parsing of the returned contest problem list and contest name.
- Keep existing Codeforces crawler CLI workflows and progress behavior unchanged.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `crawler-cli`: Codeforces contest fetching must call `contest.standings` without pagination parameters.

## Impact

- Affected code: `scripts/codeforces.py`.
- Affected workflows: Codeforces `--fetch-contest`, `--contest <id>`, and legacy contest fetch aliases that use standings discovery.
- No API server route, database schema, dependency, or configuration changes.
