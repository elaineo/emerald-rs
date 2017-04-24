//! # Core domain logic module

mod address;
mod error;
mod signature;
mod transaction;

pub use self::address::Address;
pub use self::error::Error;
pub use self::signature::{PrivateKey, ECDSA_SIGNATURE_BYTES, PRIVATE_KEY_BYTES};
pub use self::transaction::{Signature, Transaction};
use super::util;

#[cfg(test)]
mod tests {
    pub use super::*;
    pub use super::tests::*;
}
