//! Browser Page
//!
//! Page manipulation and interaction.

/// Page representation
pub struct Page {
    #[allow(dead_code)]
    url: String,
}

impl Page {
    /// Create new page
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// Get page URL
    pub fn url(&self) -> &str {
        &self.url
    }
}
