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
    use crate::models::{SignUp, SignUps, Status};
    use primitive_types::H160;
    use std::str::FromStr;
    use tokio_postgres::Row;

    const CHECK_STATUS_QUERY: &str = "SELECT status FROM vip";
    const CHECK_SIGNUP_QUERY: &str = "SELECT address FROM vip_signups WHERE address = $1";
    const SIGNUP_COMMAND: &str = "INSERT INTO vip_signups (address) VALUES ($1) RETURNING *";
    const TOTAL_SIGNUPS_QUERY: &str = "SELECT COUNT(*), MAX(signed_up_at) FROM vip_signups";

    pub async fn check(connection: &Connection, address: H160) -> crate::Result<bool> {
        let address = format!("{:x}", address);
        let result = connection
            .query_opt(CHECK_SIGNUP_QUERY, &[&address])
            .await
            .map_err(DatabaseQueryError)?;
        Ok(result.is_some())
    }

    pub async fn sign_up(connection: &Connection, address: H160) -> crate::Result<SignUp> {
        if matches!(status(connection).await?, Status::Closed) {
            return Err(crate::error::Error::VIPSignupClosed);
        }

        let address = format!("{:x}", address);
        let result = connection
            .query_one(SIGNUP_COMMAND, &[&address])
            .await
            .map_err(DatabaseQueryError)?;

        Ok(SignUp {
            address: H160::from_str(result.get(0))?,
            signed_up_at: result.get(1),
        })
    }

    pub async fn status(connection: &Connection) -> crate::Result<Status> {
        let result = connection
            .query_opt(CHECK_STATUS_QUERY, &[])
            .await
            .map_err(DatabaseQueryError)?;
        Ok(match result {
            None => Status::Closed,
            Some(result) => {
                let status: bool = result.get(0);
                match status {
                    true => Status::Open,
                    false => Status::Closed,
                }
            }
        })
    }

    pub async fn total(connection: &Connection) -> crate::Result<SignUps> {
        let status = status(connection).await?;
        let result = connection
            .query_one(TOTAL_SIGNUPS_QUERY, &[])
            .await
            .map_err(DatabaseQueryError)?;
        let total: i64 = result.get(0);
        Ok(SignUps {
            total: total as u64,
            last_signed_up: result.get(1),
            status,
        })
    }
}
