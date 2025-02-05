-- Add migration script here
ALTER TABLE idempotency MODIFY COLUMN response_status_code SMALLINT NULL;

ALTER TABLE idempotency MODIFY COLUMN response_headers BLOB NULL;

ALTER TABLE idempotency MODIFY COLUMN response_body LONGBLOB NULL;
