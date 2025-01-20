-- Add migration script here
CREATE TABLE subscription_tokens (
  subscription_token VARCHAR(255) NOT NULL,
  subscriber_id CHAR(36) NOT NULL,
  PRIMARY KEY (subscription_token),
  FOREIGN KEY (subscriber_id) REFERENCES subscriptions (id)
);
