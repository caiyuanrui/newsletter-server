-- Add migration script here
CREATE INDEX status USING HASH ON subscriptions (status);
