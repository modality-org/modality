-- RING 0: Database schema (kernel-level)
-- Only the kernel agent can modify this file.
-- Changes require human approval.

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  role TEXT DEFAULT 'user',
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  user_id INTEGER REFERENCES users(id),
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE audit_log (
  id SERIAL PRIMARY KEY,
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  target TEXT,
  timestamp TIMESTAMP DEFAULT NOW()
);
