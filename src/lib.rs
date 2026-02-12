//! # ficdata
//!
//! A library for working with Archive of Our Own (AO3) fanfiction metadata.
//!
//! This library provides data structures and utilities for:
//! - Managing fic metadata with versioning support
//! - Persisting metadata to JSON files
//! - Checking for fic updates
//!
//! For HTML extraction and scraping functionality, see the `ficscrape` crate.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ficdata::{FicMetadata, update_fic_metadata};
//!
//! // Create metadata manually
//! let metadata = FicMetadata::new(
//!     "12345".to_string(),
//!     "My Fic".to_string(),
//!     "https://archiveofourown.org/works/12345".to_string(),
//!     "2024-01-01".to_string()
//! );
//!
//! // Save to metadata file
//! update_fic_metadata("./output", metadata).unwrap();
//! ```

// Module declarations
mod errors;
pub mod metadata;
pub mod persistence;

// Re-export commonly used types and functions
pub use errors::FicDataError;
pub use metadata::{FicMetadata, TagMap, merge_tag};
pub use persistence::{
    get_next_version, load_metadata, save_metadata, should_download_fic,
    update_existing_metadata_fields, update_fic_metadata,
};
