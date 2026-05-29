## Context

The crawler surface is split across four Python entry points, a Rust argument whitelist, and a static admin UI flag map. Several flags express the same operator intent with platform-specific names:

- LeetCode metadata sync uses `--init`.
- Luogu metadata sync uses `--sync`; SPOJ uses `--sync-spoj` through `luogu.py`.
- AtCoder metadata sync uses `--sync-kenkoooo` / `--sync-history`.
- AtCoder and Codeforces contest crawling uses `--fetch-all`, with `--resume` required by the CLI to skip progress already saved in JSON files.
- Content backfill already mostly aligns on `--fill-missing-content`.

The design needs to improve the CLI contract without disrupting existing jobs, daily challenge flows, or platform-specific auxiliary operations.

## Goals / Non-Goals

**Goals:**

- Define three canonical operator-facing crawler operations:
  - `--sync-problemset`
  - `--fetch-contest`
  - `--fill-missing-content`
- Keep existing legacy flags working as aliases where practical.
- Make AtCoder and Codeforces contest crawling resume by default, with `--no-resume` as the explicit override.
- Update Python argparse definitions, Rust validation, admin UI configuration, i18n labels, and README documentation consistently.
- Keep source-specific auxiliary flags available for workflows outside the three canonical operations.

**Non-Goals:**

- Do not rewrite crawler internals, scraping/parsing logic, or database schema.
- Do not add a new dependency or external CLI framework.
- Do not introduce a new HTTP API for dynamically serving crawler flag metadata.
- Do not remove legacy flags in this change.
- Do not change LeetCode daily challenge flags or fallback crawler behavior except where whitelist/UI labels need to coexist with the unified operations.

## Decisions

### Decision 1: Add canonical flags in each existing script instead of creating a new wrapper

Each crawler script will accept the canonical operation flags it supports and map them to existing methods:

| Source | `--sync-problemset` | `--fetch-contest` | `--fill-missing-content` |
| --- | --- | --- | --- |
| `leetcode.py` | `init_all_problems()` | unsupported | existing content backfill |
| `atcoder.py` | `fetch_from_kenkoooo()` | `fetch_all_problems(resume=...)` | existing content backfill |
| `codeforces.py` | existing `sync_problemset()` | `fetch_all_problems(resume=...)` | existing content backfill |
| `luogu.py` with Luogu source | `sync(overwrite=...)` | unsupported | existing `sync_content(source="luogu")` |
| `luogu.py` with SPOJ source | `sync_spoj(overwrite=...)` | unsupported | existing `sync_content(source="spoj")` |

Alternative considered: create a new `crawler.py` wrapper that dispatches to platform scripts. Rejected because Rust already routes by source to existing scripts, and a wrapper would add another dispatch layer without reducing the need for per-source validation.

### Decision 2: Preserve legacy flags as aliases, but move UI toward canonical operations

Legacy flags remain accepted by Python scripts and Rust validation. The admin UI should prefer canonical operations for routine crawler work:

- LeetCode: expose `--sync-problemset`, `--fill-missing-content`, plus daily challenge auxiliary flags.
- AtCoder: expose `--sync-problemset`, `--fetch-contest`, `--no-resume`, `--contest`, `--fill-missing-content`, and maintained auxiliary/debug flags.
- Codeforces: expose `--sync-problemset`, `--fetch-contest`, `--no-resume`, `--contest`, `--include-gym`, `--fill-missing-content`, and maintained auxiliary/debug flags.
- Luogu: expose `--sync-problemset`, `--fill-missing-content`, `--training-list`, and maintained auxiliary flags.
- SPOJ: expose `--sync-problemset`, `--fill-missing-content`, and maintained auxiliary flags, with any internal source selection kept hidden from the user where possible.

Alternative considered: remove old flags immediately. Rejected because historical job commands, README examples, and manual scripts may still depend on them.

### Decision 3: Use resume-by-default for contest crawling

AtCoder and Codeforces already persist fetched contests in JSON progress files. The canonical `--fetch-contest` should skip already-fetched contests by default. `--no-resume` opts out by passing `resume=False` to existing contest crawl methods.

Legacy `--resume` remains accepted as a compatibility no-op under the new default. Legacy `--fetch-all` should map to the same behavior as `--fetch-contest` so old commands continue to execute, with `--no-resume` available for the previous non-resume behavior.

Alternative considered: keep legacy `--fetch-all` non-resuming unless paired with `--resume`. Rejected because it would preserve the confusing split the change is intended to remove, and the new explicit `--no-resume` provides a clearer escape hatch.

### Decision 4: Keep Rust validation as the server-side security boundary

`src/models.rs` remains responsible for per-source flag allowlisting, arity, and value validation before the admin endpoint spawns Python. The change updates the `ArgSpec` lists to include canonical flags and aliases. Unsupported canonical operations, such as `--fetch-contest` for LeetCode/Luogu/SPOJ, remain absent from those source allowlists.

Alternative considered: accept arbitrary crawler arguments and let Python reject them. Rejected because the admin API currently uses Rust validation as a subprocess safety boundary.

### Decision 5: Keep admin UI flag metadata static for this change

`static/admin.js` will keep its existing `CRAWLER_CONFIG` structure, but the entries will be reorganized around canonical operations. This avoids adding a new endpoint or code generation path during a CLI cleanup.

Alternative considered: generate admin UI options from `ArgSpec` or serve them via an endpoint. Rejected for this change because it touches API shape and frontend rendering patterns beyond the requested refactor.

## Risks / Trade-offs

- [Risk] Legacy `--fetch-all` now resumes by default, which may surprise operators expecting a full rescan. -> Mitigation: document `--no-resume` and keep it accepted through both Python and Rust validation.
- [Risk] Flag definitions remain duplicated between Python argparse, Rust `ArgSpec`, and JS UI config. -> Mitigation: add tests/checks around Rust validation and Python help/parse behavior for the canonical flags.
- [Risk] SPOJ uses `luogu.py`, so `--sync-problemset` needs source-specific dispatch without confusing Luogu behavior. -> Mitigation: keep source routing in Rust/Admin UI and preserve hidden `--source spoj` or equivalent internal handling where needed.
- [Risk] Existing documentation may keep teaching legacy flags. -> Mitigation: update README to present canonical flags first and list legacy aliases as compatibility notes.
- [Risk] Unsupported operation combinations could reach Python from direct CLI use. -> Mitigation: argparse should reject unsupported flags per script; Rust should reject unsupported flags per admin source before spawning.

## Migration Plan

1. Add canonical argparse flags and alias mapping in Python scripts.
2. Update Rust `ArgSpec` allowlists to accept canonical flags and compatibility aliases per source.
3. Update admin UI crawler options to expose canonical operations and retained auxiliary flags.
4. Update i18n labels for new flags.
5. Update README crawler documentation to describe the canonical operation model.
6. Add focused tests or checks for:
   - Rust validation accepts supported canonical flags and rejects unsupported source/flag combinations.
   - Python CLIs expose the canonical flags in help output.
   - AtCoder/Codeforces `--fetch-contest` defaults to resume and honors `--no-resume`.

Rollback is straightforward because this change is additive: revert the new aliases/UI changes, and legacy flags remain the known working interface.

## Open Questions

- Should legacy flags stay visible in the admin UI as advanced/debug controls, or only remain accepted by direct CLI and Rust validation?
- Should `--full` for LeetCode remain a separately exposed admin action, or should it be treated as an advanced legacy operation because it fetches every problem detail rather than only initial metadata?
