CREATE TABLE IF NOT EXISTS tracks (
    id BIGSERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    thumbnail_id TEXT,
    duration_seconds BIGINT,
    play_count BIGINT NOT NULL DEFAULT 0,
    uploader_id BIGINT NOT NULL,
    upload_status VARCHAR(20) NOT NULL DEFAULT 'audio not uploaded'
);

CREATE TABLE IF NOT EXISTS categories (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

INSERT INTO categories (name) VALUES
    ('Rock'),
    ('Alternative Rock'),
    ('Lo-Fi'),
    ('Spiritual'),
    ('Religious'),
    ('Hip-Hop'),
    ('Rap'),
    ('Country'),
    ('Jazz'),
    ('Disco');

CREATE TABLE IF NOT EXISTS tracks_categories (
    track_id BIGINT REFERENCES tracks (id) ON DELETE CASCADE,
    category_id BIGINT REFERENCES categories (id) ON DELETE CASCADE,
    UNIQUE (track_id, category_id)
);

CREATE TABLE IF NOT EXISTS track_likes (
    user_id BIGINT NOT NULL,
    track_id BIGINT REFERENCES tracks (id) ON DELETE CASCADE,
    UNIQUE (user_id, track_id)
);

CREATE TABLE IF NOT EXISTS track_audio (
    track_id BIGINT UNIQUE REFERENCES tracks(id) ON DELETE CASCADE,
    audio_uri TEXT NOT NULL,
    duration_seconds BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);