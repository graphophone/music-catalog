CREATE TABLE IF NOT EXISTS music (
    id BIGSERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    thumbnail_id TEXT,
    duration_seconds BIGINT,
    play_count BIGINT NOT NULL DEFAULT 0,
    uploader_id BIGINT NOT NULL,
    upload_status VARCHAR(30) NOT NULL DEFAULT 'audio_not_uploaded',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
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

CREATE TABLE IF NOT EXISTS music_categories (
    music_id BIGINT REFERENCES music (id) ON DELETE CASCADE,
    category_id BIGINT REFERENCES categories (id) ON DELETE CASCADE,
    UNIQUE (music_id, category_id)
);

CREATE TABLE IF NOT EXISTS music_likes (
    user_id BIGINT NOT NULL,
    music_id BIGINT REFERENCES music (id) ON DELETE CASCADE,
    UNIQUE (user_id, music_id)
);

CREATE TABLE IF NOT EXISTS music_audio (
    music_id BIGINT UNIQUE REFERENCES music(id) ON DELETE CASCADE,
    audio_uri TEXT NOT NULL,
    duration_seconds BIGINT NOT NULL
);