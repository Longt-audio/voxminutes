-- Segment translation: realtime translation results persisted alongside the
-- original text so history detail and TXT/SRT/Markdown exports can show them.
-- Empty string = no translation.
ALTER TABLE transcript_segments ADD COLUMN translation TEXT NOT NULL DEFAULT '';
