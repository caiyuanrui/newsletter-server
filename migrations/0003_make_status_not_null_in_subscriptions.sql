-- Add migration script here
START TRANSACTION;

UPDATE subscriptions
SET
  status = 'confirmed'
WHERE
  status IS NULL;

ALTER TABLE subscriptions MODIFY COLUMN status VARCHAR(255) NOT NULL;

COMMIT;
