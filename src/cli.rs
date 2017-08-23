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

use clap::{App, Arg};

use hyper::Uri;
use std::str::FromStr;

const NAME: &str = env!("CARGO_PKG_NAME");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

fn validate_uri(v: String) -> Result<(), String> {
    if Uri::from_str(&v).is_ok() {
        return Ok(());
    }
    Err(format!("RPC URL '{}' could not be parsed.", v))
}

pub fn build_cli<'a>() -> App<'a, 'a> {
    App::new(NAME)
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .arg(
            Arg::with_name("uri")
                .short("s")
                .long("server")
                .help("The HTTP URL pointing to the Bitcoin Core RPC server")
                .multiple(false)
                .empty_values(false)
                .value_name("URL")
                .default_value("http://localhost:18332/")
                .validator(validate_uri),
        )
        .arg(
            Arg::with_name("no_conf")
                .short("n")
                .long("no-config")
                .help("Ignore the RPC password from the Bitcoin Core config file")
                .takes_value(false),
        )
}
