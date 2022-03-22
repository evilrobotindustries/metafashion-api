use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::Filter;

#[macro_use]
extern crate log;

mod data;
mod db;
mod error;
mod filters;
mod handlers;
mod ws;

type Result<T> = std::result::Result<T, warp::Rejection>;

const CONNECTION_STRING: &str = "CONNECTION_STRING";

#[tokio::main]
async fn main() {
    if env::var_os(CONNECTION_STRING).is_none() {
        env::set_var(
            CONNECTION_STRING,
            "postgres://postgres@127.0.0.1:7878/postgres",
        );
    }

    // Initialise logging
    pretty_env_logger::init();

    // Create database connection pool and then initialise
    let pool = db::create_pool(&env::var(CONNECTION_STRING).expect(&format!(
        "{} environment variable not set",
        CONNECTION_STRING
    )))
    .expect("database connection pool cannot be created.");
    let connection = db::get_connection(&pool)
        .await
        .expect("could not get connection to database");
    db::initialise(&connection)
        .await
        .expect("database can't be initialized");

    // Create map for web socket clients
    let clients: ws::Clients = Arc::new(Mutex::new(HashMap::new()));

    // Create routes
    let routes = filters::health(pool.clone())
        .or(filters::vip::all(pool, clients.clone()))
        .or(filters::websockets(clients))
        .with(warp::log("api"))
        .with(warp::cors().allow_any_origin())
        .recover(error::handle_rejection);

    info!("Starting api...");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
