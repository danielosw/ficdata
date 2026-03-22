use crate::{errors::FicDataError, metadata::FicMetadata, metadata::TagMap};
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::OnceLock;

fn work_id_regex() -> &'static Regex {
    static WORK_ID_REGEX: OnceLock<Regex> = OnceLock::new();
    WORK_ID_REGEX.get_or_init(|| Regex::new(r"/works/(\d+)").expect("valid work id regex"))
}

fn date_regex() -> &'static Regex {
    static DATE_REGEX: OnceLock<Regex> = OnceLock::new();
    DATE_REGEX.get_or_init(|| Regex::new(r"\b(\d{4}-\d{2}-\d{2})\b").expect("valid date regex"))
}

fn parse_selector(selector: &str) -> Result<Selector, FicDataError> {
    Selector::parse(selector).map_err(|e| FicDataError::SelectorError(e.to_string()))
}

fn normalize_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn first_text(document: &Html, selector: &str) -> Result<Option<String>, FicDataError> {
    let selector = parse_selector(selector)?;
    Ok(document
        .select(&selector)
        .next()
        .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|s| !s.is_empty()))
}

fn collect_text(document: &Html, selector: &str) -> Result<Vec<String>, FicDataError> {
    let selector = parse_selector(selector)?;
    Ok(document
        .select(&selector)
        .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|s| !s.is_empty())
        .collect())
}

fn parse_number(input: Option<String>) -> Option<u32> {
    input.and_then(|value| value.replace(',', "").parse::<u32>().ok())
}

fn parse_title(document: &Html) -> Result<String, FicDataError> {
    if let Some(title) = first_text(document, "h1.title, h2.title")? {
        return Ok(title);
    }
    if let Some(title) = first_text(document, "title")? {
        return Ok(title
            .split(" - Archive of Our Own")
            .next()
            .unwrap_or("")
            .to_string());
    }
    Err(FicDataError::GenericError(
        "Could not extract title from HTML".to_string(),
    ))
}

fn parse_id_and_url(document: &Html, html: &str) -> Result<(String, String), FicDataError> {
    let selector = parse_selector("a[href*='/works/'], link[href*='/works/']")?;
    let id_regex = work_id_regex();

    let href = document
        .select(&selector)
        .filter_map(|node| node.value().attr("href"))
        .find(|href| id_regex.is_match(href))
        .map(|s| s.to_string())
        .or_else(|| {
            id_regex
                .captures(html)
                .and_then(|captures| captures.get(0).map(|m| m.as_str().to_string()))
        })
        .ok_or_else(|| FicDataError::GenericError("Could not extract fic id from HTML".to_string()))?;

    let id = id_regex
        .captures(&href)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| FicDataError::GenericError("Could not parse fic id from HTML".to_string()))?;

    let url = if href.starts_with("http://") || href.starts_with("https://") {
        href
    } else {
        format!("https://archiveofourown.org/works/{id}")
    };

    Ok((id, url))
}

fn parse_last_updated(document: &Html, html: &str) -> Result<String, FicDataError> {
    let date_regex = date_regex();
    for selector in ["dd.status", "dd.updated", "dd.published", "li.stats dd"] {
        if let Some(value) = first_text(document, selector)? {
            if let Some(captures) = date_regex.captures(&value) {
                if let Some(date) = captures.get(1) {
                    return Ok(date.as_str().to_string());
                }
            }
        }
    }

    if let Some(captures) = date_regex.captures(html) {
        if let Some(date) = captures.get(1) {
            return Ok(date.as_str().to_string());
        }
    }

    Ok(String::new())
}

fn insert_tag_group(document: &Html, tags: &mut TagMap, css_group: &str, output_key: &str) -> Result<(), FicDataError> {
    let selector = format!("dd.{css_group}.tags a.tag, dd.{css_group} a.tag");
    let values = collect_text(document, &selector)?;
    if !values.is_empty() {
        tags.insert(output_key.to_string(), values);
    }
    Ok(())
}

