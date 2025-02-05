-- Add migration script here
INSERT INTO
  users (user_id, username, password_hash)
VALUES
  (
    UUID_TO_BIN ('ddf8994f-d522-4659-8d02-c1d479057be6'),
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$xyPp2Y9KgYacGDPfn0t4OQ$pkoPfHMLPDD8M0M7aOz8NhsGLwjzsDojw8L1HACuPfM'
  )
