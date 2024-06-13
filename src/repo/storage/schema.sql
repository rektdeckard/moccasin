BEGIN;
CREATE TABLE IF NOT EXISTS feeds (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    categories TEXT NOT NULL,
    url TEXT NOT NULL,
    link TEXT NOT NULL,
    ttl TEXT,
    pub_date TEXT,
    last_fetched TEXT
);
CREATE TABLE IF NOT EXISTS items (
    id TEXT NOT NULL PRIMARY KEY,
    feed_id TEXT NOT NULL,
    title TEXT,
    author TEXT,
    content TEXT,
    description TEXT,
    text_description TEXT,
    categories TEXT,
    link TEXT,
    pub_date TEXT,
    FOREIGN KEY(feed_id) REFERENCES feeds(id) ON DELETE CASCADE
);
END;