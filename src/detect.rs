use regex::Regex;
use std::sync::LazyLock;

static ATCODER_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)atcoder\.jp/contests/([^/]+)/tasks/([^/?#]+)").unwrap()
});

static LEETCODE_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)leetcode\.(?:com|cn)/(?:contest/[^/]+/)?problems/([^/?#]+)").unwrap()
});

static CODEFORCES_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:https?://)?(?:www\.)?codeforces\.com/(?:contest/(\d+)/problem/([A-Z0-9]+)|problemset/problem/(\d+)/([A-Z0-9]+))",
    )
    .unwrap()
});

static LUOGU_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)luogu\.com\.cn/problem/([A-Z0-9_]+)").unwrap()
});

static ATCODER_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(abc|arc|agc|ahc)\d+_[a-z]\d*$").unwrap()
});

static CF_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:CF)?\d+[A-Z]\d*$").unwrap()
});

static LUOGU_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([PBTU]\d+|CF\d+[A-Z]|AT_(?:abc|arc|agc|ahc)\d+_[a-z]\d*|UVA\d+|SP\d+)$")
        .unwrap()
});

const VALID_SOURCES: &[&str] = &["atcoder", "leetcode", "codeforces", "luogu", "uva", "spoj"];

pub fn detect_source(input: &str) -> (&'static str, String) {
    let pid = input.trim();
    if pid.is_empty() {
        return ("unknown", String::new());
    }

    // URL detection
    if let Some(caps) = ATCODER_URL_RE.captures(pid) {
        return ("atcoder", caps[2].to_lowercase());
    }
    if let Some(caps) = LEETCODE_URL_RE.captures(pid) {
        return ("leetcode", caps[1].to_string());
    }
    if let Some(caps) = CODEFORCES_URL_RE.captures(pid) {
        let contest_id = caps
            .get(1)
            .or_else(|| caps.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");
        let index = caps
            .get(2)
            .or_else(|| caps.get(4))
            .map(|m| m.as_str())
            .unwrap_or("");
        return ("codeforces", format!("{}{}", contest_id, index).to_uppercase());
    }
    if let Some(caps) = LUOGU_URL_RE.captures(pid) {
        let luogu_pid = caps[1].to_uppercase();
        if luogu_pid.starts_with("CF") {
            return ("codeforces", luogu_pid);
        }
        if luogu_pid.starts_with("AT_") {
            return ("atcoder", luogu_pid[3..].to_lowercase());
        }
        if luogu_pid.starts_with("AT") {
            return ("atcoder", luogu_pid.to_lowercase());
        }
        return ("luogu", luogu_pid);
    }

    // Unknown URL
    if pid.contains("://") {
        return ("unknown", pid.to_string());
    }

    // Prefix detection
    if pid.matches(':').count() == 1 {
        let parts: Vec<&str> = pid.splitn(2, ':').collect();
        if parts.len() == 2 {
            let src = parts[0].to_lowercase();
            if VALID_SOURCES.contains(&src.as_str()) {
                return (
                    VALID_SOURCES.iter().find(|&&s| s == src).unwrap(),
                    parts[1].to_string(),
                );
            }
        }
    }

    // Pure numeric → LeetCode
    if pid.chars().all(|c| c.is_ascii_digit()) {
        return ("leetcode", pid.to_string());
    }

    // AtCoder ID pattern
    if ATCODER_ID_RE.is_match(pid) {
        return ("atcoder", pid.to_lowercase());
    }

    // Codeforces CF prefix or digit+letter pattern
    if pid.to_uppercase().starts_with("CF") && CF_ID_RE.is_match(pid) {
        // Strip CF prefix
        return ("codeforces", pid[2..].to_uppercase());
    }
    if CF_ID_RE.is_match(pid) {
        return ("codeforces", pid.to_uppercase());
    }

    // Luogu patterns
    if LUOGU_ID_RE.is_match(pid) {
        return ("luogu", pid.to_uppercase());
    }

    // Default: LeetCode slug
    ("leetcode", pid.to_string())
}
