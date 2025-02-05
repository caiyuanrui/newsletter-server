-- Add migration script here
CREATE TABLE users (
  user_id BINARY(16) NOT NULL, -- UUID
  username VARCHAR(100) NOT NULL UNIQUE,
  password VARCHAR(256) NOT NULL,
  PRIMARY KEY (user_id)
);
