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
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![forbid(unsafe_code)]

#[macro_use]
extern crate serde;
extern crate serde_json;

extern crate clap;

extern crate futures;
extern crate hyper;
extern crate tokio_core;

use std::process::exit;
use std::env;
use std::fs::File;
use std::io::{self, stderr, stdin, BufRead, Read, Write};
use hyper::header::Basic;
use tokio_core::reactor::Core;
use hyper::Client;

mod rpc_run;
mod cli;

use rpc_run::execute;
use rpc_run::commands::*;

/// Try to read the RPC password from the Bitcoin Core config.
fn try_get_password_config() -> io::Result<Option<String>> {
    if let Some(mut config_path) = env::home_dir() {
        config_path.push(".bitcoin");
        config_path.push("bitcoin.conf");

        let mut config_file = File::open(config_path)?;
        let mut config = String::new();
        config_file.read_to_string(&mut config)?;

        for line in config.lines() {
            let mut split = line.splitn(2, '=');
            if split.next() == Some("rpcpassword") {
                if let Some(password) = split.next() {
                    return Ok(Some(password.to_owned()));
                }
            }
        }
    }
    Ok(None)
}

/// Call `try_get_password_config`, if it fails get the password from `stdin`.
fn get_password(no_conf: bool) -> io::Result<String> {
    if !no_conf {
        if let Ok(Some(password)) = try_get_password_config() {
            return Ok(password);
        }
    }

    let stdin = stdin();
    let stderr = stderr();
    let mut stdin_lock = stdin.lock();
    let mut stderr_lock = stderr.lock();

    stderr_lock.write_all(b"Input RPC password: ")?;
    stderr_lock.flush()?;

    let mut password = String::new();
    stdin_lock.read_line(&mut password)?;

    password = password.trim().to_owned();

    Ok(password)
}

fn main() {
    if let Err(error) = real_main() {
        match error {
            rpc_run::Error::Http(error) => eprintln!(
                "Fatal error: \
                 HTTP error: '{}'.",
                error
            ),
            rpc_run::Error::Auth => eprintln!(
                "Fatal error: \
                 authentication failure."
            ),
            rpc_run::Error::Json(error) => eprintln!(
                "Fatal error: \
                 json error: '{}'.",
                error
            ),
            rpc_run::Error::Rpc(error) => eprintln!(
                "Fatal error: \
                 RPC error: '{}'.",
                error.message
            ),
        }
        exit(1); // TODO: possibly change this to something more specific.
    }
}

fn real_main() -> Result<(), rpc_run::Error> {
    let mut core = Core::new().expect("Could not initialize tokio core");
    let client = Client::new(&core.handle());

    let matches = cli::build_cli().get_matches();

    let uri = matches.value_of("uri").unwrap().parse().unwrap();

    let no_conf = matches.is_present("no_conf");

    // TODO: figure out how will this handle usernames and multi-wallet.
    let credentials: Basic = Basic {
        username: String::new(),
        password: Some(get_password(no_conf).expect("Failed to get RPC password")),
    };

    // This might fail if the key pool is empty and can not be replenished.
    // TODO: write a better error for this edge case.
    let pay_to_public_key_hash_address =
        execute::<GetNewAddress>(&mut core, &client, &uri, &credentials, &[])?;

    // Make the address SegWit, fixing TXID malleability.
    let pay_to_script_hash_pay_to_witness_public_key_hash_address = execute::<AddWitnessAddress>(
        &mut core,
        &client,
        &uri,
        &credentials,
        &[&pay_to_public_key_hash_address],
    )?;

    // Assert some things about the newly generated address.
    {
        let address_info = execute::<ValidateAddress>(
            &mut core,
            &client,
            &uri,
            &credentials,
            &[&pay_to_script_hash_pay_to_witness_public_key_hash_address],
        )?;

        assert_eq!(address_info.isvalid, true);
        assert_eq!(address_info.ismine, Some(true));
        assert_eq!(address_info.iswatchonly, Some(false));
    }

    println!(
        "{}",
        pay_to_script_hash_pay_to_witness_public_key_hash_address
    );

    Ok(())
}
