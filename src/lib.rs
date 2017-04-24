//! # Ethereum classic web3 like connector library

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#![deny(missing_docs)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate chrono;
extern crate crypto;
extern crate futures;
extern crate glob;
extern crate jsonrpc_core;
extern crate jsonrpc_minihttp_server;
extern crate hyper;
extern crate rand;
extern crate regex;
extern crate reqwest;
extern crate rustc_serialize;
extern crate secp256k1;
extern crate uuid;

mod core;
mod keystore;
mod contracts;
mod storage;
mod rpc;
mod util;

pub use self::core::{Address, PrivateKey};
pub use self::keystore::KeyFile;
pub use self::rpc::start;
pub use self::contracts::{ContractError, Contracts};

#[cfg(test)]
mod tests {
    pub use super::*;
    pub use super::util::*;
    pub use super::util::tests::*;
}
