// use crate::{db::ConnectionPool, handlers};
// //use log::Level::Error;
// use std::convert::Infallible;
// use std::env;
// //use warp::{reject, Filter, Rejection};
//
// // async fn handle_request(params: Params) -> Result<impl warp::Reply, warp::Rejection> {
// //     Ok(Response::builder().body(format!("key1 = {}, key2 = {}", params.key1, params.key2)))
// // }
//
// // pub fn auth(value: &str) -> impl Filter<Extract = (), Error = Rejection> + Copy {
// //     warp::filters::header::exact("x-api-key", value)
// // }
//
// // pub fn auth(value: &str) -> impl Filter<Extract = String, Error = Rejection> + Copy {
// //     warp::header::<String>("x-api-key").and_then(|n: String| async move {
// //         if n == "test" {
// //             Ok("".to_string())
// //         } else {
// //             Err(reject::custom(crate::error::Error::NotAuthorized(
// //                 "".to_string(),
// //             )))
// //         }
// //     })
// // }
//
// // pub fn exact(
// //     name: &'static str,
// //     value: &'static str,
// // ) -> impl Filter<Extract = (), Error = Rejection> + Copy {
// //     Ok(())
// //     // filter_fn(move |route| {
// //     //     //tracing::trace!("exact?({:?}, {:?})", name, value);
// //     //     // let route = route
// //     //     //     .headers()
// //     //     //     .get(name)
// //     //     //     .ok_or_else(|| reject::missing_header(name))
// //     //     //     .and_then(|val| {
// //     //     //         if val == value {
// //     //     //             Ok(())
// //     //     //         } else {
// //     //     //             Err(reject::invalid_header(name))
// //     //     //         }
// //     //     //     });
// //     //     // future::ready(route)
// //     // })
// // }
//
// /// GET /health
// pub fn health(
//     pool: ConnectionPool,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     warp::path("health")
//         .and(warp::get())
//         .and(with_pool(pool))
//         .and_then(handlers::health)
// }
//
// pub fn websockets(
//     clients: Clients,
// ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//     warp::path("ws")
//         .and(warp::ws())
//         .and(with_clients(clients))
//         .and_then(handlers::websockets)
// }
//
// fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
//     warp::any().map(move || clients.clone())
// }
//
// // Extract database connection pool for any() route
// fn with_pool(
//     pool: ConnectionPool,
// ) -> impl Filter<Extract = (ConnectionPool,), Error = Infallible> + Clone {
//     warp::any().map(move || pool.clone())
// }
//
// pub mod vip {
//
//     use crate::ws::Clients;
//     use crate::{db::ConnectionPool, handlers};
//     //use warp::Filter;
//
//     pub fn all(
//         api_key: &str,
//         pool: ConnectionPool,
//         clients: Clients,
//     ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//         check(pool.clone(), api_key)
//             .or(register(pool.clone(), clients))
//             .or(total(pool))
//     }
//
//     /// GET /vip/:address
//     pub fn check(
//         pool: ConnectionPool,
//         api_key: &str,
//     ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//         warp::path!("vip" / String)
//             .and(warp::path::end())
//             //.and(warp::filters::header::<String>("API_Key") == "")
//             //.and(super::auth(api_key))
//             // .untuple_one()
//             .and(warp::get())
//             .and(super::with_pool(pool))
//             .and_then(handlers::vip::check)
//     }
//
//     /// PUT /vip/:address
//     pub fn register(
//         pool: ConnectionPool,
//         clients: Clients,
//     ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//         warp::path!("vip" / String)
//             .and(warp::put())
//             .and(super::with_pool(pool))
//             .and(super::with_clients(clients))
//             .and_then(handlers::vip::register)
//     }
//
//     /// GET /vip
//     pub fn total(
//         pool: ConnectionPool,
//     ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
//         warp::path("vip")
//             .and(warp::get())
//             .and(super::with_pool(pool))
//             .and_then(handlers::vip::total)
//     }
// }
