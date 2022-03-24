use crate::hub::Hub;
use axum::{extract::Extension, routing::get, Router};
use std::{
    env,
    {net::SocketAddr, sync::Arc},
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod error;
mod filters;
mod handlers;
mod hub;
mod models;

type Result<T> = std::result::Result<T, error::Error>;

const CONNECTION_STRING: &str = "CONNECTION_STRING";
const API_KEY: &str = "API_KEY";

#[tokio::main]
async fn main() {
    // Validate required configuration (todo: use config crate)
    let connection_string = env::var(CONNECTION_STRING);
    if connection_string.is_err() {
        panic!("{} not set", CONNECTION_STRING)
    }
    let api_key = env::var(API_KEY);
    if api_key.is_err() {
        panic!("{} not set", API_KEY)
    }

    // Initialise logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create database connection pool
    let pool = db::ConnectionPool::create(connection_string.unwrap())
        .await
        .expect("database connection pool cannot be created.");

    // Initialise database
    let connection = pool
        .get_connection()
        .await
        .expect("could not get connection to database");
    db::initialise(&connection)
        .await
        .expect("database can't be initialized");

    // Create websocket hub
    let hub = Arc::new(Hub::init(pool.clone(), api_key.unwrap()));

    // build our application with some routes
    let app = Router::new()
        // Routes
        .route("/health", get(handlers::health))
        // .route("/vip", get(handlers::vip::total))
        // .route(
        //     "/vip/:address",
        //     get(handlers::vip::check).put(handlers::vip::register),
        // )
        .route("/ws", get(handlers::websocket))
        // Middleware
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(pool)) // Connection pool
        .layer(Extension(hub));

    // Finally start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
