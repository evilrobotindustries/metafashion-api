use crate::{db::ConnectionPool, handlers, ws::Clients};
use std::convert::Infallible;
use warp::Filter;

/// GET /health
pub fn health(
    pool: ConnectionPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("health")
        .and(warp::get())
        .and(with_pool(pool))
        .and_then(handlers::health)
}

pub fn websockets(
    clients: Clients,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("subscribe")
        .and(warp::ws())
        .and(with_clients(clients))
        .and_then(handlers::websockets)
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

// Extract database connection pool for any() route
fn with_pool(
    pool: ConnectionPool,
) -> impl Filter<Extract = (ConnectionPool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

pub mod vip {

    use crate::ws::Clients;
    use crate::{db::ConnectionPool, handlers};
    use warp::Filter;

    pub fn all(
        pool: ConnectionPool,
        clients: Clients,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        check(pool.clone())
            .or(register(pool.clone(), clients))
            .or(total(pool))
    }

    /// GET /vip/:address
    pub fn check(
        pool: ConnectionPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("vip" / String)
            .and(warp::get())
            .and(super::with_pool(pool))
            .and_then(handlers::vip::check)
    }

    /// PUT /vip/:address
    pub fn register(
        pool: ConnectionPool,
        clients: Clients,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("vip" / String)
            .and(warp::put())
            .and(super::with_pool(pool))
            .and(super::with_clients(clients))
            .and_then(handlers::vip::register)
    }

    /// GET /vip
    pub fn total(
        pool: ConnectionPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("vip")
            .and(warp::get())
            .and(super::with_pool(pool))
            .and_then(handlers::vip::total)
    }
}
