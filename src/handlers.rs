use crate::{db, error, Hub};
use axum::extract::{TypedHeader, WebSocketUpgrade};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

pub struct Connection(db::Connection);

pub async fn health(Connection(connection): Connection) -> crate::Result<StatusCode> {
    db::healthy(&connection).await?;
    Ok(StatusCode::OK)
}

pub async fn websocket(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    Extension(hub): Extension<Arc<Hub>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        tracing::debug!("`{}` connected", user_agent.as_str());
    }
    ws.on_upgrade(move |socket| async move {
        hub.connect(socket).await;
    })
}

// pub mod vip {
//     use crate::handlers::Connection;
//     use crate::models::SignUps;
//     use crate::{db, Hub};
//     use axum::extract::{Extension, Path};
//     use axum::http::StatusCode;
//     use axum::Json;
//     use std::sync::Arc;
//
//     pub async fn check(
//         Path(address): Path<String>,
//         Connection(connection): Connection,
//     ) -> crate::Result<StatusCode> {
//         if db::vip::check(&connection, &address).await? {
//             Ok(StatusCode::OK)
//         } else {
//             Ok(StatusCode::NOT_FOUND)
//         }
//     }
//
//     pub async fn sign_up(
//         Path(address): Path<String>,
//         Connection(connection): Connection,
//         Extension(hub): Extension<Arc<Hub>>,
//     ) -> crate::Result<StatusCode> {
//         let exists = db::vip::check(&connection, &address).await?;
//         if exists {
//             return Ok(StatusCode::CREATED);
//         }
//
//         // Sign up address
//         db::vip::sign_up(&connection, &address).await?;
//
//         // Broadcast updated total to clients
//         let sign_ups = db::vip::total(&connection).await?;
//         if let Err(e) = hub.broadcast(crate::hub::Message::SignedUp {
//             total: sign_ups.total,
//             address: None,
//             last_signed_up: sign_ups.last_signed_up,
//         }) {
//             tracing::error!("an error occurred whilst broadcasting a sign-up {}", e);
//         }
//         Ok(StatusCode::OK)
//     }
//
//     pub async fn total(Connection(connection): Connection) -> crate::Result<Json<SignUps>> {
//         Ok(Json(db::vip::total(&connection).await?))
//     }
// }

// Extract database connection
#[async_trait]
impl<B> FromRequest<B> for Connection
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
        let Extension(pool) = Extension::<db::ConnectionPool>::from_request(req)
            .await
            .map_err(internal_error)?;

        let connection = pool.get_connection().await.map_err(internal_error)?;

        Ok(Self(connection))
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

impl IntoResponse for error::Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
