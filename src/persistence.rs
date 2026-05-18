//! Persistence operations for fic metadata - load, save, and version management

use diesel::{
    Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection,
    connection::SimpleConnection, prelude::*,
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::de::DeserializeOwned;

use crate::{errors::FicDataError, metadata::FicMetadata};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const METADATA_SQLITE_FILENAME: &str = "fics_metadata.sqlite";
const METADATA_JSON_FILENAME: &str = "fics_metadata.json";
const METADATA_JSON_BZ2_FILENAME: &str = "fics_metadata.json.bz2";

diesel::table! {
    fics_metadata (id, version) {
        id -> Text,
        version -> Integer,
        name -> Text,
        url -> Text,
        tags_json -> Text,
        last_updated -> Text,
        description -> Text,
        authors_json -> Text,
        fandom_json -> Text,
        ship_type_json -> Text,
        language -> Nullable<Text>,
        chapters -> Nullable<Text>,
        kudos -> Nullable<Integer>,
        words -> Nullable<Integer>,
        series_json -> Text,
        hits -> Nullable<Integer>,
    }
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = fics_metadata)]
struct FicMetadataRow {
    id: String,
    version: i32,
    name: String,
    url: String,
    tags_json: String,
    last_updated: String,
    description: String,
    authors_json: String,
    fandom_json: String,
    ship_type_json: String,
    language: Option<String>,
    chapters: Option<String>,
    kudos: Option<i32>,
    words: Option<i32>,
    series_json: String,
    hits: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name = fics_metadata)]
struct NewFicMetadataRow {
    id: String,
    version: i32,
    name: String,
    url: String,
    tags_json: String,
    last_updated: String,
    description: String,
    authors_json: String,
    fandom_json: String,
    ship_type_json: String,
    language: Option<String>,
    chapters: Option<String>,
    kudos: Option<i32>,
    words: Option<i32>,
    series_json: String,
    hits: Option<i32>,
}

impl TryFrom<FicMetadataRow> for FicMetadata {
    type Error = FicDataError;

    fn try_from(value: FicMetadataRow) -> Result<Self, Self::Error> {
        let version = u32::try_from(value.version).map_err(|_| {
            FicDataError::GenericError(format!(
                "metadata version out of range for fic {}",
                value.id
            ))
        })?;
        Ok(FicMetadata {
            id: value.id,
            name: value.name,
            url: value.url,
            tags: parse_json_field(&value.tags_json, "tags_json")?,
            last_updated: value.last_updated,
            version,
            description: value.description,
            authors: parse_json_field(&value.authors_json, "authors_json")?,
            fandom: parse_json_field(&value.fandom_json, "fandom_json")?,
            ship_type: parse_json_field(&value.ship_type_json, "ship_type_json")?,
            language: value.language,
            chapters: value.chapters,
            kudos: to_u32_opt(value.kudos, "kudos")?,
            words: to_u32_opt(value.words, "words")?,
            series: parse_json_field(&value.series_json, "series_json")?,
            hits: to_u32_opt(value.hits, "hits")?,
            merged_tags: None,
        })
    }
}

/// Load fic metadata from SQLite in the output directory.
///
/// If no SQLite metadata exists yet, this automatically imports legacy metadata
/// from `fics_metadata.json.bz2` or `fics_metadata.json` (if available) and
/// persists it into SQLite.
///
/// # Arguments
/// * `output_dir` - Path to the output directory containing metadata files
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
    let sqlite_path = metadata_sqlite_path(output_dir);
    if fs::metadata(&sqlite_path).is_ok() {
        return load_from_sqlite(output_dir)
            .or_else(|_| {
                load_legacy_metadata(output_dir).ok_or_else(|| {
                    FicDataError::GenericError(
                        "failed to load metadata from sqlite and legacy files".to_string(),
                    )
                })
            })
            .unwrap_or_default();
    }

    if let Some(legacy_metadata) = load_legacy_metadata(output_dir) {
        let _ = save_to_sqlite(output_dir, &legacy_metadata);
        return legacy_metadata;
    }

    Vec::new()
}

/// Save fic metadata to SQLite, plus legacy JSON files for compatibility.
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
    save_to_sqlite(output_dir, metadata)?;
    save_legacy_exports(output_dir, metadata)
}

fn load_legacy_metadata(output_dir: &str) -> Option<Vec<FicMetadata>> {
    let metadata_path = output_file_path(output_dir, METADATA_JSON_FILENAME);
    let bzip2_path = output_file_path(output_dir, METADATA_JSON_BZ2_FILENAME);
    load_from_bzip2(&bzip2_path).or_else(|| load_from_json(&metadata_path))
}

fn save_legacy_exports(output_dir: &str, metadata: &[FicMetadata]) -> Result<(), FicDataError> {
    let metadata_path = output_file_path(output_dir, METADATA_JSON_FILENAME);
    let bzip2_path = output_file_path(output_dir, METADATA_JSON_BZ2_FILENAME);
    let json = serde_json::to_string_pretty(metadata)?;
    let json_bytes = json.as_bytes();

    write_atomic(&metadata_path, json_bytes)?;
    // Also write a bzip2 compressed version
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
    encoder.write_all(json_bytes)?;
    let compressed_data = encoder.finish()?;
    write_atomic(&bzip2_path, compressed_data.as_slice())?;
    Ok(())
}

