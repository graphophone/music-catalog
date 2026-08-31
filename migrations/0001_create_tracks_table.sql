CREATE TABLE IF NOT EXISTS tracks (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    thumbnail_url TEXT,
    audio_uri TEXT,
    duration_seconds BIGINT,
    play_count BIGINT NOT NULL DEFAULT 0,
    uploader_id BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS categories (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS tracks_categories (
    track_id BIGINT REFERENCES tracks (id) ON DELETE CASCADE,
    category_id BIGINT REFERENCES categories (id) ON DELETE CASCADE,
    UNIQUE (track_id, category_id)
);

CREATE TABLE IF NOT EXISTS track_likes (
    user_id BIGINT NOT NULL,
    track_id BIGINT REFERENCES tracks (id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    UNIQUE (user_id, track_id)
);