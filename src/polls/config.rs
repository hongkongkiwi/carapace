//! Poll Configuration
//!
//! Configuration for creating and managing polls/surveys.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of poll
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PollType {
    /// Single choice (radio button style)
    #[default]
    SingleChoice,
    /// Multiple choice (checkbox style)
    MultipleChoice,
    /// Rating scale (1-5 stars)
    Rating,
    /// Open text response
    OpenEnded,
    /// Quiz with correct answers
    Quiz,
}

/// Poll visibility settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PollVisibility {
    /// Votes are visible in real-time
    #[default]
    Public,
    /// Votes are hidden until poll closes
    Hidden,
    /// Only poll creator can see results
    Private,
}

/// Poll option/choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    /// Option ID (unique within poll)
    pub id: String,
    /// Option text/label
    pub text: String,
    /// Optional emoji/icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// For quiz polls: whether this is correct
    #[serde(default)]
    pub is_correct: bool,
    /// Option description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PollOption {
    /// Create a new poll option
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            emoji: None,
            is_correct: false,
            description: None,
        }
    }

    /// Set emoji
    pub fn with_emoji(mut self, emoji: impl Into<String>) -> Self {
        self.emoji = Some(emoji.into());
        self
    }

    /// Mark as correct answer (for quizzes)
    pub fn correct(mut self) -> Self {
        self.is_correct = true;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Poll configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollConfig {
    /// Poll ID (unique identifier)
    pub id: String,
    /// Poll title/question
    pub title: String,
    /// Poll description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Poll type
    #[serde(default)]
    pub poll_type: PollType,
    /// Visibility setting
    #[serde(default)]
    pub visibility: PollVisibility,
    /// Poll options
    pub options: Vec<PollOption>,
    /// Whether poll is active
    #[serde(default = "default_true")]
    pub is_active: bool,
    /// Whether poll is closed
    #[serde(default)]
    pub is_closed: bool,
    /// Allow users to change their vote
    #[serde(default)]
    pub allow_change_vote: bool,
    /// Allow anonymous voting
    #[serde(default)]
    pub allow_anonymous: bool,
    /// Minimum number of selections (for multiple choice)
    #[serde(default)]
    pub min_selections: u32,
    /// Maximum number of selections (for multiple choice, 0 = unlimited)
    #[serde(default)]
    pub max_selections: u32,
    /// For rating polls: minimum value
    #[serde(default = "default_one")]
    pub rating_min: u32,
    /// For rating polls: maximum value
    #[serde(default = "default_five")]
    pub rating_max: u32,
    /// Duration in seconds (0 = no expiry)
    #[serde(default)]
    pub duration_seconds: u64,
    /// Channel ID where poll was created
    pub channel_id: String,
    /// User ID of poll creator
    pub created_by: String,
    /// When poll was created (Unix ms)
    pub created_at: i64,
    /// When poll closes (Unix ms, 0 = manual close)
    #[serde(default)]
    pub closes_at: i64,
    /// Tags for organizing polls
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether to show vote counts in real-time
    #[serde(default = "default_true")]
    pub show_vote_count: bool,
}

fn default_true() -> bool {
    true
}

fn default_one() -> u32 {
    1
}

fn default_five() -> u32 {
    5
}

impl PollConfig {
    /// Create a new poll configuration
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        channel_id: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            poll_type: PollType::SingleChoice,
            visibility: PollVisibility::Public,
            options: Vec::new(),
            is_active: true,
            is_closed: false,
            allow_change_vote: false,
            allow_anonymous: false,
            min_selections: 0,
            max_selections: 0,
            rating_min: 1,
            rating_max: 5,
            duration_seconds: 0,
            channel_id: channel_id.into(),
            created_by: created_by.into(),
            created_at: now_millis(),
            closes_at: 0,
            tags: Vec::new(),
            show_vote_count: true,
        }
    }

    /// Set poll type
    pub fn with_type(mut self, poll_type: PollType) -> Self {
        self.poll_type = poll_type;
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visibility: PollVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Add an option
    pub fn add_option(&mut self, option: PollOption) {
        self.options.push(option);
    }

    /// Set options
    pub fn with_options(mut self, options: Vec<PollOption>) -> Self {
        self.options = options;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set duration
    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.duration_seconds = seconds;
        if seconds > 0 {
            self.closes_at = self.created_at + (seconds as i64 * 1000);
        }
        self
    }

    /// Set allow change vote
    pub fn allow_change_vote(mut self, allow: bool) -> Self {
        self.allow_change_vote = allow;
        self
    }

    /// Set allow anonymous
    pub fn allow_anonymous(mut self, allow: bool) -> Self {
        self.allow_anonymous = allow;
        self
    }

    /// Set selection limits (for multiple choice)
    pub fn with_selection_limits(mut self, min: u32, max: u32) -> Self {
        self.min_selections = min;
        self.max_selections = max;
        self
    }

    /// Set rating range
    pub fn with_rating_range(mut self, min: u32, max: u32) -> Self {
        self.rating_min = min;
        self.rating_max = max;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Close the poll
    pub fn close(&mut self) {
        self.is_closed = true;
        self.is_active = false;
    }

    /// Reopen the poll
    pub fn reopen(&mut self) {
        self.is_closed = false;
        self.is_active = true;
    }

    /// Check if poll has expired
    pub fn is_expired(&self) -> bool {
        if self.closes_at == 0 {
            return false;
        }
        now_millis() > self.closes_at
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Poll title is required".to_string());
        }

        match self.poll_type {
            PollType::SingleChoice | PollType::MultipleChoice | PollType::Quiz => {
                if self.options.len() < 2 {
                    return Err("Poll must have at least 2 options".to_string());
                }
                if self.options.len() > 50 {
                    return Err("Poll cannot have more than 50 options".to_string());
                }

                // Check for duplicate option IDs
                let mut seen_ids = std::collections::HashSet::new();
                for option in &self.options {
                    if !seen_ids.insert(&option.id) {
                        return Err(format!("Duplicate option ID: {}", option.id));
                    }
                }

                // For quiz, at least one option should be correct
                if self.poll_type == PollType::Quiz && !self.options.iter().any(|o| o.is_correct) {
                    return Err("Quiz poll must have at least one correct answer".to_string());
                }
            }
            PollType::Rating => {
                if self.rating_min >= self.rating_max {
                    return Err("Rating min must be less than rating max".to_string());
                }
                if self.rating_max - self.rating_min > 10 {
                    return Err("Rating range cannot exceed 10".to_string());
                }
            }
            PollType::OpenEnded => {
                // Open ended doesn't need options
            }
        }

        if self.poll_type == PollType::MultipleChoice {
            if self.max_selections > 0 && self.min_selections > self.max_selections {
                return Err("Min selections cannot exceed max selections".to_string());
            }
        }

        Ok(())
    }
}

