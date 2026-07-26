-- VoxMinutes MVP simplified schema
-- Replaces the legacy multi-table schema with 3 core tables.

-- recordings: a single recording or offline transcription job
CREATE TABLE IF NOT EXISTS recordings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    duration_ms INTEGER DEFAULT 0,
    audio_path TEXT,
    folder_path TEXT,
    source TEXT NOT NULL DEFAULT 'realtime',
    asr_engine TEXT,
    language TEXT,
    status TEXT NOT NULL DEFAULT 'completed'
);

-- transcript_segments: individual transcription segments
CREATE TABLE IF NOT EXISTS transcript_segments (
    id TEXT PRIMARY KEY,
    recording_id TEXT NOT NULL,
    text TEXT NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER,
    speaker TEXT,
    source TEXT NOT NULL DEFAULT 'realtime',
    created_at TEXT NOT NULL,
    FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
);

-- settings: key/value user settings
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recordings_created_at ON recordings(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_transcript_segments_recording_id ON transcript_segments(recording_id);
CREATE INDEX IF NOT EXISTS idx_transcript_segments_start_ms ON transcript_segments(start_ms);
