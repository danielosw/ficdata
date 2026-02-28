-- Your SQL goes here
CREATE TABLE fics_tags (
    fic_id TEXT NOT NULL REFERENCES fics(id),
    tag_name TEXT NOT NULL REFERENCES tags(name),
    PRIMARY KEY (fic_id, tag_name)
);