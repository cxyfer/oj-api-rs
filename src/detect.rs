use regex::Regex;
use std::sync::LazyLock;

static ATCODER_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)atcoder\.jp/contests/([^/]+)/tasks/([^/?#]+)").unwrap());

static LEETCODE_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)leetcode\.(?:com|cn)/(?:contest/[^/]+/)?problems/([^/?#]+)").unwrap()
});

static CODEFORCES_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:https?://)?(?:www\.)?codeforces\.com/(?:contest/(\d+)/problem/([A-Z0-9]+)|problemset/problem/(\d+)/([A-Z0-9]+))",
    )
    .unwrap()
});

static LUOGU_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)luogu\.com\.cn/problem/([A-Z0-9_]+)").unwrap());

static ATCODER_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(abc|arc|agc|ahc)\d+_[a-z]\d*$").unwrap());

static CF_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:CF)?\d+[A-Z]\d*$").unwrap());

static LUOGU_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([PBTU]\d+|CF\d+[A-Z]|AT_(?:abc|arc|agc|ahc)\d+_[a-z]\d*|UVA\d+)$")
        .unwrap()
});

static SP_ID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^SP\d+$").unwrap());

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
        return (
            "codeforces",
            format!("{}{}", contest_id, index).to_uppercase(),
        );
    }
    if let Some(caps) = LUOGU_URL_RE.captures(pid) {
        let luogu_pid = caps[1].to_uppercase();
        if let Some(stripped) = luogu_pid.strip_prefix("CF") {
            return ("codeforces", stripped.to_string());
        }
        if let Some(stripped) = luogu_pid.strip_prefix("AT_") {
            return ("atcoder", stripped.to_lowercase());
        }
        if luogu_pid.starts_with("AT") {
            return ("atcoder", luogu_pid.to_lowercase());
        }
        if luogu_pid.starts_with("SP") && SP_ID_RE.is_match(&luogu_pid) {
            return ("spoj", luogu_pid);
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

    // SPOJ SP\d+ pattern
    if SP_ID_RE.is_match(pid) {
        return ("spoj", pid.to_uppercase());
    }

    // Luogu patterns
    if LUOGU_ID_RE.is_match(pid) {
        return ("luogu", pid.to_uppercase());
    }

    // Default: LeetCode slug
    ("leetcode", pid.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spoj_id() {
        assert_eq!(detect_source("SP1"), ("spoj", "SP1".to_string()));
        assert_eq!(detect_source("sp1"), ("spoj", "SP1".to_string()));
        assert_eq!(detect_source("SP12345"), ("spoj", "SP12345".to_string()));
    }

    #[test]
    fn test_spoj_luogu_url() {
        assert_eq!(
            detect_source("https://www.luogu.com.cn/problem/SP1"),
            ("spoj", "SP1".to_string())
        );
    }

    #[test]
    fn test_atcoder_url() {
        assert_eq!(
            detect_source("https://atcoder.jp/contests/abc300/tasks/abc300_a"),
            ("atcoder", "abc300_a".to_string())
        );
        assert_eq!(
            detect_source("https://atcoder.jp/contests/arc100/tasks/arc100_c"),
            ("atcoder", "arc100_c".to_string())
        );
        assert_eq!(
            detect_source("https://atcoder.jp/contests/agc050/tasks/agc050_a"),
            ("atcoder", "agc050_a".to_string())
        );
    }

    #[test]
    fn test_leetcode_url() {
        assert_eq!(
            detect_source("https://leetcode.com/problems/two-sum/"),
            ("leetcode", "two-sum".to_string())
        );
        assert_eq!(
            detect_source("https://leetcode.cn/problems/two-sum/"),
            ("leetcode", "two-sum".to_string())
        );
        assert_eq!(
            detect_source("https://leetcode.com/contest/biweekly-100/problems/two-sum/"),
            ("leetcode", "two-sum".to_string())
        );
    }

    #[test]
    fn test_codeforces_url() {
        assert_eq!(
            detect_source("https://codeforces.com/contest/2000/problem/A"),
            ("codeforces", "2000A".to_string())
        );
        assert_eq!(
            detect_source("https://codeforces.com/problemset/problem/2000/A"),
            ("codeforces", "2000A".to_string())
        );
    }

    #[test]
    fn test_luogu_url() {
        let cases = vec![
            ("https://www.luogu.com.cn/problem/P1001", "luogu", "P1001"),
            (
                "https://www.luogu.com.cn/problem/CF1900A",
                "codeforces",
                "1900A",
            ),
            (
                "https://www.luogu.com.cn/problem/AT_abc300_a",
                "atcoder",
                "abc300_a",
            ),
            ("https://www.luogu.com.cn/problem/SP1", "spoj", "SP1"),
        ];

        for (input, expected_source, expected_id) in cases {
            assert_eq!(
                detect_source(input),
                (expected_source, expected_id.to_string())
            );
        }
    }

    #[test]
    fn test_luogu_id_unaffected() {
        assert_eq!(detect_source("P1000"), ("luogu", "P1000".to_string()));
        assert_eq!(detect_source("B2001"), ("luogu", "B2001".to_string()));
        assert_eq!(detect_source("T1000"), ("luogu", "T1000".to_string()));
        assert_eq!(detect_source("U12345"), ("luogu", "U12345".to_string()));
        assert_eq!(detect_source("UVA100"), ("luogu", "UVA100".to_string()));
    }

    #[test]
    fn test_atcoder_id() {
        assert_eq!(detect_source("abc300_a"), ("atcoder", "abc300_a".to_string()));
        assert_eq!(detect_source("arc100_c"), ("atcoder", "arc100_c".to_string()));
        assert_eq!(detect_source("agc050_a"), ("atcoder", "agc050_a".to_string()));
        assert_eq!(detect_source("ahc001_a"), ("atcoder", "ahc001_a".to_string()));
    }

    #[test]
    fn test_codeforces_id() {
        let cases = vec![
            ("CF1900A", "1900A"),
            ("1900A", "1900A"),
            ("1999B1", "1999B1"),
            ("CF1999B1", "1999B1"),
        ];

        for (input, expected_id) in cases {
            assert_eq!(detect_source(input), ("codeforces", expected_id.to_string()));
        }
    }

    #[test]
    fn test_codeforces_id_consistency() {
        let cases = vec![
            ("CF1900A", "1900A"),
            ("https://www.luogu.com.cn/problem/CF1900A", "1900A"),
        ];

        for (input, expected_id) in cases {
            assert_eq!(detect_source(input), ("codeforces", expected_id.to_string()));
        }
    }

    #[test]
    fn test_leetcode_numeric() {
        assert_eq!(detect_source("1"), ("leetcode", "1".to_string()));
        assert_eq!(detect_source("two-sum"), ("leetcode", "two-sum".to_string()));
    }

    #[test]
    fn test_prefix_format() {
        assert_eq!(
            detect_source("atcoder:abc321_a"),
            ("atcoder", "abc321_a".to_string())
        );
        assert_eq!(
            detect_source("codeforces:1900A"),
            ("codeforces", "1900A".to_string())
        );
        assert_eq!(
            detect_source("LeetCode:two-sum"),
            ("leetcode", "two-sum".to_string())
        );
        assert_eq!(detect_source("luogu:P1000"), ("luogu", "P1000".to_string()));
        assert_eq!(detect_source("uva:100"), ("uva", "100".to_string()));
        assert_eq!(detect_source("spoj:SP1"), ("spoj", "SP1".to_string()));
    }

    #[test]
    fn test_invalid_prefix_fallback() {
        assert_eq!(
            detect_source("invalid:abc"),
            ("leetcode", "invalid:abc".to_string())
        );
        assert_eq!(
            detect_source("foo:bar:baz"),
            ("leetcode", "foo:bar:baz".to_string())
        );
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(detect_source(""), ("unknown", "".to_string()));
        assert_eq!(detect_source("   "), ("unknown", "".to_string()));
        assert_eq!(detect_source("\t\n"), ("unknown", "".to_string()));
    }

    #[test]
    fn test_unknown_url() {
        assert_eq!(
            detect_source("https://example.com/problem/1"),
            ("unknown", "https://example.com/problem/1".to_string())
        );
        assert_eq!(
            detect_source("http://unknown-oj.com/p/123"),
            ("unknown", "http://unknown-oj.com/p/123".to_string())
        );
    }

    #[test]
    fn test_case_normalization() {
        assert_eq!(detect_source("ABC300_A"), ("atcoder", "abc300_a".to_string()));
        assert_eq!(detect_source("cf1900a"), ("codeforces", "1900A".to_string()));
        assert_eq!(detect_source("Sp1"), ("spoj", "SP1".to_string()));
    }
}
