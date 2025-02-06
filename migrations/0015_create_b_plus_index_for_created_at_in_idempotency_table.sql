-- Add migration script here
CREATE INDEX created_at ON idempotency (created_at);
