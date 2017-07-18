//! # Module to work with `HD Wallets`
//!
//! Currently supports only Ledger Nano S & Ledger Blue
//! `HD(Hierarchical Deterministic) Wallet` specified in
//! [BIP32](https://github.com/bitcoin/bips/blob/master/bip-0032.medёiawiki)

mod error;
mod apdu;
mod hd_keystore;
mod comm;

use self::comm::sendrecv;
use self::apdu::ApduBuilder;
use self::error::Error;
use super::{to_arr, Address, Signature, ECDSA_SIGNATURE_BYTES};
use uuid::Uuid;
use hidapi::{HidDeviceInfo, HidApi, HidDevice};
use std::{thread, time};
use std::str::{FromStr, from_utf8};


const GET_ETH_ADDRESS: u8 = 0x02;
const SIGN_ETH_TRANSACTION: u8 = 0x04;
const CHUNK_SIZE: usize = 255;
const ETC_DERIVATION_PATH: [u8; 21] =  [
    5,
    0x80, 0, 0, 44,
    0x80, 0, 0, 60,
    0x80, 0x02, 0x73, 0xd0,
    0x80, 0, 0, 0,
    0, 0, 0, 0
];  // 44'/60'/160720'/0'/0

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_PID: u16 = 0x0001; // for Nano S model


/// Type used for device listing,
/// String corresponds to file descriptor of the device
pub type DevicesList = Vec<(Address, String)>;

///
#[derive(Debug)]
struct Device {
    ///
    fd: String,
    ///
    address: Address,
    ///
    hid_info: HidDeviceInfo,
}

impl PartialEq for Device {
    fn eq(&self, other: &Device) -> bool {
        self.fd == other.fd
    }
}

impl From<HidDeviceInfo> for Device {
    fn from(hid_info: HidDeviceInfo) -> Self {
        let info = hid_info.clone();
        Device {
            fd: hid_info.path,
            address: Address::default(),
            hid_info: info,
        }
    }
}

///
pub struct WManager {
    /// HID point used for communication
    hid: HidApi,
    /// List of available wallets
    devices: Vec<Device>,
    /// Derivation path
    hd_path: Vec<u8>,
}

impl WManager {
    /// Creates new `Wallet Manager` with a specified
    /// derivation path
    pub fn new(dpath: &[u8]) -> Result<WManager, Error> {
        let mut p: Vec<u8> = Vec::new();
        p.extend_from_slice(dpath);

        Ok(Self {
            hid: HidApi::new()?,
            devices: Vec::new(),
            hd_path: p,
        })
    }

    ///
    pub fn get_address(&self, fd: &str) -> Result<Address, Error> {
        let apdu = ApduBuilder::new(GET_ETH_ADDRESS)
            .with_data(&self.hd_path)
            .build();

        let handle = self.open(fd)?;
        let addr = sendrecv(&handle, &apdu)
            .and_then(|res| { match res.len() {
                    107 => Ok(res),
                    _ => Err(Error::HDWalletError("Address read returned invalid data length".to_string())),
                }
            })
            .and_then(|res: Vec<u8>| from_utf8(&res[67..107])
                .map(|ptr| ptr.to_string())
                .map_err(|e| Error::HDWalletError(format!("Can't parse address: {}", e.to_string()))))
            .and_then(|s| Address::from_str(&s)
                .map_err(|e| Error::HDWalletError(format!("Can't parse address: {}", e.to_string())))
            )?;

        Ok(addr)
    }

    /// Sign hash for transaction
    pub fn sign_transaction(&self, fd: &str, tr: &[u8]) -> Result<Signature, Error> { ;
        let _mock = Vec::new();
        let (init, cont) = match tr.len() {
            0...CHUNK_SIZE => (tr, _mock.as_slice()),
            _ => tr.split_at(CHUNK_SIZE - self.hd_path.len()),
        };

        let init_apdu = ApduBuilder::new(SIGN_ETH_TRANSACTION)
            .with_p1(0x00)
            .with_data(&self.hd_path)
            .with_data(init)
            .build();

        let handle = self.open(fd)?;
        let mut res = sendrecv(&handle, &init_apdu)?;

        for chunk in cont.chunks(CHUNK_SIZE) {
            let apdu_cont = ApduBuilder::new(SIGN_ETH_TRANSACTION)
                .with_p1(0x80)
                .with_data(chunk)
                .build();

            res = sendrecv(&handle, &apdu_cont)?;
        }

        match res.len() {
            ECDSA_SIGNATURE_BYTES => {
                //TODO: upgrade util::to_arr to handle array.len() > 32
                let mut val: [u8; ECDSA_SIGNATURE_BYTES] = [0; ECDSA_SIGNATURE_BYTES];
                val.copy_from_slice(&res);
                Ok(Signature::from(val))
            },
            _ => Err(Error::HDWalletError("Invalid signature length".to_string()))
        }
    }

