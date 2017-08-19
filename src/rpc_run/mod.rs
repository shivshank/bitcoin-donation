// bitcoin-donation - Generate a Bitcoin address for donations.
// Copyright (C) 2017 Cooper Paul EdenDay
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use {hyper, serde, serde_json};
use futures::{Future, Stream};
use hyper::{Body, Chunk, Client, Method, Request, StatusCode, Uri};
use hyper::header::{Authorization, Basic, ContentLength, ContentType};
use tokio_core::reactor::Core;
use hyper::client::HttpConnector;

pub mod error;
pub mod commands;

pub trait BitcoinCommand {
    const COMMAND: &'static str;
    type OutputFormat: for<'de> serde::Deserialize<'de>;
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcInput<'a> {
    jsonrpc: f32,
    id: Option<&'a str>,
    method: &'a str,
    params: &'a [&'a str],
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcError {
    code: u32,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcOutput<T> {
    result: Option<T>,
    error: Option<RpcError>,
    id: Option<String>,
}

pub fn execute<X: BitcoinCommand>(
    core: &mut Core,
    client: &Client<HttpConnector, Body>,
    server: &Uri,
    credentials: &Basic,
    params: &[&str],
) -> error::Result<X::OutputFormat> {
    let mut request = Request::new(Method::Post, server.clone());
    request.headers_mut().set(ContentType::json());

    request
        .headers_mut()
        .set(Authorization(credentials.clone()));

    let input = RpcInput {
        jsonrpc: 2.0,
        id: None,
        method: X::COMMAND,
        params,
    };

    let encoded_input = serde_json::to_vec(&input)?;

    request
        .headers_mut()
        .set(ContentLength(encoded_input.len() as u64));
    request.set_body(encoded_input);

    let check_status = client.request(request).map(
        |response| match response.status() {
            StatusCode::Ok => Ok(response.body().concat2()),
            StatusCode::Unauthorized => Err(error::Error::Auth),
            _ => Err(error::Error::Http(hyper::Error::Status)),
        },
    );

    let decode_body = core.run(check_status)??.map(|body: Chunk| {
        let x: RpcOutput<X::OutputFormat> = serde_json::from_slice(&body)?;

        if let Some(output) = x.result {
            Ok(output)
        } else {
            Err(error::Error::Rpc(
                x.error
                    .expect("`error` should be present if `result` is not"),
            ))
        }
    });

    let output: error::Result<X::OutputFormat> = core.run(decode_body)?;

    output
}