/// Extract fic metadata from a downloaded AO3 HTML file contents.
///
/// This is intended for offline recovery when a fic has been removed from AO3
/// but an HTML download still exists locally.
pub fn extract_metadata_from_downloaded_html(html: &str) -> Result<FicMetadata, FicDataError> {
    let document = Html::parse_document(html);
    let (id, url) = parse_id_and_url(&document, html)?;
    let name = parse_title(&document)?;
    let last_updated = parse_last_updated(&document, html)?;

    let mut tags = TagMap::new();
    insert_tag_group(&document, &mut tags, "rating", "rating")?;
    insert_tag_group(&document, &mut tags, "warning", "warnings")?;
    insert_tag_group(&document, &mut tags, "category", "categories")?;
    insert_tag_group(&document, &mut tags, "fandom", "fandoms")?;
    insert_tag_group(&document, &mut tags, "relationship", "relationships")?;
    insert_tag_group(&document, &mut tags, "character", "characters")?;
    insert_tag_group(&document, &mut tags, "freeform", "freeforms")?;

    let description = first_text(
        &document,
        "blockquote.userstuff.summary, .summary blockquote.userstuff, blockquote.summary",
    )?
    .unwrap_or_default();
    let authors = collect_text(&document, "a[rel='author'], h2.byline a")?;
    let fandom = tags.get("fandoms").cloned().unwrap_or_default();
    let ship_type = tags.get("categories").cloned().unwrap_or_default();
    let language = first_text(&document, "dd.language")?;
    let chapters = first_text(&document, "dd.chapters")?;
    let kudos = parse_number(first_text(&document, "dd.kudos")?);
    let words = parse_number(first_text(&document, "dd.words")?);
    let hits = parse_number(first_text(&document, "dd.hits")?);
    let series = collect_text(&document, "dd.series a, li.series a")?;

    Ok(FicMetadata::new(id, name, url, last_updated)
        .with_tags(tags)
        .with_description(description)
        .with_authors(authors)
        .with_fandom(fandom)
        .with_ship_type(ship_type)
        .with_language(language)
        .with_chapters(chapters)
        .with_kudos(kudos)
        .with_words(words)
        .with_series(series)
        .with_hits(hits))
}

#[cfg(test)]
mod tests {
    use super::extract_metadata_from_downloaded_html;

    #[test]
    fn extracts_metadata_from_ao3_like_downloaded_html() {
        let html = r#"
<!doctype html>
<html>
<head><title>Example Fic - Archive of Our Own</title></head>
<body>
<a href="/works/12345">Work Link</a>
<h1 class="title heading">Example Fic</h1>
<h2 class="byline heading"><a rel="author" href="/users/author">author_name</a></h2>
<blockquote class="userstuff summary">A short summary.</blockquote>
<dl class="meta">
  <dd class="rating tags"><a class="tag">Teen And Up Audiences</a></dd>
  <dd class="warning tags"><a class="tag">No Archive Warnings Apply</a></dd>
  <dd class="category tags"><a class="tag">M/M</a></dd>
  <dd class="fandom tags"><a class="tag">My Fandom</a></dd>
  <dd class="relationship tags"><a class="tag">A/B</a></dd>
  <dd class="character tags"><a class="tag">Character A</a></dd>
  <dd class="freeform tags"><a class="tag">Fluff</a></dd>
  <dd class="language">English</dd>
  <dd class="chapters">3/5</dd>
  <dd class="kudos">1,234</dd>
  <dd class="words">12,345</dd>
  <dd class="hits">9,876</dd>
  <dd class="status">Updated: 2024-05-01</dd>
  <dd class="series"><a href="/series/1">Series One</a></dd>
</dl>
</body>
</html>
        "#;

        let metadata = extract_metadata_from_downloaded_html(html).expect("metadata should parse");
        assert_eq!(metadata.id, "12345");
        assert_eq!(metadata.url, "https://archiveofourown.org/works/12345");
        assert_eq!(metadata.name, "Example Fic");
        assert_eq!(metadata.last_updated, "2024-05-01");
        assert_eq!(metadata.description, "A short summary.");
        assert_eq!(metadata.authors, vec!["author_name"]);
        assert_eq!(metadata.fandom, vec!["My Fandom"]);
        assert_eq!(metadata.ship_type, vec!["M/M"]);
        assert_eq!(metadata.language.as_deref(), Some("English"));
        assert_eq!(metadata.chapters.as_deref(), Some("3/5"));
        assert_eq!(metadata.kudos, Some(1234));
        assert_eq!(metadata.words, Some(12345));
        assert_eq!(metadata.hits, Some(9876));
        assert_eq!(metadata.series, vec!["Series One"]);
        assert_eq!(metadata.tags["freeforms"], vec!["Fluff"]);
        assert_eq!(metadata.tags["relationships"], vec!["A/B"]);
    }

    #[test]
    fn extracts_from_minimal_html_using_fallbacks() {
        let html = r#"
<html>
  <head><title>Fallback Title - Archive of Our Own</title></head>
  <body>
    <a href="https://archiveofourown.org/works/7777">Link</a>
    <p>Published: 2019-01-20</p>
  </body>
</html>
        "#;

        let metadata = extract_metadata_from_downloaded_html(html).expect("metadata should parse");
        assert_eq!(metadata.id, "7777");
        assert_eq!(metadata.url, "https://archiveofourown.org/works/7777");
        assert_eq!(metadata.name, "Fallback Title");
        assert_eq!(metadata.last_updated, "2019-01-20");
    }
}
