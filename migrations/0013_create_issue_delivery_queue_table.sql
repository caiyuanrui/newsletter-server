-- Add migration script here
CREATE TABLE issue_delivery_queue (
  newsletter_issue_id BINARY(16) NOT NULL REFERENCES newsletter_issues (newsletter_issue_id),
  subscriber_email VARCHAR(255) NOT NULL,
  PRIMARY KEY (newsletter_issue_id, subscriber_email)
)
