## Context

Crawler proxy configuration already accepts `socks5h://`, but aiohttp-based session creation passes that URL directly to `aiohttp_socks.ProxyConnector.from_url()`. The installed `python-socks` URL parser rejects `socks5h://`, so `diag.py --test codeforces` and any aiohttp-based crawler fail before making a request.

curl_cffi accepts `socks5h://` in its `proxies` mapping, so the fix should target the aiohttp connector path without changing curl-based crawlers.

## Goals / Non-Goals

**Goals:**

- Make `_create_aiohttp_session()` accept `socks5h://` proxies.
- Preserve existing `socks5://`, HTTP/HTTPS proxy, and curl_cffi behavior.
- Keep config validation aligned with runtime behavior.
- Add targeted tests for `socks5h://` parsing and remote DNS semantics.
- Update examples so users know `socks5h://` is supported and means proxy-side DNS resolution.

**Non-Goals:**

- Do not add new proxy schemes beyond `http`, `https`, `socks5`, and `socks5h`.
- Do not change crawler transport choices such as Codeforces/Luogu using curl_cffi.
- Do not introduce new dependencies.
- Do not require a live proxy server for unit tests.

## Decisions

1. Handle `socks5h://` in BaseCrawler, not in each crawler.
   - Rationale: all aiohttp-based crawlers and `diag.py` share `_create_aiohttp_session()`.
   - Alternative considered: special-case `diag.py --test codeforces` to use curl_cffi. That would leave LeetCode/AtCoder aiohttp paths broken.

2. Map `socks5h://` to `ProxyConnector(..., proxy_type=ProxyType.SOCKS5, rdns=True)`.
   - Rationale: `ProxyConnector.__init__` supports `rdns`; `from_url()` fails only because URL parsing rejects the `socks5h` scheme.
   - Alternative considered: normalize `socks5h://` to `socks5://` before `from_url()`. That loses remote DNS semantics.

3. Keep `socks5://` on `ProxyConnector.from_url()`.
   - Rationale: it is existing working behavior and avoids reimplementing parser behavior unnecessarily.
   - Alternative considered: manually parse both SOCKS schemes. That increases surface area without fixing an observed bug.

4. Leave curl_cffi proxy URLs unchanged.
   - Rationale: runtime smoke tests show `curl_cffi` accepts `socks5h://`; preserving the original URL keeps libcurl remote DNS semantics.

## Risks / Trade-offs

- Manual URL parsing could miss edge cases → Keep parsing narrow, validate host/port, and cover credentials in tests.
- `socks5h://` without a port may pass current config validation → require a valid port for SOCKS connector creation and fail early with a descriptive error.
- Tests may couple to aiohttp-socks private fields when checking `rdns` → prefer testing connector construction behavior; if private inspection is used, keep it limited to the targeted regression.
