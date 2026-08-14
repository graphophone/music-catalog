CREATE TABLE IF NOT EXISTS tracks (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    thumbnail_url TEXT,
    audio_uri TEXT,
    duration_seconds BIGINT,
    play_count BIGINT NOT NULL DEFAULT 0,
    user_id BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS categories (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS tracks_categories (
    track_id BIGINT REFERENCES tracks (id) ON DELETE CASCADE,
    category_id BIGINT REFERENCES categories (id) ON DELETE CASCADE
);