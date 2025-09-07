use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionRecord {
    pub article_id: String,
    pub version: u32,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub editor: String,
}
