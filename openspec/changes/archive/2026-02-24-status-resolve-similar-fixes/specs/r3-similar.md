# Spec: R3 — Fix similar_by_text query parameter

## Requirement
Accept `q` as alias for `query` parameter. Strip surrounding double quotes from the value.

## Scenarios

### S3.1: Parameter alias `q`
- **When**: GET /api/v1/similar?q=binary+search
- **Then**: accepted, searches for "binary search"

### S3.2: Original parameter `query` still works
- **When**: GET /api/v1/similar?query=binary+search
- **Then**: accepted, searches for "binary search"

### S3.3: URL-encoded quotes stripped
- **When**: GET /api/v1/similar?q=%22two-sum%22 (decoded: `"two-sum"`)
- **Then**: quotes stripped, searches for "two-sum"

### S3.4: Unbalanced quotes not stripped
- **When**: GET /api/v1/similar?q=%22two-sum (decoded: `"two-sum`)
- **Then**: NOT stripped, searches for `"two-sum` (7 chars, passes length check)

### S3.5: Length validation after stripping
- **When**: GET /api/v1/similar?q=%22ab%22 (decoded: `"ab"`, stripped: `ab`)
- **Then**: 400 "query must be at least 3 characters" (2 chars after stripping)

### S3.6: Empty after stripping
- **When**: GET /api/v1/similar?q=%22%22 (decoded: `""`, stripped: empty)
- **Then**: 400 "query parameter is required" (None-equivalent after stripping)

## PBT Properties

### P3.1: Alias equivalence
- `?q=X` and `?query=X` produce identical behavior for any value X.

### P3.2: Quote stripping idempotency
- Stripping is applied at most once. `"\"hello\""` → `"hello"` (one layer removed), not `hello`.

### P3.3: Non-quoted passthrough
- If value does not start AND end with `"`, it passes through unchanged.

### P3.4: Length bounds preserved
- After stripping, if len < 3 → 400. If len > 2000 → 400. Always enforced.