/// Poll vote record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollVote {
    /// Vote ID
    pub id: String,
    /// Poll ID
    pub poll_id: String,
    /// User ID who voted
    pub user_id: String,
    /// Selected option IDs (for choice polls)
    #[serde(default)]
    pub selected_options: Vec<String>,
    /// Rating value (for rating polls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<u32>,
    /// Open text response (for open-ended polls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_response: Option<String>,
    /// Whether vote is anonymous
    #[serde(default)]
    pub is_anonymous: bool,
    /// When vote was cast (Unix ms)
    pub voted_at: i64,
}

impl PollVote {
    /// Create a new vote for a choice poll
    pub fn new_choice(
        id: impl Into<String>,
        poll_id: impl Into<String>,
        user_id: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            poll_id: poll_id.into(),
            user_id: user_id.into(),
            selected_options: options,
            rating: None,
            text_response: None,
            is_anonymous: false,
            voted_at: now_millis(),
        }
    }

    /// Create a new rating vote
    pub fn new_rating(
        id: impl Into<String>,
        poll_id: impl Into<String>,
        user_id: impl Into<String>,
        rating: u32,
    ) -> Self {
        Self {
            id: id.into(),
            poll_id: poll_id.into(),
            user_id: user_id.into(),
            selected_options: Vec::new(),
            rating: Some(rating),
            text_response: None,
            is_anonymous: false,
            voted_at: now_millis(),
        }
    }

    /// Create a new open-ended vote
    pub fn new_open(
        id: impl Into<String>,
        poll_id: impl Into<String>,
        user_id: impl Into<String>,
        response: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            poll_id: poll_id.into(),
            user_id: user_id.into(),
            selected_options: Vec::new(),
            rating: None,
            text_response: Some(response.into()),
            is_anonymous: false,
            voted_at: now_millis(),
        }
    }

    /// Set anonymous
    pub fn anonymous(mut self) -> Self {
        self.is_anonymous = true;
        self
    }
}

