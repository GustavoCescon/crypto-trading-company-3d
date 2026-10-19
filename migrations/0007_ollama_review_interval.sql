ALTER TABLE ai_runtime_state
    DROP CONSTRAINT ai_runtime_state_review_minutes_check;

ALTER TABLE ai_runtime_state
    ADD CONSTRAINT ai_runtime_state_review_minutes_check CHECK(review_minutes BETWEEN 2 AND 60);

ALTER TABLE ai_runtime_state
    ALTER COLUMN review_minutes SET DEFAULT 2;

UPDATE ai_runtime_state
SET review_minutes=2, updated_at=now()
WHERE id=1;
