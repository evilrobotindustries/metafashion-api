use crate::{data::*, error, error::Error::*};
use mobc::Pool;
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::{Config, Error, NoTls};

pub type Connection = mobc::Connection<PgConnectionManager<NoTls>>;
pub type ConnectionPool = mobc::Pool<PgConnectionManager<NoTls>>;
type Result<T> = std::result::Result<T, error::Error>;

const DB_POOL_MAX_OPEN: u64 = 32;
const DB_POOL_MAX_IDLE: u64 = 8;
const DB_POOL_TIMEOUT_SECONDS: u64 = 15;
const INITIALISATION_SCRIPT: &str = "./db.sql";
const CHECK_REGISTRATION_QUERY: &str = "SELECT address FROM vip WHERE address = $1";
const REGISTER_QUERY: &str = "INSERT INTO vip (address) VALUES ($1) RETURNING *";
const TOTAL_REGISTRATIONS_QUERY: &str = "SELECT COUNT(*), MAX(registered_at) FROM vip";

// Initialise the database
pub async fn initialise(connection: &Connection) -> Result<()> {
    let initialisation_script = fs::read_to_string(INITIALISATION_SCRIPT)?;
    connection
        .batch_execute(initialisation_script.as_str())
        .await
        .map_err(DatabaseInitialisationError)?;
    Ok(())
}

pub async fn healthy(connection: &Connection) -> Result<()> {
    connection
        .execute("SELECT 1", &[])
        .await
        .map_err(DatabaseQueryError)?;
    Ok(())
}

// Get connection from the pool
pub async fn get_connection(pool: &ConnectionPool) -> Result<Connection> {
    pool.get().await.map_err(ConnectionPoolError)
}

pub fn create_pool(
    connection_string: &str,
) -> std::result::Result<ConnectionPool, mobc::Error<Error>> {
    let config = Config::from_str(connection_string)?;

    let manager = PgConnectionManager::new(config, NoTls);
    Ok(Pool::builder()
        .max_open(DB_POOL_MAX_OPEN)
        .max_idle(DB_POOL_MAX_IDLE)
        .get_timeout(Some(Duration::from_secs(DB_POOL_TIMEOUT_SECONDS)))
        .build(manager))
}

pub async fn check(connection: &Connection, address: &str) -> Result<bool> {
    let result = connection
        .query_opt(CHECK_REGISTRATION_QUERY, &[&address])
        .await
        .map_err(DatabaseQueryError)?;
    Ok(result.is_some())
}

pub async fn register(connection: &Connection, address: &str) -> Result<Registration> {
    let result = connection
        .query_one(REGISTER_QUERY, &[&address])
        .await
        .map_err(DatabaseQueryError)?;

    Ok(Registration {
        address: result.get(0),
        registered_at: result.get(1),
    })
}

pub async fn total(connection: &Connection) -> Result<Registrations> {
    let result = connection
        .query_one(TOTAL_REGISTRATIONS_QUERY, &[])
        .await
        .map_err(DatabaseQueryError)?;
    Ok(Registrations {
        total: result.get(0),
        last_registered: result.get(1),
    })
}