/// Poll results summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PollResults {
    /// Poll ID
    pub poll_id: String,
    /// Total number of votes
    pub total_votes: u32,
    /// Number of unique voters
    pub unique_voters: u32,
    /// Results per option (option_id -> count)
    pub option_counts: HashMap<String, u32>,
    /// Results per option with percentages
    pub option_percentages: HashMap<String, f64>,
    /// For rating polls: average rating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_rating: Option<f64>,
    /// For rating polls: rating distribution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating_distribution: Option<HashMap<u32, u32>>,
    /// For open-ended: sample responses (anonymized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_responses: Option<Vec<String>>,
}

/// Poll filter for listing polls
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PollFilter {
    /// Filter by channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    /// Filter by creator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Filter by active status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    /// Filter by closed status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_closed: Option<bool>,
    /// Filter by tags (must have all specified tags)
    #[serde(default)]
    pub tags: Vec<String>,
    /// Limit results
    #[serde(default)]
    pub limit: usize,
    /// Offset for pagination
    #[serde(default)]
    pub offset: usize,
}

impl PollFilter {
    /// Create a new filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by channel
    pub fn in_channel(mut self, channel_id: impl Into<String>) -> Self {
        self.channel_id = Some(channel_id.into());
        self
    }

    /// Filter by creator
    pub fn created_by(mut self, user_id: impl Into<String>) -> Self {
        self.created_by = Some(user_id.into());
        self
    }

    /// Filter active polls
    pub fn active(mut self) -> Self {
        self.is_active = Some(true);
        self
    }

    /// Filter closed polls
    pub fn closed(mut self) -> Self {
        self.is_closed = Some(true);
        self
    }

    /// Add tag filter
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

/// Get current time in milliseconds since Unix epoch
fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_option_builder() {
        let option = PollOption::new("opt1", "Option 1")
            .with_emoji("👍")
            .with_description("This is option 1");

        assert_eq!(option.id, "opt1");
        assert_eq!(option.text, "Option 1");
        assert_eq!(option.emoji, Some("👍".to_string()));
        assert_eq!(option.description, Some("This is option 1".to_string()));
        assert!(!option.is_correct);
    }

    #[test]
    fn test_poll_option_correct() {
        let option = PollOption::new("opt1", "Option 1").correct();
        assert!(option.is_correct);
    }

    #[test]
    fn test_poll_config_builder() {
        let poll = PollConfig::new("poll1", "What's your favorite color?", "telegram", "user1")
            .with_description("Choose wisely")
            .with_type(PollType::SingleChoice)
            .with_duration(3600)
            .allow_change_vote(true);

        assert_eq!(poll.id, "poll1");
        assert_eq!(poll.title, "What's your favorite color?");
        assert_eq!(poll.description, Some("Choose wisely".to_string()));
        assert_eq!(poll.poll_type, PollType::SingleChoice);
        assert_eq!(poll.duration_seconds, 3600);
        assert!(poll.allow_change_vote);
        assert!(poll.closes_at > poll.created_at);
    }

    #[test]
    fn test_poll_config_add_options() {
        let mut poll = PollConfig::new("poll1", "Favorite color?", "telegram", "user1");
        poll.add_option(PollOption::new("red", "Red"));
        poll.add_option(PollOption::new("blue", "Blue"));

        assert_eq!(poll.options.len(), 2);
        assert_eq!(poll.options[0].id, "red");
        assert_eq!(poll.options[1].id, "blue");
    }

