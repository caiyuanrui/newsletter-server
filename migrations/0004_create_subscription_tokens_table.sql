-- Add migration script here
CREATE TABLE subscription_tokens (
  subscription_token VARCHAR(255) NOT NULL,
  subscriber_id BINARY(16) NOT NULL, -- UUID
  PRIMARY KEY (subscription_token),
  FOREIGN KEY (subscriber_id) REFERENCES subscriptions (id)
);
