CREATE TABLE IF NOT EXISTS vip
(
    status BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS vip_signups
(
    address VARCHAR (40) PRIMARY KEY NOT NULL,
    signed_up_at TIMESTAMP with time zone DEFAULT (now() at time zone 'utc')
);