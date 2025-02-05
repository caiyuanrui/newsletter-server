-- Add migration script here
CREATE TABLE idempotency (
  user_id CHAR(36) NOT NULL,
  idempotency_key VARCHAR(256) NOT NULL,
  response_status_code SMALLINT NOT NULL,
  response_headers BLOB NOT NULL,
  response_body LONGBLOB NOT NULL,
  created_at TIMESTAMP NOT NULL,
  PRIMARY KEY (user_id, idempotency_key),
  FOREIGN KEY (user_id) REFERENCES users (user_id)
);
