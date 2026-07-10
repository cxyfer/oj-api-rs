## 1. Proxy Runtime Implementation

- [x] 1.1 Add a BaseCrawler helper that builds SOCKS connectors and maps `socks5h://` to SOCKS5 with remote DNS enabled
- [x] 1.2 Update `_create_aiohttp_session()` to use the helper for both `socks5://` and `socks5h://`
- [x] 1.3 Tighten proxy URL validation so SOCKS proxies require an explicit port

## 2. Tests and Smoke Checks

- [x] 2.1 Add unit tests for `socks5h://` aiohttp connector creation, credentials, request-level proxy behavior, and curl_cffi proxy preservation
- [x] 2.2 Add config validation tests for valid `socks5h://` URLs and SOCKS URLs missing a port
- [x] 2.3 Run targeted `uv` tests and a `diag.py --test codeforces` smoke check against a `socks5h://` config that validates initialization behavior

## 3. Documentation

- [x] 3.1 Update `config.toml.example` to list supported proxy schemes and describe `socks5h://` remote DNS behavior
- [x] 3.2 Update README crawler proxy examples to include `socks5h://`

## 4. Final Verification

- [x] 4.1 Run Python formatting/checks for modified scripts files
- [x] 4.2 Run relevant project checks and confirm OpenSpec status is apply-ready
