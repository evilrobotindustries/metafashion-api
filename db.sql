CREATE TABLE IF NOT EXISTS vip
(
    address VARCHAR (42) PRIMARY KEY NOT NULL,
    registered_at timestamp with time zone DEFAULT (now() at time zone 'utc')
);