fn metadata_sqlite_path(output_dir: &str) -> String {
    output_file_path(output_dir, METADATA_SQLITE_FILENAME)
        .to_string_lossy()
        .into_owned()
}

fn output_file_path(output_dir: &str, file_name: &str) -> PathBuf {
    Path::new(output_dir).join(file_name)
}

fn connect_metadata_db(output_dir: &str) -> Result<SqliteConnection, FicDataError> {
    let mut conn = SqliteConnection::establish(&metadata_sqlite_path(output_dir))?;
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS fics_metadata (
            id TEXT NOT NULL,
            version INTEGER NOT NULL,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            last_updated TEXT NOT NULL,
            description TEXT NOT NULL,
            authors_json TEXT NOT NULL,
            fandom_json TEXT NOT NULL,
            ship_type_json TEXT NOT NULL,
            language TEXT NULL,
            chapters TEXT NULL,
            kudos INTEGER NULL,
            words INTEGER NULL,
            series_json TEXT NOT NULL,
            hits INTEGER NULL,
            PRIMARY KEY (id, version)
        );",
    )?;
    Ok(conn)
}

fn parse_json_field<T: DeserializeOwned>(raw: &str, field_name: &str) -> Result<T, FicDataError> {
    serde_json::from_str(raw).map_err(|err| {
        FicDataError::GenericError(format!("failed to parse {field_name} JSON field: {err}"))
    })
}

fn to_u32_opt(value: Option<i32>, field_name: &str) -> Result<Option<u32>, FicDataError> {
    value
        .map(|value| {
            u32::try_from(value).map_err(|_| {
                FicDataError::GenericError(format!("{field_name} value out of range: {value}"))
            })
        })
        .transpose()
}

fn to_i32_opt(value: Option<u32>, field_name: &str) -> Result<Option<i32>, FicDataError> {
    value
        .map(|value| {
            i32::try_from(value).map_err(|_| {
                FicDataError::GenericError(format!("{field_name} value out of range: {value}"))
            })
        })
        .transpose()
}

fn to_row(fic: &FicMetadata) -> Result<NewFicMetadataRow, FicDataError> {
    Ok(NewFicMetadataRow {
        id: fic.id.clone(),
        version: i32::try_from(fic.version).map_err(|_| {
            FicDataError::GenericError(format!("version value out of range: {}", fic.version))
        })?,
        name: fic.name.clone(),
        url: fic.url.clone(),
        tags_json: serde_json::to_string(&fic.tags)?,
        last_updated: fic.last_updated.clone(),
        description: fic.description.clone(),
        authors_json: serde_json::to_string(&fic.authors)?,
        fandom_json: serde_json::to_string(&fic.fandom)?,
        ship_type_json: serde_json::to_string(&fic.ship_type)?,
        language: fic.language.clone(),
        chapters: fic.chapters.clone(),
        kudos: to_i32_opt(fic.kudos, "kudos")?,
        words: to_i32_opt(fic.words, "words")?,
        series_json: serde_json::to_string(&fic.series)?,
        hits: to_i32_opt(fic.hits, "hits")?,
    })
}

fn load_from_sqlite(output_dir: &str) -> Result<Vec<FicMetadata>, FicDataError> {
    use self::fics_metadata::dsl::*;

    let mut conn = connect_metadata_db(output_dir)?;
    let rows = fics_metadata
        .order((id.asc(), version.asc()))
        .select(FicMetadataRow::as_select())
        .load::<FicMetadataRow>(&mut conn)?;

    rows.into_iter().map(FicMetadata::try_from).collect()
}

fn save_to_sqlite(output_dir: &str, metadata: &[FicMetadata]) -> Result<(), FicDataError> {
    use self::fics_metadata::dsl::*;

    let mut conn = connect_metadata_db(output_dir)?;
    conn.transaction(|conn| {
        diesel::delete(fics_metadata).execute(conn)?;

        if !metadata.is_empty() {
            let rows: Vec<NewFicMetadataRow> = metadata
                .iter()
                .map(to_row)
                .collect::<Result<Vec<_>, FicDataError>>()?;
            diesel::insert_into(fics_metadata)
                .values(&rows)
                .execute(conn)?;
        }
        Ok(())
    })
}

fn load_from_bzip2(path: &Path) -> Option<Vec<FicMetadata>> {
    let compressed_data = fs::read(path).ok()?;
    let decompressed_data = bzip2::read::BzDecoder::new(&compressed_data[..]);
    let mut content = String::new();
    std::io::BufReader::new(decompressed_data)
        .read_to_string(&mut content)
        .ok()?;
    serde_json::from_str(&content).ok()
}

fn load_from_json(path: &Path) -> Option<Vec<FicMetadata>> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<(), FicDataError> {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path.file_name().ok_or_else(|| {
        FicDataError::GenericError(format!("invalid file path for atomic write: {path:?}"))
    })?;
    let tmp_path = path.with_file_name(format!("{}.tmp.{suffix}", file_name.to_string_lossy()));

    fs::write(&tmp_path, data)?;
    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err.into());
    }
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
/// * `output_dir` - Path to the output directory
/// * `fic_id` - The fic ID to check
///
/// # Returns
/// * Returns the next version number (1 if this is the first version)
pub fn get_next_version(output_dir: &str, fic_id: &str) -> u32 {
    let metadata = load_metadata(output_dir);
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
