# ficdata

Data structures and metadata handling for AO3 fanfiction.

This crate provides core data structures and persistence utilities for working
with Archive of Our Own (AO3) fanfiction metadata. Currently only being worked
on for personal use.

## Features

- **Metadata Management**: Load, save, and update fic metadata with versioning
  support
- **Version Tracking**: Automatic versioning when fics are updated
- **JSON Persistence**: Save and load metadata to/from JSON files
- **Update Detection**: Check if fics have been updated since last download

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ficdata = "0.1"
```

## Usage

### Managing Metadata

```rust
use ficdata::{load_metadata, save_metadata, update_fic_metadata, FicMetadata};

// Load existing metadata
let metadata = load_metadata("./output");
println!("Found {} fics", metadata.len());

// Create new metadata
let new_fic = FicMetadata::new(
    "12345".to_string(),
    "My Fic".to_string(),
    "https://archiveofourown.org/works/12345".to_string(),
    "2024-01-01".to_string()
);

// Save metadata (automatically assigns version number)
update_fic_metadata("./output", new_fic);
```

### Checking for Updates

```rust
use ficdata::{should_download_fic, FicMetadata};

let metadata = FicMetadata::new(
    "12345".to_string(),
    "My Fic".to_string(),
    "https://archiveofourown.org/works/12345".to_string(),
    "2024-01-15".to_string()
);

if should_download_fic("./output", &metadata).unwrap_or(true) {
    println!("Fic has been updated! Download it.");
} else {
    println!("Fic is up-to-date.");
}
```

## Data Structures

### FicMetadata

The main metadata structure containing all information about a fic:

```rust
pub struct FicMetadata {
    pub id: String,              // Fic ID from AO3
    pub name: String,            // Title
    pub url: String,             // Full AO3 URL
    pub tags: TagMap,            // Tags organized by category
    pub last_updated: String,    // Date last updated on AO3
    pub version: u32,            // Version number (increments on updates)
    pub description: String,     // Fic summary/description
    pub authors: Vec<String>,    // List of author usernames
    pub fandom: Vec<String>,     // Fandoms
    pub ship_type: Vec<String>,  // Relationship categories (F/M, M/M, etc.)
    pub language: Option<String>, // Language
    pub chapters: Option<String>, // Chapter info (e.g., "5/10")
    pub kudos: Option<u32>,      // Kudos count
    pub words: Option<u32>,      // Word count
    pub series: Vec<String>,     // Series info
    pub hits: Option<u32>,       // View count
	pub merged_tags: Option<Vec<String>>, // Generated list of all tags
}
```

### TagMap

A type alias for organizing tags:

```rust
pub type TagMap = HashMap<String, Vec<String>>;
```

Maps tag categories (like "freeforms", "warnings", "relationships") to their
values.

## API Overview

### Persistence Functions

- `load_metadata(output_dir: &str) -> Vec<FicMetadata>` - Load metadata from
  JSON
- `save_metadata(output_dir: &str, metadata: &[FicMetadata]) -> Result<(), FicDataError>` -
  Save metadata to JSON
- `update_fic_metadata(output_dir: &str, fic: FicMetadata) -> Result<(), FicDataError>` -
  Add/update a fic's metadata
- `update_existing_metadata_fields(output_dir: &str, new_fic: &FicMetadata) -> Result<(), FicDataError>` -
  Update fields without creating new version

### Version Management

- `should_download_fic(output_dir: &str, fic_metadata: &FicMetadata) -> Result<bool, FicDataError>` -
  Check if fic needs downloading
- `get_next_version(metadatafile: &str, fic_id: &str) -> u32` - Get next version
  number

## Metadata File Format

Metadata is stored in `fics_metadata.json` in the output directory as a JSON
array:

```json
[
	{
		"id": "12345",
		"name": "My Fic Title",
		"url": "https://archiveofourown.org/works/12345",
		"tags": {
			"freeforms": ["Fluff", "Angst"],
			"warnings": ["No Archive Warnings Apply"]
		},
		"last_updated": "2024-01-01",
		"version": 1,
		"description": "A great fic",
		"authors": ["author1"],
		"fandom": ["Fandom Name"]
	}
]
```
