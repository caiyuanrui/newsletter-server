mod data;
mod server;

use std::fmt::{Debug, Display};

use axum::response::{IntoResponse, Response};
pub use data::Data;
use hyper::{header, StatusCode};
pub use server::Server;

pub fn e500<T>(e: T) -> Response
where
    T: Display + Debug + 'static,
{
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

pub fn see_other(location: &str) -> Response {
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}
