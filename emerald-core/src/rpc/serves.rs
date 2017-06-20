use super::Error;
use super::serialize::RPCTransaction;

use addressbook::Addressbook;
use core::Address;
use jsonrpc_core::{self, Params, Value};
use keystore::{KdfDepthLevel, KeyFile};
use std::path::PathBuf;
use std::str::FromStr;

fn to_chain_id(chain: String, chain_id: Option<usize>) -> u8 {
    if chain_id.is_some() {
        chain_id.unwrap() as u8
    } else if chain == "mainnet" {
        61
    } else if chain == "testnet" {
        62
    } else {
        61
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

impl<T, U: Default> Either<T, U> {
    pub fn into_right(self) -> U {
        match self {
            Either::Left(t) => U::default(),
            Either::Right(u) => u,
        }
    }
}

impl<T: Default, U> Either<T, U> {
    pub fn into_left(self) -> T {
        match self {
            Either::Left(t) => t,
            Either::Right(u) => T::default(),
        }
    }
}

impl<T, U: Default> Either<(T,), (T, U)> {
    fn into_full(self) -> (T, U) {
        match self {
            Either::Left((t,)) => (t, U::default()),
            Either::Right((t, u)) => (t, u),
        }
    }
}

pub fn current_version(params: ()) -> Result<&'static str, Error> {
    Ok(::version())
}

pub fn heartbeat(params: ()) -> Result<i64, Error> {
    use time::get_time;
    Ok(get_time().sec)
}

#[derive(Deserialize)]
pub struct ListAccountsAdditional {
    #[serde(default)]
    chain: String,
    #[serde(default)]
    chain_id: Option<usize>,
    #[serde(default)]
    show_hidden: bool,
}

pub fn list_accounts(params: Either<(), (ListAccountsAdditional,)>,
                     keystore_path: &PathBuf)
                     -> Result<Vec<Value>, Error> {
    let address_book = Addressbook::new(keystore_path.clone());
    Ok(address_book.list())
}

#[derive(Deserialize, Default)]
pub struct CommonAdditional {
    #[serde(default)]
    chain: String,
    #[serde(default)]
    chain_id: Option<usize>,
}

#[derive(Deserialize)]
pub struct HideAccountAccount {
    address: String,
}

pub fn hide_account(params: Either<(HideAccountAccount,), (HideAccountAccount, CommonAdditional)>,
                    keystore_path: &PathBuf)
                    -> Result<bool, Error> {
    let (account, additional) = params.into_full();
    unimplemented!();
}

#[derive(Deserialize)]
pub struct UnhideAccountAccount {
    address: String,
}

pub fn unhide_account(params: Either<(UnhideAccountAccount,),
                                     (UnhideAccountAccount, CommonAdditional)>,
                      keystore_path: &PathBuf)
                      -> Result<bool, Error> {
    let (account, additional) = params.into_full();
    unimplemented!();
}

#[derive(Deserialize)]
pub struct ShakeAccountAccount {
    address: String,
    old_passphrase: String,
    new_passphrase: String,
}

pub fn shake_account(params: Either<(ShakeAccountAccount,),
                                    (ShakeAccountAccount, CommonAdditional)>,
                     keystore_path: &PathBuf)
                     -> Result<bool, Error> {
    use keystore::os_random;

    let (account, additional) = params.into_full();
    let addr = Address::from_str(&account.address)?;

    let kf = KeyFile::search_by_address(&addr, keystore_path)?;
    let pk = kf.decrypt_key(&account.old_passphrase)?;
    let new_kf = KeyFile::new_custom(pk,
                                     &account.new_passphrase,
                                     kf.kdf,
                                     &mut os_random(),
                                     kf.name,
                                     kf.description)?;
    new_kf.flush(keystore_path)?;
    Ok(true)
}

#[derive(Deserialize)]
pub struct NewAccountAccount {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    passphrase: String,
}

pub fn new_account(params: Either<(NewAccountAccount,), (NewAccountAccount, CommonAdditional)>,
                   sec: &KdfDepthLevel,
                   keystore_path: &PathBuf)
                   -> Result<String, Error> {
    let (account, additional) = params.into_full();

    if account.passphrase.is_empty() {
        return Err(Error::InvalidDataFormat("Empty passphase".to_string()));
    }

    let kf = KeyFile::new(&account.passphrase,
                          &sec,
                          Some(account.name),
                          Some(account.description))?;
    let addr = kf.address.to_string();
    kf.flush(keystore_path)?;

    Ok(addr)
}

#[derive(Deserialize)]
pub struct SignTransactionTransaction {
    pub from: String,
    pub to: String,
    pub gas: String,
    #[serde(rename="gasPrice")]
    pub gas_price: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub data: String,
    pub nonce: String,
    pub passphrase: String,
}

pub fn sign_transaction(params: Either<(SignTransactionTransaction,),
                                       (SignTransactionTransaction, CommonAdditional)>,
                        keystore_path: &PathBuf)
                        -> Result<Params, Error> {
    let (transaction, additional) = params.into_full();
    let addr = Address::from_str(&transaction.from)?;

    match KeyFile::search_by_address(&addr, keystore_path) {
        Ok(kf) => {
            if let Ok(pk) = kf.decrypt_key(&transaction.passphrase) {
                let transaction = RPCTransaction {
                    from: transaction.from,
                    to: transaction.to,
                    gas: transaction.gas,
                    gas_price: transaction.gas_price,
                    value: transaction.value,
                    data: transaction.data,
                    nonce: transaction.nonce,
                };
                match transaction.try_into() {
                    Ok(tr) => {
                        Ok(tr.to_raw_params(pk, to_chain_id(additional.chain, additional.chain_id)))
                    }
                    Err(err) => Err(Error::InvalidDataFormat(err.to_string())),
                }
            } else {
                Err(Error::InvalidDataFormat("Invalid passphrase".to_string()))
            }
        }
        Err(_) => Err(Error::InvalidDataFormat("Can't find account".to_string())),
    }
}