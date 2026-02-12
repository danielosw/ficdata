//! Metadata structures and types for AO3 fanfiction

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

/// Type alias for tag mapping: category name -> list of tag values
pub type TagMap = HashMap<String, Vec<String>>;

/// Metadata for a fic that gets saved to JSON
///
/// This structure is used to track information about processed fics.
/// The metadata is saved to `fics_metadata.json` in the output directory
/// and is updated as fics are downloaded.
///
/// # Fields
/// * `id` - The numeric ID of the fic from its URL
/// * `name` - The title of the fic
/// * `url` - The full URL to the fic on Archive of Our Own
/// * `tags` - A map of tag categories to their values
/// * `last_updated` - The date the fic was last updated (as shown on AO3)
/// * `version` - Version number for archiving (increments when fic is updated)
/// * `authors` - List of author usernames for the fic
/// * `fandom` - List of fandoms for the fic
/// * `ship_type` - List of relationship categories (e.g., ["F/M", "Multi"] or ["M/M"])
/// * `language` - The language of the fic (e.g., "English")
/// * `chapters` - Chapter information (e.g., "1/1" or "5/10")
/// * `kudos` - Number of kudos the fic has received
/// * `words` - Number of words in the fic
/// * `series` - List of series the fic belongs to, with part numbers and URLs
/// * `hits` - Number of hits/views the fic has received
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FicMetadata {
    pub id: String,
    pub name: String,
    pub url: String,
    pub tags: TagMap,
    pub last_updated: String,
    pub version: u32,
    pub description: String,
    pub authors: Vec<String>,
    pub fandom: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub ship_type: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub chapters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kudos: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub words: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub series: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hits: Option<u32>,
    #[serde(skip_serializing, default)]
    pub merged_tags: Option<Vec<String>>,
}
fn safe_get_hash(tags: HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
    match tags.get(key) {
        Some(tag_list) => tag_list.clone(),
        None => Vec::new(),
    }
}

impl FicMetadata {
    /// Create a new FicMetadata with required fields (version unset, to be assigned by update_fic_metadata)
    pub fn new(id: String, name: String, url: String, last_updated: String) -> Self {
        Self {
            id,
            name,
            url,
            tags: TagMap::new(),
            last_updated,
            version: 0,
            description: String::new(),
            authors: Vec::new(),
            fandom: Vec::new(),
            ship_type: Vec::new(),
            language: None,
            chapters: None,
            kudos: None,
            words: None,
            series: Vec::new(),
            hits: None,
            merged_tags: None,
        }
    }

    /// Builder method to set tags
    pub fn with_tags(mut self, tags: TagMap) -> Self {
        self.tags = tags;
        self
    }

    /// Builder method to set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Builder method to set authors
    pub fn with_authors(mut self, authors: Vec<String>) -> Self {
        self.authors = authors;
        self
    }

    /// Builder method to set fandom
    pub fn with_fandom(mut self, fandom: Vec<String>) -> Self {
        self.fandom = fandom;
        self
    }

    /// Builder method to set ship_type
    pub fn with_ship_type(mut self, ship_type: Vec<String>) -> Self {
        self.ship_type = ship_type;
        self
    }

    /// Builder method to set language
    pub fn with_language(mut self, language: Option<String>) -> Self {
        self.language = language;
        self
    }

    /// Builder method to set chapters
    pub fn with_chapters(mut self, chapters: Option<String>) -> Self {
        self.chapters = chapters;
        self
    }

    /// Builder method to set kudos
    pub fn with_kudos(mut self, kudos: Option<u32>) -> Self {
        self.kudos = kudos;
        self
    }

    /// Builder method to set words
    pub fn with_words(mut self, words: Option<u32>) -> Self {
        self.words = words;
        self
    }

    /// Builder method to set series
    pub fn with_series(mut self, series: Vec<String>) -> Self {
        self.series = series;
        self
    }

    /// Builder method to set hits
    pub fn with_hits(mut self, hits: Option<u32>) -> Self {
        self.hits = hits;
        self
    }

    /// Check if files exist for this fic in the output directory
    pub fn files_exist(&self, output_dir: &str) -> bool {
        let base_name = if self.version > 1 {
            format!("{}_v{}", self.id, self.version)
        } else {
            self.id.clone()
        };

        let file_types = ["html", "pdf", "epub", "mobi", "azw3"];
        file_types
            .par_iter()
            .any(|ext| fs::metadata(format!("{}/{}.{}", output_dir, base_name, ext)).is_ok())
    }
    pub fn get_tags(&self, r#type: &str) -> Vec<std::string::String> {
        safe_get_hash(self.tags.clone(), r#type)
    }
}
pub fn merge_tag(fic_metadata: &FicMetadata) -> Vec<std::string::String> {
    fic_metadata
        .tags
        .values()
        .flat_map(|v| v.iter().cloned())
        .collect()
}
