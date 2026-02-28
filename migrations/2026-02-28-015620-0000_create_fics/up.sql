-- Your SQL goes here
CREATE TABLE fics (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    last_updated text NOT NULL,
    version integer NOT NULL,
    description TEXT NOT NULL,
    authors text[] NOT NULL,
    fandom text[] NOT NULL,
    ship_type text[] NOT NULL,
    language text,
    chapters text,
    kudos integer,
    words integer,
    series text[],
    hits integer,
    merged_tags text[]
);