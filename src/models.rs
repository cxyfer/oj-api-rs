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
    pub started_at: String,
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
