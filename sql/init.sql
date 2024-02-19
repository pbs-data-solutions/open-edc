SELECT 'CREATE DATABASE open_edc'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'open_edc')\gexec

\connect open_edc
