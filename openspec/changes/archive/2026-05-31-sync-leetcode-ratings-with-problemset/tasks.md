## 1. Rating merge flow

- [x] 1.1 Add a LeetCode helper that merges fetched rating records into problemset metadata by normalized frontend problem id
- [x] 1.2 Update `init_all_problems()` so `--sync-problemset` and `--init` attempt rating merge before persistence
- [x] 1.3 Ensure rating fetch failures or empty rating payloads log a warning and continue problemset sync

## 2. Metadata-safe persistence

- [x] 2.1 Add or adapt a database update path for LeetCode problemset metadata refreshes that updates rating, contest, problem index, and basic metadata on existing rows
- [x] 2.2 Ensure the metadata refresh path preserves existing content, content_cn, tags, and similar_questions when incoming problemset rows do not provide detail data
- [x] 2.3 Ensure existing positive ratings are not overwritten by zero or null placeholders when rating metadata is unavailable

## 3. Tests and validation

- [x] 3.1 Add Python tests for rating merge by normalized problem id
- [x] 3.2 Add Python tests proving LeetCode problemset sync refreshes existing zero ratings
- [x] 3.3 Add Python tests proving rating-source failure preserves existing positive ratings and still persists problem metadata
- [x] 3.4 Add Python tests proving problemset sync does not clear existing detail fields
- [x] 3.5 Run the relevant Python test suite and format/check touched Python files
