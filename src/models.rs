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
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
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

#[derive(Debug, Clone)]
pub enum CrawlerAction {
    None,
    Daily,
    Date(String),
    Init,
    Monthly(u16, u8),
}

impl CrawlerAction {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        if args.is_empty() {
            return Ok(Self::None);
        }
        match args[0].as_str() {
            "--daily" if args.len() == 1 => Ok(Self::Daily),
            "--init" if args.len() == 1 => Ok(Self::Init),
            "--date" if args.len() == 2 => {
                let date = &args[1];
                let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
                if !re.is_match(date) {
                    return Err("invalid date format, expected YYYY-MM-DD".into());
                }
                if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
                    return Err("invalid calendar date".into());
                }
                Ok(Self::Date(date.clone()))
            }
            "--monthly" if args.len() == 3 => {
                let year: u16 = args[1].parse().map_err(|_| "invalid year")?;
                let month: u8 = args[2].parse().map_err(|_| "invalid month")?;
                if !(2000..=2100).contains(&year) {
                    return Err("year must be between 2000 and 2100".into());
                }
                if !(1..=12).contains(&month) {
                    return Err("month must be between 1 and 12".into());
                }
                Ok(Self::Monthly(year, month))
            }
            other => Err(format!("unknown or malformed argument: {}", other)),
        }
    }

    pub fn to_args(&self) -> Vec<String> {
        match self {
            Self::None => vec![],
            Self::Daily => vec!["--daily".into()],
            Self::Date(d) => vec!["--date".into(), d.clone()],
            Self::Init => vec!["--init".into()],
            Self::Monthly(y, m) => vec!["--monthly".into(), y.to_string(), m.to_string()],
        }
    }
}

pub struct DailyFallbackEntry {
    pub status: CrawlerStatus,
    pub started_at: tokio::time::Instant,
    pub cooldown_until: Option<tokio::time::Instant>,
}
