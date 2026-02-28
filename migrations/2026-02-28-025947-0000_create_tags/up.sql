-- Your SQL goes here
CREATE TABLE tags (
    name TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    parent text,
    sibligs text[],
    children text[]
);