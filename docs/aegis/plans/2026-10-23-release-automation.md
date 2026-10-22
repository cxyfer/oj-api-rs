# English release notes and RC image automation

## Goal

Implement the user-approved urlet-style English release format for stable and
RC tags. Build and push the corresponding GHCR image before creating a release
draft. Preserve manually edited notes on workflow reruns.

## Architecture and tech stack

Reuse `.github/workflows/ghcr.yml` as the versioned-image owner. Add one Python
standard-library release helper for version selection, notes generation and
notes checks; use Git and the authenticated GitHub CLI for source metadata.
The workflow owns container publication and draft creation. No AI credentials,
additional release service, runtime changes or new package dependencies.

## Baseline and authority refs

- User request: English only; release candidates must receive notes and images.
- Previous proposal: automated drafts with editorial summary and highlights.
- `.github/workflows/ghcr.yml`: currently pushes the exact version and `latest`.
- `.github/workflows/ghcr-dev.yml`, `ghcr-nightly.yml`: separate rolling channels.
- `Cargo.toml`, `Cargo.lock`: version identity; current version is `0.4.0`.
- Existing commit subjects use gitmoji and PR-number suffixes.
- `CHANGELOG.md`: preserve historical content; this task does not regenerate it.
- `docs/aegis/BASELINE-GOVERNANCE.md`: workflow ownership and verification.

## Compatibility boundary

Keep existing version image names and the dev/nightly channel behavior. Only
the newest stable tag may update `latest`; RCs publish their exact image only,
as explicitly confirmed by the user. Do not
create a release tag, bump a version, publish a production release, or merge the
PR without a concrete requested release version.

## Verification

Run standard-library unit/integration tests using temporary Git repositories
and mocked GitHub metadata (`python3 -S scripts/test_release_notes.py -v`).
Cover stable and RC comparison ranges, moving tags,
version ordering, direct commits, PR deduplication, historical contributors,
metadata errors, overwrite protection and notes validation. Run actionlint and
Python lint on the changed files. Use a PR-triggered GitHub Actions build to
prove the Docker image builds; local Docker is unavailable in WSL.

## Change necessity and existence check

The current workflow cannot create the requested notes. GitHub's default
generator cannot supply the editorial format and does not own this repository's
PR suffix convention. One release helper is the minimum reusable owner; reuse
the existing image workflow instead of adding a competing publisher.

## Tasks

1. Add `scripts/release_notes.py` and `scripts/test_release_notes.py`.
   - CLI: `metadata TAG --github-output FILE`, `generate TAG --repo OWNER/REPO
     --output FILE [--previous-tag TAG]`, and `check FILE`.
   - Accept exact `vMAJOR.MINOR.PATCH` and `vMAJOR.MINOR.PATCH-rc.N` tags.
   - Metadata verifies the tag, checkout, Cargo manifest and lockfile; emits
     tag, version, prerelease, previous_tag, is_latest and is_latest_stable.
   - Stable notes compare with the previous reachable stable version; RC notes
     compare with the previous reachable version. Exclude nightly and unrelated
     or newer tags. Handle the first release explicitly.
   - Generate the English important callout, editorial highlights placeholders,
     complete PR/direct-commit list, verified first-time PR contributors and
     full comparison link. Never silently ignore API errors or overwrite notes.
   - Check required sections, unresolved placeholders and hard-wrapped prose.
2. Extend `.github/workflows/ghcr.yml`.
   - Test release tooling and build PR images without pushing.
   - Add manual validation/rebuild support. Only an explicit version tag may
     publish an image; never publish a branch build to `latest`.
   - Validate version identity, build/push the image, then create an English
     draft and mark RC drafts as prereleases. Retain existing release notes.
   - Keep write permissions scoped to the jobs that require them.
3. Document `.github/release-notes-template.md`, `RELEASING.md`, and a README
   entry with exact commands, RC behavior, publication order and rerun behavior.
4. Independently review requirements and code, run focused checks, commit using
   repository history style, push `feat/release-automation`, and open a PR
   against `dev`. Observe the hosted image build before reporting completion.

## Execution readiness and risks

Scope is release tooling, documentation and existing image workflow only.
No application, database, crawler, persistent volume or homepage changes.
TDD route: off; behavioral tests are required because incorrect ranges or
reruns could publish misleading notes. Architecture review is required for the
release owner and permission boundary. No legacy release owner is removed.
RC image policy is confirmed: exact version only, with `latest` reserved for
the newest stable release. Actual RC publication requires the user to identify
a version tag.
