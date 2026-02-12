//! Persistence operations for fic metadata - load, save, and version management

use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{errors::FicDataError, metadata::FicMetadata};
use std::{
    fs,
    io::{Read, Write},
};

/// Load fic metadata from JSON file in the output directory
///
/// # Arguments
/// * `output_dir` - Path to the output directory containing fics_metadata.json
///
/// # Returns
/// * Returns a Vec of FicMetadata, or an empty Vec if file doesn't exist
///
/// # Example
/// ```no_run
/// use ficdata::load_metadata;
/// let metadata = load_metadata("./output");
/// ```
pub fn load_metadata(output_dir: &str) -> Vec<FicMetadata> {
    let metadata_path = format!("{}/fics_metadata.json", output_dir);
    let bzip2_path = format!("{}/fics_metadata.json.bz2", output_dir);
    if fs::metadata(&bzip2_path).is_ok() {
        // If we have a bzip2 compressed file, read and decompress it
        let compressed_data = fs::read(&bzip2_path).unwrap_or_else(|_| Vec::new());
        let decompressed_data = bzip2::read::BzDecoder::new(&compressed_data[..]);
        let mut content = String::new();
        std::io::BufReader::new(decompressed_data)
            .read_to_string(&mut content)
            .unwrap_or(0);
        serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
    } else if fs::metadata(&metadata_path).is_ok() {
        let content = fs::read_to_string(&metadata_path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    }
}

/// Save fic metadata to JSON file
///
/// # Arguments
/// * `output_dir` - Path to the output directory
/// * `metadata` - Slice of FicMetadata to save
///
/// # Example
/// ```no_run
/// use ficdata::{save_metadata, FicMetadata};
/// let metadata = vec![FicMetadata::new(
///     "12345".to_string(),
///     "My Fic".to_string(),
///     "https://archiveofourown.org/works/12345".to_string(),
///     "2024-01-01".to_string()
/// )];
/// save_metadata("./output", &metadata).unwrap();
/// ```
pub fn save_metadata(output_dir: &str, metadata: &[FicMetadata]) -> Result<(), FicDataError> {
    let metadata_path = format!("{}/fics_metadata.json", output_dir);
    let bzip2_path = format!("{}/fics_metadata.json.bz2", output_dir);

    let json = serde_json::to_string_pretty(metadata)?;

    fs::write(&metadata_path, json)?;
    // Also write a bzip2 compressed version
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
    encoder.write_all(fs::read(&metadata_path)?.as_slice())?;
    let compressed_data = encoder.finish()?;
    fs::write(&bzip2_path, compressed_data)?;
    Ok(())
}

/// Check if a fic should be downloaded based on its metadata
///
/// Returns true if:
/// - The fic has never been downloaded before, OR
/// - The fic's last_updated date is different from the latest version we have
///
/// This prevents unnecessary re-downloads and saves space.
///
/// # Arguments
/// * `output_dir` - Path to the output directory
/// * `fic_metadata` - Metadata of the fic to check
///
/// # Returns
/// * Returns true if the fic should be downloaded
pub fn should_download_fic(
    output_dir: &str,
    fic_metadata: &FicMetadata,
) -> Result<bool, FicDataError> {
    let existing_metadata = load_metadata(output_dir);

    // Find all versions of this fic
    let existing_versions: Vec<&FicMetadata> = existing_metadata
        .iter()
        .par_bridge()
        .filter(|f| f.id == fic_metadata.id)
        .collect();

    // If we've never downloaded this fic, we should download it
    if existing_versions.is_empty() {
        return Ok(true);
    }

    // Find the latest version
    let latest_version = existing_versions
        .iter()
        .par_bridge()
        .max_by_key(|f| f.version)
        .ok_or(FicDataError::GenericError(
            "Failed to find fic with id of given fic.".to_string(),
        ));

    // Check if the last_updated date has changed
    Ok(latest_version?.last_updated != fic_metadata.last_updated)
}

/// Get the next version number for a fic without saving
///
/// Returns the version number that will be assigned when the fic is saved.
///
/// # Arguments
/// * `metadatafile` - Path to the output directory
/// * `fic_id` - The fic ID to check
///
/// # Returns
/// * Returns the next version number (1 if this is the first version)
pub fn get_next_version(metadatafile: &str, fic_id: &str) -> u32 {
    let metadata = load_metadata(metadatafile);
    // Find the highest version number for this fic ID
    let max_version = metadata
        .iter()
        .par_bridge()
        .filter(|f| f.id == fic_id)
        .map(|f| f.version)
        .max()
        .unwrap_or(0);

    // Return max + 1 (or 1 if this is the first version)
    max_version + 1
}

/// Update metadata for a single fic
///
/// If the fic already exists, increments the version number and adds a new entry.
/// This allows archiving multiple versions of the same fic.
///
/// # Arguments
/// * `output_dir` - Path to the output directory
/// * `fic` - FicMetadata to save (version will be set automatically)
pub fn update_fic_metadata(output_dir: &str, mut fic: FicMetadata) -> Result<(), FicDataError> {
    let mut metadata = load_metadata(output_dir);

    // Find the highest version number for this fic ID
    let max_version = metadata
        .iter()
        .par_bridge()
        .filter(|f| f.id == fic.id)
        .map(|f| f.version)
        .max()
        .unwrap_or(0);

    // Set version to max + 1 (or 1 if this is the first version)
    fic.version = max_version + 1;

    // Add new entry (don't remove old ones - we're archiving)
    metadata.push(fic);

    // Save updated metadata
    save_metadata(output_dir, &metadata)
}

/// Update existing metadata entry with new information without creating a new version
///
/// This function overwrites all fields of existing metadata entries with data from the new
/// metadata, except for the version number which is preserved. This ensures that metadata
/// stays up-to-date with the latest information from AO3 (e.g., updated kudos, hits, tags)
/// without creating a new version.
///
/// **Important**: This function overwrites the `last_updated` field. To ensure version detection
/// works correctly, this function should only be called in these scenarios:
/// 1. AFTER `should_download_fic()` has determined a new version is needed and the download
///    will proceed (typically called after successful download)
/// 2. When `should_download_fic()` has determined NO new version is needed (to update metadata
///    for the current version with latest stats like kudos/hits)
///
/// Never call this BEFORE checking `should_download_fic()`, as it would overwrite the
/// `last_updated` field and break version detection.
///
/// All versions of the fic with matching ID will be updated with the new metadata.
///
/// # Arguments
/// * `output_dir` - Path to the output directory
/// * `new_fic` - Metadata with updated information
pub fn update_existing_metadata_fields(
    output_dir: &str,
    new_fic: &FicMetadata,
) -> Result<(), FicDataError> {
    let mut metadata = load_metadata(output_dir);
    // Find and update all versions of this fic - completely overwrite fields except version
    metadata.iter_mut().par_bridge().for_each(|existing_fic| {
        if existing_fic.id == new_fic.id {
            // Preserve the version number
            let original_version = existing_fic.version;

            // Overwrite all fields with new metadata
            existing_fic.name = new_fic.name.clone();
            existing_fic.url = new_fic.url.clone();
            existing_fic.tags = new_fic.tags.clone();
            existing_fic.last_updated = new_fic.last_updated.clone();
            existing_fic.description = new_fic.description.clone();
            existing_fic.authors = new_fic.authors.clone();
            existing_fic.fandom = new_fic.fandom.clone();
            existing_fic.ship_type = new_fic.ship_type.clone();
            existing_fic.language = new_fic.language.clone();
            existing_fic.chapters = new_fic.chapters.clone();
            existing_fic.kudos = new_fic.kudos;
            existing_fic.words = new_fic.words;
            existing_fic.series = new_fic.series.clone();
            existing_fic.hits = new_fic.hits;

            // Restore the version number
            existing_fic.version = original_version;
        }
    });

    // Save if we made any updates
    save_metadata(output_dir, &metadata)?;
    Ok(())
}
