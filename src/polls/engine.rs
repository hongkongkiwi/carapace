//! Poll Engine
//!
//! Handles poll creation, voting, and result calculation.

use super::config::{PollConfig, PollFilter, PollResults, PollType, PollVisibility, PollVote};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Poll engine for managing polls
#[derive(Debug)]
pub struct PollEngine {
    /// Stored polls by ID
    polls: RwLock<HashMap<String, PollConfig>>,
    /// Votes by poll ID (poll_id -> vote_id -> vote)
    votes: RwLock<HashMap<String, HashMap<String, PollVote>>>,
    /// Track which users have voted in each poll (poll_id -> user_id -> vote_id)
    user_votes: RwLock<HashMap<String, HashMap<String, String>>>,
}

impl Default for PollEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PollEngine {
    /// Create a new poll engine
    pub fn new() -> Self {
        Self {
            polls: RwLock::new(HashMap::new()),
            votes: RwLock::new(HashMap::new()),
            user_votes: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new poll
    pub fn create_poll(&self, config: PollConfig) -> Result<PollConfig, String> {
        // Validate the configuration
        config.validate()?;

        // Check for duplicate ID
        let polls = self.polls.read();
        if polls.contains_key(&config.id) {
            return Err(format!("Poll with ID '{}' already exists", config.id));
        }
        drop(polls);

        // Store the poll
        let mut polls = self.polls.write();
        let id = config.id.clone();
        polls.insert(id, config.clone());

        Ok(config)
    }

    /// Get a poll by ID
    pub fn get_poll(&self, poll_id: &str) -> Option<PollConfig> {
        let polls = self.polls.read();
        polls.get(poll_id).cloned()
    }

    /// Update a poll
    pub fn update_poll(
        &self,
        poll_id: &str,
        updater: impl FnOnce(&mut PollConfig),
    ) -> Result<PollConfig, String> {
        let mut polls = self.polls.write();
        if let Some(poll) = polls.get_mut(poll_id) {
            updater(poll);
            // Validate after update
            poll.validate()?;
            Ok(poll.clone())
        } else {
            Err(format!("Poll '{}' not found", poll_id))
        }
    }

    /// Delete a poll and all its votes
    pub fn delete_poll(&self, poll_id: &str) -> bool {
        let mut polls = self.polls.write();
        let mut votes = self.votes.write();
        let mut user_votes = self.user_votes.write();

        let removed = polls.remove(poll_id).is_some();
        if removed {
            votes.remove(poll_id);
            user_votes.remove(poll_id);
        }
        removed
    }

    /// Close a poll
    pub fn close_poll(&self, poll_id: &str) -> Result<PollConfig, String> {
        self.update_poll(poll_id, |poll| poll.close())
    }

    /// Reopen a poll
    pub fn reopen_poll(&self, poll_id: &str) -> Result<PollConfig, String> {
        self.update_poll(poll_id, |poll| poll.reopen())
    }

    /// List polls with optional filter
    pub fn list_polls(&self, filter: Option<&PollFilter>) -> Vec<PollConfig> {
        let polls = self.polls.read();
        let mut results: Vec<PollConfig> = polls.values().cloned().collect();
        drop(polls);

        // Apply filters
        if let Some(filter) = filter {
            if let Some(channel_id) = &filter.channel_id {
                results.retain(|p| &p.channel_id == channel_id);
            }

            if let Some(created_by) = &filter.created_by {
                results.retain(|p| &p.created_by == created_by);
            }

            if let Some(is_active) = filter.is_active {
                results.retain(|p| p.is_active == is_active);
            }

            if let Some(is_closed) = filter.is_closed {
                results.retain(|p| p.is_closed == is_closed);
            }

            if !filter.tags.is_empty() {
                results.retain(|p| filter.tags.iter().all(|tag| p.tags.contains(tag)));
            }

            // Apply offset and limit
            let offset = filter.offset;
            let limit = if filter.limit > 0 {
                filter.limit
            } else {
                results.len()
            };

            results = results.into_iter().skip(offset).take(limit).collect();
        }

        // Sort by creation date (newest first)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        results
    }

    /// Cast a vote
    pub fn cast_vote(&self, vote: PollVote) -> Result<PollVote, String> {
        let poll = self.get_poll(&vote.poll_id).ok_or("Poll not found")?;

        // Check if poll is active
        if !poll.is_active || poll.is_closed {
            return Err("Poll is not active".to_string());
        }

        // Check if expired
        if poll.is_expired() {
            return Err("Poll has expired".to_string());
        }

        // Check if user has already voted
        let mut user_votes = self.user_votes.write();
        let poll_user_votes = user_votes.entry(vote.poll_id.clone()).or_default();

        if let Some(existing_vote_id) = poll_user_votes.get(&vote.user_id) {
            if !poll.allow_change_vote {
                return Err("You have already voted and cannot change your vote".to_string());
            }
            // Remove old vote
            let mut votes = self.votes.write();
            if let Some(poll_votes) = votes.get_mut(&vote.poll_id) {
                poll_votes.remove(existing_vote_id);
            }
        }

        // Validate vote based on poll type
        self.validate_vote(&poll, &vote)?;

        // Store the vote
        let mut votes = self.votes.write();
        let poll_votes = votes.entry(vote.poll_id.clone()).or_default();
        let vote_id = vote.id.clone();
        poll_votes.insert(vote_id.clone(), vote.clone());

        // Track user vote
        poll_user_votes.insert(vote.user_id.clone(), vote_id);

        Ok(vote)
    }

    /// Validate a vote against poll configuration
    fn validate_vote(&self, poll: &PollConfig, vote: &PollVote) -> Result<(), String> {
        match poll.poll_type {
            super::config::PollType::SingleChoice => {
                if vote.selected_options.len() != 1 {
                    return Err("Single choice poll requires exactly one selection".to_string());
                }
                self.validate_options_exist(poll, &vote.selected_options)?;
            }
            super::config::PollType::MultipleChoice => {
                let count = vote.selected_options.len() as u32;
                if poll.min_selections > 0 && count < poll.min_selections {
                    return Err(format!(
                        "Must select at least {} options",
                        poll.min_selections
                    ));
                }
                if poll.max_selections > 0 && count > poll.max_selections {
                    return Err(format!(
                        "Cannot select more than {} options",
                        poll.max_selections
                    ));
                }
                self.validate_options_exist(poll, &vote.selected_options)?;
            }
            super::config::PollType::Rating => {
                let rating = vote.rating.ok_or("Rating is required")?;
                if rating < poll.rating_min || rating > poll.rating_max {
                    return Err(format!(
                        "Rating must be between {} and {}",
                        poll.rating_min, poll.rating_max
                    ));
                }
            }
            super::config::PollType::OpenEnded => {
                if vote.text_response.is_none()
                    || vote.text_response.as_ref().unwrap().trim().is_empty()
                {
                    return Err("Response text is required".to_string());
                }
            }
            super::config::PollType::Quiz => {
                if vote.selected_options.is_empty() {
                    return Err("Must select at least one answer".to_string());
                }
                self.validate_options_exist(poll, &vote.selected_options)?;
            }
        }
        Ok(())
    }

    /// Validate that selected options exist in the poll
    fn validate_options_exist(&self, poll: &PollConfig, options: &[String]) -> Result<(), String> {
        let valid_ids: std::collections::HashSet<_> = poll.options.iter().map(|o| &o.id).collect();
        for option_id in options {
            if !valid_ids.contains(option_id) {
                return Err(format!("Invalid option ID: {}", option_id));
            }
        }
        Ok(())
    }

    /// Get a specific vote
    pub fn get_vote(&self, poll_id: &str, vote_id: &str) -> Option<PollVote> {
        let votes = self.votes.read();
        votes.get(poll_id)?.get(vote_id).cloned()
    }

    /// Get a user's vote in a poll
    pub fn get_user_vote(&self, poll_id: &str, user_id: &str) -> Option<PollVote> {
        let user_votes = self.user_votes.read();
        let vote_id = user_votes.get(poll_id)?.get(user_id)?.clone();
        drop(user_votes);
        self.get_vote(poll_id, &vote_id)
    }

    /// Check if a user has voted
    pub fn has_voted(&self, poll_id: &str, user_id: &str) -> bool {
        let user_votes = self.user_votes.read();
        user_votes
            .get(poll_id)
            .map(|m| m.contains_key(user_id))
            .unwrap_or(false)
    }

    /// Get results for a poll
    pub fn get_results(&self, poll_id: &str) -> Option<PollResults> {
        let poll = self.get_poll(poll_id)?;
        let votes = self.votes.read();
        let poll_votes = votes.get(poll_id);

        let mut results = PollResults {
            poll_id: poll_id.to_string(),
            total_votes: poll_votes.map(|v| v.len() as u32).unwrap_or(0),
            unique_voters: poll_votes.map(|v| v.len() as u32).unwrap_or(0),
            option_counts: HashMap::new(),
            option_percentages: HashMap::new(),
            average_rating: None,
            rating_distribution: None,
            sample_responses: None,
        };

        if let Some(poll_votes) = poll_votes {
            match poll.poll_type {
                super::config::PollType::SingleChoice
                | super::config::PollType::MultipleChoice
                | super::config::PollType::Quiz => {
                    // Count votes per option
                    for vote in poll_votes.values() {
                        for option_id in &vote.selected_options {
                            *results.option_counts.entry(option_id.clone()).or_insert(0) += 1;
                        }
                    }

                    // Calculate percentages
                    if results.total_votes > 0 {
                        for (option_id, count) in &results.option_counts {
                            let percentage = (*count as f64 / results.total_votes as f64) * 100.0;
                            results
                                .option_percentages
                                .insert(option_id.clone(), percentage);
                        }
                    }
                }
                super::config::PollType::Rating => {
                    let mut total_rating: u64 = 0;
                    let mut distribution: HashMap<u32, u32> = HashMap::new();

                    for vote in poll_votes.values() {
                        if let Some(rating) = vote.rating {
                            total_rating += rating as u64;
                            *distribution.entry(rating).or_insert(0) += 1;
                        }
                    }

                    if results.total_votes > 0 {
                        results.average_rating =
                            Some(total_rating as f64 / results.total_votes as f64);
                    }
                    results.rating_distribution = Some(distribution);
                }
                super::config::PollType::OpenEnded => {
                    // Collect sample responses (anonymized)
                    let mut responses: Vec<String> = poll_votes
                        .values()
                        .filter_map(|v| v.text_response.clone())
                        .collect();
                    responses.truncate(100); // Limit to 100 samples
                    results.sample_responses = Some(responses);
                }
            }
        }

        Some(results)
    }

    /// Get results only if poll allows public visibility or is closed
    pub fn get_results_visible(&self, poll_id: &str, requester_id: &str) -> Option<PollResults> {
        let poll = self.get_poll(poll_id)?;

        // Check visibility
        match poll.visibility {
            PollVisibility::Public => {}
            PollVisibility::Hidden => {
                if !poll.is_closed {
                    return None;
                }
            }
            PollVisibility::Private => {
                if poll.created_by != requester_id {
                    return None;
                }
            }
        }

        self.get_results(poll_id)
    }

    /// Get correct answers for a quiz poll
    pub fn get_correct_answers(&self, poll_id: &str, requester_id: &str) -> Option<Vec<String>> {
        let poll = self.get_poll(poll_id)?;

        // Only for quiz polls
        if poll.poll_type != super::config::PollType::Quiz {
            return None;
        }

        // Only creator can see correct answers before poll closes
        if !poll.is_closed && poll.created_by != requester_id {
            return None;
        }

        Some(
            poll.options
                .iter()
                .filter(|o| o.is_correct)
                .map(|o| o.id.clone())
                .collect(),
        )
    }

    /// Revoke a user's vote
    pub fn revoke_vote(&self, poll_id: &str, user_id: &str) -> Result<bool, String> {
        let poll = self.get_poll(poll_id).ok_or("Poll not found")?;

        if !poll.allow_change_vote {
            return Err("Votes cannot be revoked for this poll".to_string());
        }

        let mut user_votes = self.user_votes.write();
        let poll_user_votes = user_votes
            .get_mut(poll_id)
            .ok_or("No votes found for this poll")?;

        let vote_id = poll_user_votes
            .remove(user_id)
            .ok_or("You have not voted in this poll")?;
        drop(user_votes);

        let mut votes = self.votes.write();
        if let Some(poll_votes) = votes.get_mut(poll_id) {
            poll_votes.remove(&vote_id);
        }

        Ok(true)
    }

    /// Delete all votes for a poll (reset)
    pub fn reset_poll(&self, poll_id: &str, requester_id: &str) -> Result<(), String> {
        let poll = self.get_poll(poll_id).ok_or("Poll not found")?;

        if poll.created_by != requester_id {
            return Err("Only the poll creator can reset votes".to_string());
        }

        let mut votes = self.votes.write();
        let mut user_votes = self.user_votes.write();

        votes.remove(poll_id);
        user_votes.remove(poll_id);

        Ok(())
    }

    /// Get poll statistics
    pub fn get_stats(&self) -> PollEngineStats {
        let polls = self.polls.read();
        let votes = self.votes.read();

        let total_polls = polls.len();
        let active_polls = polls
            .values()
            .filter(|p| p.is_active && !p.is_closed)
            .count();
        let closed_polls = polls.values().filter(|p| p.is_closed).count();
        let total_votes: usize = votes.values().map(|v| v.len()).sum();

        PollEngineStats {
            total_polls,
            active_polls,
            closed_polls,
            total_votes,
        }
    }
}

/// Statistics for the poll engine
#[derive(Debug, Clone)]
pub struct PollEngineStats {
    /// Total number of polls
    pub total_polls: usize,
    /// Number of active polls
    pub active_polls: usize,
    /// Number of closed polls
    pub closed_polls: usize,
    /// Total number of votes cast
    pub total_votes: usize,
}

/// Create a shared poll engine
pub fn create_engine() -> Arc<PollEngine> {
    Arc::new(PollEngine::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polls::config::{PollOption, PollType, PollVisibility};

    fn create_test_poll() -> PollConfig {
        PollConfig::new("poll1", "Favorite color?", "telegram", "user1").with_options(vec![
            PollOption::new("red", "Red"),
            PollOption::new("blue", "Blue"),
            PollOption::new("green", "Green"),
        ])
    }

    #[test]
    fn test_create_poll() {
        let engine = PollEngine::new();
        let poll = create_test_poll();

        let result = engine.create_poll(poll);
        assert!(result.is_ok());

        // Duplicate should fail
        let poll2 = create_test_poll();
        let result = engine.create_poll(poll2);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_poll() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        let retrieved = engine.get_poll("poll1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Favorite color?");

        assert!(engine.get_poll("nonexistent").is_none());
    }

    #[test]
    fn test_close_and_reopen_poll() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        let closed = engine.close_poll("poll1").unwrap();
        assert!(closed.is_closed);
        assert!(!closed.is_active);

        let reopened = engine.reopen_poll("poll1").unwrap();
        assert!(!reopened.is_closed);
        assert!(reopened.is_active);
    }

    #[test]
    fn test_delete_poll() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        assert!(engine.delete_poll("poll1"));
        assert!(engine.get_poll("poll1").is_none());
        assert!(!engine.delete_poll("poll1"));
    }

    #[test]
    fn test_cast_vote_single_choice() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        let vote = PollVote::new_choice("vote1", "poll1", "user1", vec!["red".to_string()]);
        let result = engine.cast_vote(vote);
        assert!(result.is_ok());

        // Duplicate vote should fail
        let vote2 = PollVote::new_choice("vote2", "poll1", "user1", vec!["blue".to_string()]);
        let result = engine.cast_vote(vote2);
        assert!(result.is_err());
    }

