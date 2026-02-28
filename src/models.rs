use std::fmt;
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Serialize};

fn deserialize_json_array<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            serde_json::from_str::<Vec<String>>(&s).or_else(|_| Ok(Vec::new()))
        }
        _ => Ok(Vec::new()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: String,
    pub source: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    #[serde(deserialize_with = "deserialize_json_array", default)]
    pub tags: Vec<String>,
    pub link: Option<String>,
    pub category: Option<String>,
    pub paid_only: Option<i32>,
    pub content: Option<String>,
    pub content_cn: Option<String>,
    #[serde(deserialize_with = "deserialize_json_array", default)]
    pub similar_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProblemSummary {
    pub id: String,
    pub source: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    pub tags: Vec<String>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyChallenge {
    pub date: String,
    pub domain: String,
    pub id: String,
    pub slug: String,
    pub title: Option<String>,
    pub title_cn: Option<String>,
    pub difficulty: Option<String>,
    pub ac_rate: Option<f64>,
    pub rating: Option<f64>,
    pub contest: Option<String>,
    pub problem_index: Option<String>,
    #[serde(deserialize_with = "deserialize_json_array", default)]
    pub tags: Vec<String>,
    pub link: Option<String>,
    pub category: Option<String>,
    pub paid_only: Option<i32>,
    pub content: Option<String>,
    pub content_cn: Option<String>,
    #[serde(deserialize_with = "deserialize_json_array", default)]
    pub similar_questions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeetCodeDomain {
    Com,
    Cn,
}

impl LeetCodeDomain {
    pub fn today(&self) -> String {
        match self {
            Self::Com => Utc::now().format("%Y-%m-%d").to_string(),
            Self::Cn => {
                let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
                Utc::now().with_timezone(&cst).format("%Y-%m-%d").to_string()
            }
        }
    }

    pub fn today_naive(&self) -> chrono::NaiveDate {
        match self {
            Self::Com => Utc::now().date_naive(),
            Self::Cn => {
                let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
                Utc::now().with_timezone(&cst).date_naive()
            }
        }
    }
}

impl fmt::Display for LeetCodeDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Com => write!(f, "com"),
            Self::Cn => write!(f, "cn"),
        }
    }
}

impl FromStr for LeetCodeDomain {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "com" => Ok(Self::Com),
            "cn" => Ok(Self::Cn),
            _ => Err(format!("invalid domain: {}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for LeetCodeDomain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for LeetCodeDomain {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiToken {
    pub token: String,
    pub label: Option<String>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub is_active: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrawlerJob {
    pub job_id: String,
    pub source: String,
    pub args: Vec<String>,
    pub trigger: CrawlerTrigger,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: CrawlerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

impl CrawlerJob {
    pub fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.stdout = if stdout.is_empty() {
            None
        } else {
            let s = if stdout.len() > MAX_OUTPUT_BYTES {
                String::from_utf8_lossy(&stdout[stdout.len() - MAX_OUTPUT_BYTES..]).into_owned()
            } else {
                String::from_utf8_lossy(&stdout).into_owned()
            };
            Some(s)
        };
        self.stderr = if stderr.is_empty() {
            None
        } else {
            let s = if stderr.len() > MAX_OUTPUT_BYTES {
                String::from_utf8_lossy(&stderr[stderr.len() - MAX_OUTPUT_BYTES..]).into_owned()
            } else {
                String::from_utf8_lossy(&stderr).into_owned()
            };
            Some(s)
        };
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerTrigger {
    Admin,
    DailyFallback,
}

impl std::fmt::Display for CrawlerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::TimedOut => write!(f, "timed_out"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::fmt::Display for CrawlerTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::DailyFallback => write!(f, "daily_fallback"),
        }
    }
}

// Per-source argument validation

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    None,
    Date,
    Int,
    Float,
    Str,
    YearMonth,
    Domain,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ArgSpec {
    pub flag: &'static str,
    pub arity: u8,
    pub value_type: ValueType,
    pub ui_exposed: bool,
}

pub static LEETCODE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--init",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--full",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--daily",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--date",
        arity: 1,
        value_type: ValueType::Date,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--monthly",
        arity: 2,
        value_type: ValueType::YearMonth,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content-workers",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--domain",
        arity: 1,
        value_type: ValueType::Domain,
        ui_exposed: true,
    },
];

pub static ATCODER_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--sync-kenkoooo",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--sync-history",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fetch-all",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--resume",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--contest",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--status",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--reprocess-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static CODEFORCES_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--sync-problemset",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fetch-all",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--resume",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--contest",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--status",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-problems",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--reprocess-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--include-gym",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static LUOGU_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--sync",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--status",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--overwrite",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--batch-size",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--training-list",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--source",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static SPOJ_ARGS: &[ArgSpec] = &[
    ArgSpec {
        flag: "--sync-spoj",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--fill-missing-content",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--missing-content-stats",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--overwrite",
        arity: 0,
        value_type: ValueType::None,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--source",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--rate-limit",
        arity: 1,
        value_type: ValueType::Float,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--batch-size",
        arity: 1,
        value_type: ValueType::Int,
        ui_exposed: true,
    },
    ArgSpec {
        flag: "--data-dir",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
    ArgSpec {
        flag: "--db-path",
        arity: 1,
        value_type: ValueType::Str,
        ui_exposed: false,
    },
];

pub static DIAG_ARGS: &[ArgSpec] = &[ArgSpec {
    flag: "--test",
    arity: 1,
    value_type: ValueType::Str,
    ui_exposed: true,
}];

#[derive(Debug, Clone, Copy)]
pub enum CrawlerSource {
    LeetCode,
    AtCoder,
    Codeforces,
    Luogu,
    Spoj,
    Diag,
}

impl CrawlerSource {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "leetcode" => Ok(Self::LeetCode),
            "atcoder" => Ok(Self::AtCoder),
            "codeforces" => Ok(Self::Codeforces),
            "luogu" => Ok(Self::Luogu),
            "spoj" => Ok(Self::Spoj),
            "diag" => Ok(Self::Diag),
            _ => Err(format!("invalid source: {}", s)),
        }
    }

    pub fn script_name(&self) -> &'static str {
        match self {
            Self::LeetCode => "leetcode.py",
            Self::AtCoder => "atcoder.py",
            Self::Codeforces => "codeforces.py",
            Self::Luogu => "luogu.py",
            Self::Spoj => "luogu.py",
            Self::Diag => "diag.py",
        }
    }

    pub fn arg_specs(&self) -> &'static [ArgSpec] {
        match self {
            Self::LeetCode => LEETCODE_ARGS,
            Self::AtCoder => ATCODER_ARGS,
            Self::Codeforces => CODEFORCES_ARGS,
            Self::Luogu => LUOGU_ARGS,
            Self::Spoj => SPOJ_ARGS,
            Self::Diag => DIAG_ARGS,
        }
    }
}

