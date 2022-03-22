use crate::{db, db::ConnectionPool, ws, Result};
use warp::{http::StatusCode, reject, Reply};

pub async fn health(pool: ConnectionPool) -> Result<impl Reply> {
    let connection = db::get_connection(&pool).await?;
    db::healthy(&connection)
        .await
        .map_err(|e| reject::custom(e))?;
    Ok(StatusCode::OK)
}

pub async fn websockets(ws: warp::ws::Ws, clients: ws::Clients) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| super::ws::client_connection(socket, clients)))
}

pub mod vip {
    use crate::ws::Clients;
    use crate::{data::*, db, db::ConnectionPool, Result};
    use warp::{http::StatusCode, reject, reply::json, Reply};

    pub async fn check(address: String, pool: ConnectionPool) -> Result<impl Reply> {
        let connection = db::get_connection(&pool).await?;
        let exists = db::check(&connection, &address)
            .await
            .map_err(|e| reject::custom(e))?;

        Ok(if exists {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        })
    }

    pub async fn register(
        address: String,
        pool: ConnectionPool,
        clients: Clients,
    ) -> Result<impl Reply> {
        let connection = db::get_connection(&pool).await?;

        let exists = db::check(&connection, &address)
            .await
            .map_err(|e| reject::custom(e))?;
        if exists {
            return Ok(StatusCode::CREATED);
        }

        let registration = db::register(&connection, &address)
            .await
            .map_err(|e| reject::custom(e))?;

        // Broadcast updated total to clients
        let registrations = db::total(&connection);
        // clients
        //     .read()
        //     .await
        //     .iter()
        //     .filter(|(_, client)| match body.user_id {
        //         Some(v) => client.user_id == v,
        //         None => true,
        //     })
        //     .filter(|(_, client)| client.topics.contains(&body.topic))
        //     .for_each(|(_, client)| {
        //         if let Some(sender) = &client.sender {
        //             let _ = sender.send(Ok(Message::text(body.message.clone())));
        //         }
        //     });

        Ok(StatusCode::OK)
    }

    pub async fn total(pool: ConnectionPool) -> Result<impl Reply> {
        let connection = db::get_connection(&pool).await?;
        Ok(json(&Registrations::from(db::total(&connection).await?)))
    }
}
