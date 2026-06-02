## 1. Codeforces Standings Request

- [x] 1.1 Update `scripts/codeforces.py` so `fetch_contest_problems` builds `contest.standings` URLs with only `contestId`.
- [x] 1.2 Confirm no other Codeforces contest standings URL construction still appends `from` or `count`.

## 2. Verification

- [x] 2.1 Add or update a focused regression test if the current crawler test structure supports URL construction checks.
- [x] 2.2 Run Python formatting and lint checks for the touched crawler script.
- [x] 2.3 Run the relevant crawler test or a focused smoke check that confirms contest `2214` standings discovery uses `https://codeforces.com/api/contest.standings?contestId=2214`.