pub fn validate_args(source: &CrawlerSource, raw_args: &[String]) -> Result<Vec<String>, String> {
    let specs = source.arg_specs();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0;

    while i < raw_args.len() {
        let token = &raw_args[i];
        if !token.starts_with("--") {
            return Err(format!("unexpected value without flag: {}", token));
        }

        let spec = specs
            .iter()
            .find(|s| s.flag == token)
            .ok_or_else(|| format!("unknown argument: {}", token))?;

        if !seen.insert(spec.flag) {
            return Err(format!("duplicate argument: {}", token));
        }

        let arity = spec.arity as usize;
        if i + arity >= raw_args.len() {
            return Err(format!("{} requires {} value(s)", token, arity));
        }

        match spec.value_type {
            ValueType::None => {}
            ValueType::Date => {
                let v = &raw_args[i + 1];
                if chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() {
                    return Err(format!(
                        "{}: invalid date '{}', expected YYYY-MM-DD",
                        token, v
                    ));
                }
            }
            ValueType::Int => {
                let v = &raw_args[i + 1];
                if v.parse::<u64>().is_err() {
                    return Err(format!("{}: invalid integer '{}'", token, v));
                }
            }
            ValueType::Float => {
                let v = &raw_args[i + 1];
                match v.parse::<f64>() {
                    Ok(f) if f.is_finite() && f > 0.0 => {}
                    _ => return Err(format!("{}: invalid positive float '{}'", token, v)),
                }
            }
            ValueType::Str => {
                let v = &raw_args[i + 1];
                if v.is_empty() {
                    return Err(format!("{}: value must not be empty", token));
                }
                if spec.flag == "--data-dir" || spec.flag == "--db-path" {
                    if v.starts_with('/') {
                        return Err(format!("{}: must be a relative path", token));
                    }
                    if v.contains("..") {
                        return Err(format!("{}: must not contain '..'", token));
                    }
                }
            }
            ValueType::Domain => {
                let v = &raw_args[i + 1];
                if v != "com" && v != "cn" {
                    return Err(format!(
                        "{}: invalid domain '{}', expected 'com' or 'cn'",
                        token, v
                    ));
                }
            }
            ValueType::YearMonth => {
                let yv = &raw_args[i + 1];
                let mv = &raw_args[i + 2];
                let year: u16 = yv
                    .parse()
                    .map_err(|_| format!("{}: invalid year '{}'", token, yv))?;
                let month: u8 = mv
                    .parse()
                    .map_err(|_| format!("{}: invalid month '{}'", token, mv))?;
                if !(2000..=2100).contains(&year) {
                    return Err(format!("{}: year must be between 2000 and 2100", token));
                }
                if !(1..=12).contains(&month) {
                    return Err(format!("{}: month must be between 1 and 12", token));
                }
            }
        }

        i += 1 + arity;
    }

    Ok(raw_args.to_vec())
}

pub struct DailyFallbackEntry {
    pub status: CrawlerStatus,
    pub started_at: tokio::time::Instant,
    pub cooldown_until: Option<tokio::time::Instant>,
}

// Embedding job model (parallel to CrawlerJob)

#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingJob {
    pub job_id: String,
    pub source: String,
    pub args: Vec<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: CrawlerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

impl EmbeddingJob {
    pub fn set_output(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.stdout = if stdout.is_empty() {
            None
        } else {
            let s = if stdout.len() > MAX_OUTPUT_BYTES {
                String::from_utf8_lossy(&stdout[stdout.len() - MAX_OUTPUT_BYTES..]).into_owned()
            } else {
                String::from_utf8_lossy(&stdout).into_owned()
            };
            Some(s)
        };
        self.stderr = if stderr.is_empty() {
            None
        } else {
            let s = if stderr.len() > MAX_OUTPUT_BYTES {
                String::from_utf8_lossy(&stderr[stderr.len() - MAX_OUTPUT_BYTES..]).into_owned()
            } else {
                String::from_utf8_lossy(&stderr).into_owned()
            };
            Some(s)
        };
    }
}
