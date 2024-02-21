SELECT 'CREATE DATABASE open_edc'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'open_edc')\gexec

\connect open_edc

CREATE TYPE bookstatus AS ENUM('read', 'currently_reading', 'want_to_read')\gexec

CREATE TABLE IF NOT EXISTS organizations(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  active BOOLEAN NOT NULL,
  date_added TIMESTAMP with time zone NOT NULL,
  date_modified TIMESTAMP with time zone NOT NULL
)\gexec

CREATE INDEX ON organizations(name)\gexec
CREATE INDEX ON organizations(active)\gexec

CREATE TABLE IF NOT EXISTS users(
  id TEXT PRIMARY KEY,
  user_name TEXT NOT NULL UNIQUE,
  first_name TEXT NOT NULL,
  last_name TEXT NOT NULL,
  email TEXT NOT NULL,
  hashed_password TEXT NOT NULL,
  organization_id TEXT NOT NULL REFERENCES organizations ON DELETE CASCADE,
  active BOOLEAN NOT NULL,
  date_added TIMESTAMP with time zone NOT NULL,
  date_modified TIMESTAMP with time zone NOT NULL
)\gexec

CREATE INDEX ON users(email)\gexec
CREATE INDEX ON users(active)\gexec