    #[test]
    fn test_cast_vote_multiple_selections() {
        let engine = PollEngine::new();
        let poll = create_test_poll()
            .with_type(PollType::MultipleChoice)
            .with_selection_limits(1, 2);
        engine.create_poll(poll).unwrap();

        // Valid vote with 2 selections
        let vote = PollVote::new_choice(
            "vote1",
            "poll1",
            "user1",
            vec!["red".to_string(), "blue".to_string()],
        );
        assert!(engine.cast_vote(vote).is_ok());

        // Too many selections should fail
        let vote2 = PollVote::new_choice(
            "vote2",
            "poll1",
            "user2",
            vec!["red".to_string(), "blue".to_string(), "green".to_string()],
        );
        assert!(engine.cast_vote(vote2).is_err());
    }

    #[test]
    fn test_cast_vote_rating() {
        let engine = PollEngine::new();
        let poll = PollConfig::new("poll1", "Rate this", "telegram", "user1")
            .with_type(PollType::Rating)
            .with_rating_range(1, 5);
        engine.create_poll(poll).unwrap();

        let vote = PollVote::new_rating("vote1", "poll1", "user1", 4);
        assert!(engine.cast_vote(vote).is_ok());

        // Out of range should fail
        let vote2 = PollVote::new_rating("vote2", "poll1", "user2", 10);
        assert!(engine.cast_vote(vote2).is_err());
    }

