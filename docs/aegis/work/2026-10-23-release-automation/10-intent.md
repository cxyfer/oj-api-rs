# Release automation intent

Outcome: English urlet-style release drafts and images for stable and RC tags.
Authority: user discussion and `../../plans/2026-10-23-release-automation.md`.
Baseline read set: existing GHCR workflows, Cargo version files, CHANGELOG,
README, commit history, and BASELINE-GOVERNANCE. All read; no missing baseline.
Impact: release CLI, versioned-image workflow, release documentation and tests.
Non-goals: runtime changes, channel consolidation, version bump, tagging,
production publication, automatic merge, rewriting historical changelogs.
Owner invariant: existing GHCR workflow builds images; one helper owns version
selection and generated note structure; a maintainer owns editorial prose.
Evidence needed: focused tests, workflow lint, independent review, and a hosted
Docker build. Current Docker CLI is a WSL shim without a working daemon.
Confirmed image policy: RCs publish only their exact version; `latest` is
reserved for the newest stable release. An actual RC needs a user-specified tag.
