# Releasing oj-api-rs

Versioned releases use English release notes in the
[standard format](.github/release-notes-template.md). Both stable tags and RC
tags build and push a GHCR image. The image must exist before the workflow
creates the GitHub release draft.

## Tags and images

Supported tags are `vMAJOR.MINOR.PATCH` and `vMAJOR.MINOR.PATCH-rc.N`. The version
after `v` must match both `Cargo.toml` and the root package in `Cargo.lock`.

| Tag or branch | Image | GitHub release |
| --- | --- | --- |
| `v0.5.0` | `ghcr.io/cxyfer/oj-api-rs:v0.5.0` | Stable draft |
| `v0.5.0-rc.1` | `ghcr.io/cxyfer/oj-api-rs:v0.5.0-rc.1` | Prerelease draft |
| `dev` | `ghcr.io/cxyfer/oj-api-rs:dev` | No versioned release |
| `master` without a release tag | `ghcr.io/cxyfer/oj-api-rs:nightly` | Rolling nightly prerelease |

Only the newest stable tag also updates the Docker `latest` image. RC builds
publish their exact version image and never move `latest`. Version ordering is
numeric: `rc.10` follows `rc.2`, and a stable version follows its own RCs.
Rebuilding an older stable version updates only its exact image tag. RCs remain
GitHub prereleases; the Docker `latest` image and GitHub's latest release label
both remain reserved for stable releases.

Release builds share a serialized queue, preventing an older in-flight build
from finishing after a newer build and moving `latest` backwards. PR validation
builds use separate concurrency groups.

Image names retain the `v` prefix. There are no additional floating major,
minor or `rc` image aliases. Dev and nightly are managed by their existing
workflows.

## Prepare a version

1. Choose a stable or RC version and update `Cargo.toml` in a release PR.
2. Refresh the root package entry in `Cargo.lock` and check it in:

   ```bash
   cargo update --package oj-api-rs
   cargo fmt --check
   cargo test --locked
   uv --directory scripts run --frozen pytest
   ```

3. Review incompatible API or MCP changes, configuration defaults and data
   changes for the upgrade guidance. The notes generator cannot infer these.
4. Merge the release PR into the intended release branch before tagging. The
   tagged commit must contain the release workflow and helper. Never move or
   reuse a published tag; fix a release with a new version.

## Build the image and create a draft

For example, after preparing version `0.5.0-rc.1`:

```bash
git tag --annotate v0.5.0-rc.1 --message v0.5.0-rc.1
git push origin v0.5.0-rc.1
gh run list --workflow ghcr.yml --limit 5
gh run watch RUN_ID --exit-status
```

The workflow tests the release helper, validates the version, builds and pushes
the exact image, and then creates an English release draft. RC drafts are
marked as prereleases. If a draft or published release already exists for the
tag, the workflow preserves its notes and publication status. API errors fail
the run instead of being treated as a missing release.

Verify the published image before publishing the notes:

```bash
docker pull ghcr.io/cxyfer/oj-api-rs:v0.5.0-rc.1
docker image inspect ghcr.io/cxyfer/oj-api-rs:v0.5.0-rc.1
```

For a retry or explicit rebuild of a tag that includes this tooling:

```bash
gh workflow run ghcr.yml --ref dev -f tag=v0.5.0-rc.1
```

The workflow must first be available on the repository's default branch for
manual dispatch. Select a ref containing the current workflow. With no `tag`
input, a manual run only builds and smoke-tests an image without pushing or
creating a release. Relevant pull requests run the same validation build, so
workflow changes can be tested before publishing a version.

The smoke test starts a fresh container, verifies the homepage, and checks that
SQLite and sqlite-vec are available. An empty vector table deliberately returns
HTTP 503 with `vec_dimension: null`; validation requires exactly that expected
empty-database state rather than treating every 503 response as success.

Successful validation runs retain an `oj-api-rs-validation-image` artifact for
three days. Download it, extract the archive, and load the image locally:

```bash
docker load --input oj-api-rs-validation-image.tar.gz
```

## Edit and publish the release notes

Download the draft, fill in every editorial placeholder, and run the check:

```bash
tag=v0.5.0-rc.1
notes=/tmp/oj-api-rs-release-notes.md
gh release view "$tag" --json body --jq .body > "$notes"
# Edit the important callout and Highlights in $notes.
python3 scripts/release_notes.py check "$notes"
gh release edit "$tag" --notes-file "$notes" --draft=false --prerelease --latest=false
```

For a new stable release, publish with `--prerelease=false` and use `--latest`
only when it should become GitHub's latest stable release. The generator does
not publish anything itself, and creating or publishing notes does not trigger
another container build: the image was already pushed by the tag workflow.

To generate a separate local draft, use an output path that does not exist:

```bash
python3 scripts/release_notes.py generate v0.5.0-rc.1 \
  --repo cxyfer/oj-api-rs --output /tmp/oj-api-rs-rc-draft.md
```

Add `--previous-tag v0.4.0` only when deliberately overriding the automatic
range. The previous tag must be an older supported version and an ancestor of
the target. Existing output files are never overwritten.

## Comparison ranges

- A stable release compares against the previous reachable stable version,
  including all changes from its RC cycle.
- An RC compares against the previous reachable RC or stable version, so each
  candidate describes its incremental changes.
- `nightly`, unrelated tags and versions newer than the target are excluded.
- A first release includes its available history and links to its commit list.
- PRs are listed once; direct commits are included with their short SHA.
- First-time PR contributors are checked against the preceding represented Git
  history rather than estimated from the previous tag's calendar date.

Historical `CHANGELOG.md` entries are retained. This automation does not rewrite
them or commit generated release notes back to a protected branch.
