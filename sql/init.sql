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
