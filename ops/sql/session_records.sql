CREATE TABLE IF NOT EXISTS session_records (
    session_id TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    query TEXT,
    verdict TEXT,
    requires_manual_review BOOLEAN NOT NULL,
    math_status TEXT,
    math_alert_required BOOLEAN NOT NULL,
    math_stdout TEXT,
    math_stderr TEXT,
    trace_path TEXT,
    sandbox_failure_streak INTEGER,
    domain_label TEXT,
    confidence_bucket TEXT,
    consent_provided BOOLEAN,
    math_outputs JSONB,
    PRIMARY KEY (session_id, recorded_at)
);