    #[test]
    fn test_poll_validate_empty_title() {
        let poll = PollConfig::new("poll1", "", "telegram", "user1");
        let result = poll.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("title"));
    }

    #[test]
    fn test_poll_validate_not_enough_options() {
        let poll = PollConfig::new("poll1", "Question?", "telegram", "user1");
        // Only 1 option
        let poll = poll.with_options(vec![PollOption::new("a", "A")]);
        let result = poll.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 2 options"));
    }

    #[test]
    fn test_poll_validate_duplicate_option_ids() {
        let poll = PollConfig::new("poll1", "Question?", "telegram", "user1").with_options(vec![
            PollOption::new("a", "Option A"),
            PollOption::new("a", "Option A again"),
        ]);
        let result = poll.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate option ID"));
    }

    #[test]
    fn test_poll_validate_quiz_no_correct_answer() {
        let poll = PollConfig::new("poll1", "Question?", "telegram", "user1")
            .with_type(PollType::Quiz)
            .with_options(vec![PollOption::new("a", "A"), PollOption::new("b", "B")]);
        let result = poll.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("correct answer"));
    }

    #[test]
    fn test_poll_validate_quiz_with_correct_answer() {
        let poll = PollConfig::new("poll1", "Question?", "telegram", "user1")
            .with_type(PollType::Quiz)
            .with_options(vec![
                PollOption::new("a", "A"),
                PollOption::new("b", "B").correct(),
            ]);
        let result = poll.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_poll_validate_rating_range() {
        let poll = PollConfig::new("poll1", "Rate?", "telegram", "user1")
            .with_type(PollType::Rating)
            .with_rating_range(5, 5);
        let result = poll.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("min must be less than"));
    }

    #[test]
    fn test_poll_close_and_reopen() {
        let mut poll = PollConfig::new("poll1", "Question?", "telegram", "user1");
        assert!(poll.is_active);
        assert!(!poll.is_closed);

        poll.close();
        assert!(!poll.is_active);
        assert!(poll.is_closed);

        poll.reopen();
        assert!(poll.is_active);
        assert!(!poll.is_closed);
    }

    #[test]
    fn test_vote_creation() {
        let vote = PollVote::new_choice("vote1", "poll1", "user1", vec!["opt1".to_string()]);
        assert_eq!(vote.poll_id, "poll1");
        assert_eq!(vote.user_id, "user1");
        assert_eq!(vote.selected_options, vec!["opt1"]);
        assert!(!vote.is_anonymous);
    }

    #[test]
    fn test_vote_anonymous() {
        let vote =
            PollVote::new_choice("vote1", "poll1", "user1", vec!["opt1".to_string()]).anonymous();
        assert!(vote.is_anonymous);
    }

    #[test]
    fn test_vote_rating() {
        let vote = PollVote::new_rating("vote1", "poll1", "user1", 4);
        assert_eq!(vote.rating, Some(4));
        assert!(vote.selected_options.is_empty());
    }

    #[test]
    fn test_vote_open_ended() {
        let vote = PollVote::new_open("vote1", "poll1", "user1", "Great poll!");
        assert_eq!(vote.text_response, Some("Great poll!".to_string()));
    }

    #[test]
    fn test_poll_filter_builder() {
        let filter = PollFilter::new()
            .in_channel("telegram")
            .created_by("user1")
            .active()
            .with_tag("fun")
            .limit(10);

        assert_eq!(filter.channel_id, Some("telegram".to_string()));
        assert_eq!(filter.created_by, Some("user1".to_string()));
        assert_eq!(filter.is_active, Some(true));
        assert_eq!(filter.tags, vec!["fun"]);
        assert_eq!(filter.limit, 10);
    }

    #[test]
    fn test_poll_results_default() {
        let results = PollResults {
            poll_id: "poll1".to_string(),
            ..Default::default()
        };
        assert_eq!(results.poll_id, "poll1");
        assert_eq!(results.total_votes, 0);
    }

    #[test]
    fn test_poll_type_serialization() {
        let poll_type = PollType::MultipleChoice;
        let json = serde_json::to_string(&poll_type).unwrap();
        assert!(json.contains("multiple_choice"));

        let parsed: PollType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PollType::MultipleChoice);
    }

    #[test]
    fn test_poll_visibility_serialization() {
        let visibility = PollVisibility::Hidden;
        let json = serde_json::to_string(&visibility).unwrap();
        assert!(json.contains("hidden"));

        let parsed: PollVisibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PollVisibility::Hidden);
    }
}
