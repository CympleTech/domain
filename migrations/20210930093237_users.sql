-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
  id          BIGSERIAL PRIMARY KEY,
  name        CHAR(255) NOT NULL,
  pid         CHAR(64) NOT NULL,
  bio         TEXT NOT NULL,
  is_actived  BOOLEAN NOT NULL DEFAULT TRUE,
  datetime    BIGINT  NOT NULL,
  is_deleted  BOOLEAN NOT NULL DEFAULT FALSE
);
