#[macro_use]
extern crate serde;
extern crate serde_json;

extern crate futures;
extern crate hyper;
extern crate tokio_core;

use std::io::{stdin, BufRead};
use futures::{Future, Stream};
use hyper::{Body, Chunk, Client, Method, Request, StatusCode, Uri};
use hyper::header::{Authorization, Basic, ContentLength, ContentType};
use tokio_core::reactor::Core;
use hyper::client::HttpConnector;

mod error;

trait BitcoinCommand {
    const COMMAND: &'static str;
    type OutputFormat: for<'de> serde::Deserialize<'de>;
}

#[derive(Debug)]
enum GetNewAddress {}

impl BitcoinCommand for GetNewAddress {
    const COMMAND: &'static str = "getnewaddress";
    type OutputFormat = String;
}

#[derive(Debug)]
enum AddWitnessAddress {}

impl BitcoinCommand for AddWitnessAddress {
    const COMMAND: &'static str = "addwitnessaddress";
    type OutputFormat = String;
}

#[derive(Debug, Clone, Serialize)]
struct RpcInput<'a> {
    jsonrpc: f32,
    id: Option<&'a str>,
    method: &'a str,
    params: &'a [&'a str],
}

#[derive(Debug, Clone, Deserialize)]
struct RpcOutput<T> {
    result: Option<T>,
    error: Option<String>,
    id: Option<String>,
}

fn execute<X: BitcoinCommand>(
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
        |response| if response.status() == StatusCode::Ok {
            Ok(response.body().concat2())
        } else {
            Err(error::Error::Http(hyper::Error::Status))
        },
    );

    let decode_body = core.run(check_status)??.map(|body: Chunk| {
        let x: RpcOutput<X::OutputFormat> = serde_json::from_slice(&body)?;

        if let Some(output) = x.result {
            Ok(output)
        } else {
            Err(error::Error::External(
                x.error
                    .expect("`error` should be present if `result` is not")
                    .to_owned(),
            ))
        }
    });

    let output: error::Result<X::OutputFormat> = core.run(decode_body)?;

    output
}

fn main() {
    let mut core = Core::new().expect("Could not initialize tokio");
    let client = Client::new(&core.handle());

    let uri: Uri = "http://127.0.0.1:18332/".parse().unwrap();

    let stdin = stdin();
    let mut stdin_lock = stdin.lock();
    println!("Input RPC password:");
    let mut password = String::new();
    stdin_lock
        .read_line(&mut password)
        .expect("Failed to read line from STDIN");
    drop(stdin_lock);

    let credentials: Basic = Basic {
        username: String::new(),
        password: Some(password),
    };

    let pay_to_public_key_hash_address =
        execute::<GetNewAddress>(&mut core, &client, &uri, &credentials, &[]).unwrap();
    let segregated_witness_pay_to_script_hash_address = execute::<AddWitnessAddress>(
        &mut core,
        &client,
        &uri,
        &credentials,
        &[&pay_to_public_key_hash_address],
    ).unwrap();

    println!("{}", segregated_witness_pay_to_script_hash_address);
}
