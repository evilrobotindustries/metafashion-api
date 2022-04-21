use bb8::RunError;
use rustc_hex::FromHexError;
use serde::Serialize;
use thiserror::Error;

//pub const LOG_TARGET: &str = "api";

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error getting connection from the pool: {0}")]
    ConnectionPoolError(#[from] RunError<tokio_postgres::Error>),
    #[error("Error executing database query: {0}")]
    DatabaseQueryError(#[from] tokio_postgres::Error),
    #[error("Error initialising the database: {0}")]
    DatabaseInitialisationError(tokio_postgres::Error),
    #[error("error reading file: {0}")]
    ReadFileError(#[from] std::io::Error),
    #[error("Error getting connection from the pool: {0}")]
    SerialisationError(#[from] serde_json::Error),
    #[error("The request was unauthorised")]
    UnauthorisedError,
    #[error("error converting from hex: {0}")]
    HashError(#[from] FromHexError),
    #[error("VIP signup closed")]
    VIPSignupClosed,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

//impl warp::reject::Reject for Error {}

// pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
//     let code;
//     let message;
//
//     if err.is_not_found() {
//         code = StatusCode::NOT_FOUND;
//         message = "Not Found";
//     } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
//         code = StatusCode::BAD_REQUEST;
//         message = "Invalid Body";
//     } else if let Some(e) = err.find::<Error>() {
//         match e {
//             Error::DatabaseQueryError(_) => {
//                 code = StatusCode::BAD_REQUEST;
//                 message = "Could not Execute request";
//             }
//             _ => {
//                 eprintln!("unhandled application error: {:?}", err);
//                 code = StatusCode::INTERNAL_SERVER_ERROR;
//                 message = "Internal Server Error";
//             }
//         }
//     } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
//         code = StatusCode::METHOD_NOT_ALLOWED;
//         message = "Method Not Allowed";
//     } else {
//         eprintln!("unhandled error: {:?}", err);
//         //error!("unhandled error: {:?}", err);
//         code = StatusCode::INTERNAL_SERVER_ERROR;
//         message = "Internal Server Error";
//     }
//
//     //error!(target: LOG_TARGET, "{} {}: {:?}", code, message, err);
//
//     let json = warp::reply::json(&ErrorResponse {
//         message: message.into(),
//     });
//
//     Ok(warp::reply::with_status(json, code))
// }
