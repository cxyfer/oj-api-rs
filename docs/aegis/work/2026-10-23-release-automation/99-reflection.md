# Release automation reflection

The existing versioned GHCR workflow remains the only publisher for stable and
RC tags. A standard-library helper owns version identity, comparison ranges and
note structure; maintainers own the editorial summary and publication decision.
The user explicitly reserved `latest` for stable images, so prereleases carry
only their exact version image.

Independent review caught a meaningful integration detail: an empty database
reports HTTP 503 even when SQLite and sqlite-vec initialize correctly. The
smoke test now verifies that exact response and the homepage rather than
changing runtime behavior or accepting arbitrary failures.

Focused tests run the script directly with `python3 -S`, avoiding unrelated
crawler package imports and proving the helper needs no installed Python
dependencies. Live historical generation complements mocked API tests. The
hosted container build is required because the local WSL Docker daemon is not
available.

No release version was chosen. Publishing a real tag and validating registry
push plus draft creation remain part of the documented release procedure.
