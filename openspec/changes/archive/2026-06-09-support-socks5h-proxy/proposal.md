## Why

Crawler proxy configuration accepts `socks5h://`, but aiohttp-based diagnostics and crawlers fail at runtime because `aiohttp_socks.ProxyConnector.from_url()` rejects that scheme. This makes per-crawler proxy validation misleading and breaks `diag.py --test codeforces` even though curl-based Codeforces fetching can accept `socks5h://`.

## What Changes

- Add explicit `socks5h://` handling for aiohttp SOCKS connectors by mapping it to SOCKS5 with remote DNS enabled.
- Preserve existing `socks5://`, HTTP, HTTPS, and curl_cffi proxy behavior.
- Cover `socks5h://` behavior with targeted tests and diagnostics-oriented smoke checks.
- Document supported crawler proxy schemes and clarify that `socks5h://` resolves DNS through the proxy.

## Capabilities

### New Capabilities

### Modified Capabilities

- `base-crawler`: aiohttp session creation must support `socks5h://` proxies with remote DNS instead of passing the unsupported scheme to `ProxyConnector.from_url()`.
- `crawler-config`: crawler proxy configuration must document and validate supported schemes consistently with runtime behavior.
- `config-example`: sample configuration must advertise `socks5h://` as a supported crawler proxy scheme.

## Impact

- Affected Python code: `scripts/utils/base_crawler.py`, proxy-related tests, and optionally `scripts/diag.py` smoke coverage.
- Affected docs/config examples: `config.toml.example`, README crawler configuration snippets.
- No API route, database schema, or Rust service behavior changes.
- No new runtime dependencies; the change uses existing `aiohttp-socks`, `python-socks`, and `curl_cffi` dependencies.
