use chrono::Utc;

#[derive(Debug, Clone, Copy)]
pub enum TimeRange {
    All,
    DaysAgo(i64),
}

impl TimeRange {
    pub fn should_delete(&self, timestamp: i64) -> bool {
        match self {
            TimeRange::All => true,
            TimeRange::DaysAgo(days) => {
                let now = Utc::now().timestamp();
                let cutoff = now - (days * 86400);
                timestamp < cutoff
            }
        }
    }

    pub fn description(&self) -> String {
        match self {
            TimeRange::All => "全部时间".to_string(),
            TimeRange::DaysAgo(days) => format!("{} 天前", days),
        }
    }
}

