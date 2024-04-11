CREATE TABLE IF NOT EXISTS organizations(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  active BOOLEAN NOT NULL,
  date_added TIMESTAMP with time zone NOT NULL,
  date_modified TIMESTAMP with time zone NOT NULL
);

CREATE TABLE IF NOT EXISTS studies(
  id TEXT PRIMARY KEY,
  study_id TEXT NOT NULL UNIQUE,
  study_name TEXT,
  study_description TEXT,
  organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE NOT NULL,
  date_added TIMESTAMP with time zone NOT NULL,
  date_modified TIMESTAMP with time zone NOT NULL
);

CREATE TABLE IF NOT EXISTS users(
  id TEXT PRIMARY KEY,
  user_name TEXT NOT NULL UNIQUE,
  first_name TEXT NOT NULL,
  last_name TEXT NOT NULL,
  email TEXT NOT NULL,
  hashed_password TEXT NOT NULL,
  organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE NOT NULL,
  active BOOLEAN NOT NULL,
  date_added TIMESTAMP with time zone NOT NULL,
  date_modified TIMESTAMP with time zone NOT NULL
);

CREATE INDEX ON users(email);

CREATE TABLE IF NOT EXISTS user_studies(
  id TEXT PRIMARY KEY,
  user_id TEXT REFERENCES users(id) ON DELETE CASCADE NOT NULL,
  study_id TEXT REFERENCES studies(id) ON DELETE CASCADE NOT NULL,
  date_added TIMESTAMP with time zone NOT NULL,
  date_modified TIMESTAMP with time zone NOT NULL,
  UNIQUE(user_id, study_id)
);