    #[test]
    fn test_cast_vote_open_ended() {
        let engine = PollEngine::new();
        let poll = PollConfig::new("poll1", "Feedback?", "telegram", "user1")
            .with_type(PollType::OpenEnded);
        engine.create_poll(poll).unwrap();

        let vote = PollVote::new_open("vote1", "poll1", "user1", "Great work!");
        assert!(engine.cast_vote(vote).is_ok());

        // Empty response should fail
        let vote2 = PollVote::new_open("vote2", "poll1", "user2", "");
        assert!(engine.cast_vote(vote2).is_err());
    }

    #[test]
    fn test_cast_vote_invalid_option() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        let vote = PollVote::new_choice("vote1", "poll1", "user1", vec!["purple".to_string()]);
        assert!(engine.cast_vote(vote).is_err());
    }

    #[test]
    fn test_get_results() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        // Cast some votes
        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();
        engine
            .cast_vote(PollVote::new_choice(
                "v2",
                "poll1",
                "user2",
                vec!["red".to_string()],
            ))
            .unwrap();
        engine
            .cast_vote(PollVote::new_choice(
                "v3",
                "poll1",
                "user3",
                vec!["blue".to_string()],
            ))
            .unwrap();

        let results = engine.get_results("poll1").unwrap();
        assert_eq!(results.total_votes, 3);
        assert_eq!(results.option_counts.get("red"), Some(&2));
        assert_eq!(results.option_counts.get("blue"), Some(&1));
        assert!(results.option_percentages.contains_key("red"));
    }

    #[test]
    fn test_get_results_rating() {
        let engine = PollEngine::new();
        let poll = PollConfig::new("poll1", "Rate", "telegram", "user1")
            .with_type(PollType::Rating)
            .with_rating_range(1, 5);
        engine.create_poll(poll).unwrap();

        engine
            .cast_vote(PollVote::new_rating("v1", "poll1", "user1", 5))
            .unwrap();
        engine
            .cast_vote(PollVote::new_rating("v2", "poll1", "user2", 3))
            .unwrap();
        engine
            .cast_vote(PollVote::new_rating("v3", "poll1", "user3", 4))
            .unwrap();

        let results = engine.get_results("poll1").unwrap();
        assert_eq!(results.total_votes, 3);
        assert_eq!(results.average_rating, Some(4.0));
        assert!(results.rating_distribution.is_some());
    }

    #[test]
    fn test_has_voted() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        assert!(!engine.has_voted("poll1", "user1"));

        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();

        assert!(engine.has_voted("poll1", "user1"));
        assert!(!engine.has_voted("poll1", "user2"));
    }

    #[test]
    fn test_get_user_vote() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();

        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();

        let vote = engine.get_user_vote("poll1", "user1");
        assert!(vote.is_some());
        assert_eq!(vote.unwrap().selected_options, vec!["red"]);
    }

    #[test]
    fn test_revoke_vote() {
        let engine = PollEngine::new();
        let poll = create_test_poll().allow_change_vote(true);
        engine.create_poll(poll).unwrap();

        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();
        assert!(engine.has_voted("poll1", "user1"));

        assert!(engine.revoke_vote("poll1", "user1").unwrap());
        assert!(!engine.has_voted("poll1", "user1"));
    }

    #[test]
    fn test_revoke_vote_not_allowed() {
        let engine = PollEngine::new();
        let poll = create_test_poll(); // allow_change_vote = false
        engine.create_poll(poll).unwrap();

        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();

        let result = engine.revoke_vote("poll1", "user1");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_polls_with_filter() {
        let engine = PollEngine::new();

        engine
            .create_poll(
                PollConfig::new("p1", "Poll 1", "telegram", "user1")
                    .with_options(vec![PollOption::new("a", "A"), PollOption::new("b", "B")]),
            )
            .unwrap();
        engine
            .create_poll(
                PollConfig::new("p2", "Poll 2", "discord", "user1")
                    .with_options(vec![PollOption::new("a", "A"), PollOption::new("b", "B")]),
            )
            .unwrap();
        engine
            .create_poll(
                PollConfig::new("p3", "Poll 3", "telegram", "user2")
                    .with_options(vec![PollOption::new("a", "A"), PollOption::new("b", "B")]),
            )
            .unwrap();

        // Filter by channel
        let filter = PollFilter::new().in_channel("telegram");
        let results = engine.list_polls(Some(&filter));
        assert_eq!(results.len(), 2);

        // Filter by creator
        let filter = PollFilter::new().created_by("user2");
        let results = engine.list_polls(Some(&filter));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "p3");
    }

    #[test]
    fn test_private_visibility() {
        let engine = PollEngine::new();
        let poll = create_test_poll().with_visibility(PollVisibility::Private);
        engine.create_poll(poll).unwrap();

        // Creator can see results
        assert!(engine.get_results_visible("poll1", "user1").is_some());

        // Others cannot
        assert!(engine.get_results_visible("poll1", "user2").is_none());
    }

    #[test]
    fn test_hidden_visibility() {
        let engine = PollEngine::new();
        let poll = create_test_poll().with_visibility(PollVisibility::Hidden);
        engine.create_poll(poll).unwrap();

        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();

        // Cannot see results while poll is open
        assert!(engine.get_results_visible("poll1", "user1").is_none());

        // Close the poll
        engine.close_poll("poll1").unwrap();

        // Now results are visible
        assert!(engine.get_results_visible("poll1", "user1").is_some());
    }

    #[test]
    fn test_quiz_correct_answers() {
        let engine = PollEngine::new();
        let poll = PollConfig::new("quiz1", "What is 2+2?", "telegram", "user1")
            .with_type(PollType::Quiz)
            .with_options(vec![
                PollOption::new("a", "3"),
                PollOption::new("b", "4").correct(),
                PollOption::new("c", "5"),
            ]);
        engine.create_poll(poll).unwrap();

        // Creator can see correct answers before closing
        let correct = engine.get_correct_answers("quiz1", "user1");
        assert_eq!(correct, Some(vec!["b".to_string()]));

        // Others cannot
        let correct = engine.get_correct_answers("quiz1", "user2");
        assert!(correct.is_none());

        // After closing, others can see
        engine.close_poll("quiz1").unwrap();
        let correct = engine.get_correct_answers("quiz1", "user2");
        assert_eq!(correct, Some(vec!["b".to_string()]));
    }

    #[test]
    fn test_cast_vote_closed_poll() {
        let engine = PollEngine::new();
        let poll = create_test_poll();
        engine.create_poll(poll).unwrap();
        engine.close_poll("poll1").unwrap();

        let vote = PollVote::new_choice("v1", "poll1", "user1", vec!["red".to_string()]);
        assert!(engine.cast_vote(vote).is_err());
    }

    #[test]
    fn test_engine_stats() {
        let engine = PollEngine::new();

        // Empty stats
        let stats = engine.get_stats();
        assert_eq!(stats.total_polls, 0);

        // Create polls
        engine.create_poll(create_test_poll()).unwrap();
        engine
            .create_poll(
                PollConfig::new("p2", "Poll 2", "telegram", "user1")
                    .with_options(vec![PollOption::new("a", "A"), PollOption::new("b", "B")]),
            )
            .unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_polls, 2);
        assert_eq!(stats.active_polls, 2);

        // Add votes
        engine
            .cast_vote(PollVote::new_choice(
                "v1",
                "poll1",
                "user1",
                vec!["red".to_string()],
            ))
            .unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_votes, 1);
    }
}
