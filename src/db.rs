use crate::error::Error::*;
use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use std::fs;
use tokio_postgres::Error;
use tokio_postgres_rustls::MakeRustlsConnect;

pub type Connection = PooledConnection<'static, PostgresConnectionManager<MakeRustlsConnect>>;

const INITIALISATION_SCRIPT: &str = "./db.sql";

#[derive(Clone)]
pub struct ConnectionPool(Pool<PostgresConnectionManager<MakeRustlsConnect>>);

impl ConnectionPool {
    pub async fn create(connection_string: String) -> std::result::Result<ConnectionPool, Error> {
        // Initialise root certificate store using Mozilla root certificates
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        // Create tls config, using root cert store
        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(config);

        // Create manager and then finally
        let manager = PostgresConnectionManager::new_from_stringlike(connection_string, tls)
            .expect("could not parse the connection string");
        let pool = Pool::builder()
            .build(manager)
            .await
            .expect("could not build connection pool");
        Ok(ConnectionPool(pool))
    }

    pub async fn get_connection(&self) -> crate::Result<Connection> {
        Ok(self.0.get_owned().await?)
    }
}

// Initialise the database
pub async fn initialise(connection: &Connection) -> crate::Result<()> {
    let initialisation_script = fs::read_to_string(INITIALISATION_SCRIPT)?;
    connection
        .batch_execute(initialisation_script.as_str())
        .await
        .map_err(DatabaseInitialisationError)?;
    Ok(())
}

pub async fn healthy(connection: &Connection) -> crate::Result<()> {
    connection
        .execute("SELECT 1", &[])
        .await
        .map_err(DatabaseQueryError)?;
    Ok(())
}

pub mod vip {
    use crate::db::Connection;
    use crate::error::Error::DatabaseQueryError;
    use crate::models::{Registration, Registrations};

    const CHECK_REGISTRATION_QUERY: &str = "SELECT address FROM vip WHERE address = $1";
    const REGISTER_QUERY: &str = "INSERT INTO vip (address) VALUES ($1) RETURNING *";
    const TOTAL_REGISTRATIONS_QUERY: &str = "SELECT COUNT(*), MAX(registered_at) FROM vip";

    pub async fn check(connection: &Connection, address: &str) -> crate::Result<bool> {
        let result = connection
            .query_opt(CHECK_REGISTRATION_QUERY, &[&address])
            .await
            .map_err(DatabaseQueryError)?;
        Ok(result.is_some())
    }

    pub async fn register(connection: &Connection, address: &str) -> crate::Result<Registration> {
        let result = connection
            .query_one(REGISTER_QUERY, &[&address])
            .await
            .map_err(DatabaseQueryError)?;

        Ok(Registration {
            address: result.get(0),
            registered_at: result.get(1),
        })
    }

    pub async fn total(connection: &Connection) -> crate::Result<Registrations> {
        let result = connection
            .query_one(TOTAL_REGISTRATIONS_QUERY, &[])
            .await
            .map_err(DatabaseQueryError)?;
        Ok(Registrations {
            total: result.get(0),
            last_registered: result.get(1),
        })
    }
}