    ///
    pub fn devices(&self) -> DevicesList {
        self.devices.iter()
            .map(|d| (d.address.clone(), d.fd.clone()))
            .collect()
    }

    /// Update device list
    pub fn update(&mut self) -> Result<(), Error> {
        self.hid.refresh_devices();
        let mut new_devices = Vec::new();

        for hid_info in self.hid.devices() {
            if hid_info.product_id != LEDGER_PID || hid_info.vendor_id != LEDGER_VID  {
                continue;
            }
            let mut d = Device::from(hid_info);
            d.address = self.get_address(&d.fd)?;
            new_devices.push(d);
        }
        self.devices = new_devices;
        println!("Devices found {:?}", self.devices);

        Ok(())
    }

    fn open(&self, path: &str) -> Result<HidDevice, Error> {
        for _ in 0..5 {
            match self.hid.open_path(&path) {
                Ok(h) => return Ok(h),
                Err(_) => (),
            }
            thread::sleep(time::Duration::from_millis(100));
        }

        Err(Error::HDWalletError(format!("Can't open path: {}", path)))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tests::*;
    use rustc_serialize::hex::ToHex;
    use core::Transaction;

    #[test]
    pub fn should_sign_with_ledger() {
        let tx = Transaction {
            nonce: 0x00,
            gas_price: /* 21000000000 */
            to_32bytes("0000000000000000000000000000000\
                                          0000000000000000000000004e3b29200"),
            gas_limit: 21000,
            to: Some("78296F1058dD49C5D6500855F59094F0a2876397"
                .parse::<Address>()
                .unwrap()),
            value: /* 1 ETC */
            to_32bytes("00000000000000000000000000000000\
                                          00000000000000000de0b6b3a7640000"),
            data: Vec::new(),
        };

        /*
            {
                "nonce":"0x00",
                "gasPrice":"0x04e3b29200",
                "gasLimit":"0x5208",
                "to":"0x78296F1058dD49C5D6500855F59094F0a2876397",
                "value":"0x0de0b6b3a7640000",
                "data":"",
                "chainId":61,
                "v":"0x9d",
                "r":"0x5cba84eb9aac6854c8ff6aa21b3e0c6c2036e07ebdee44bcf7ace95bab569d8f",
                "s":"0x6eab3be528ef7565c887e147a2d53340c6c9fab5d6f56694681c90b518b64183"
            }
        */

        // 0xf86d808504e3b292008252089478296f1058dd49c5d6500855f59094f0a2876397880de0b6b3a76400008081
        // 9d
        // a0
        // 5cba84eb9aac6854c8ff6aa21b3e0c6c2036e07ebdee44bcf7ace95bab569d8f
        // a0
        // 6eab3be528ef7565c887e147a2d53340c6c9fab5d6f56694681c90b518b64183
        let rlp = tx.to_rlp().tail;
        let mut manager = WManager::new(&ETC_DERIVATION_PATH).unwrap();
        manager.update().unwrap();

        let fd = &manager.devices()[0].1;


        println!("RLP: {:?}", &rlp.to_hex());
        let sign = manager.sign_transaction(&fd, &rlp).unwrap();
        println!("Signature: {:?}", &sign);

    }

    #[test]
    pub fn should_get_address_with_ledger() {
        let mut manager = WManager::new(&ETC_DERIVATION_PATH).unwrap();
        manager.update().unwrap();

        let fd = &manager.devices()[0].1;
        let addr = manager.get_address(fd).unwrap();

        assert_eq!("78296f1058dd49c5d6500855f59094f0a2876397", addr.to_hex());
    }
}
