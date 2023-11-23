#![allow(dead_code)]
use std::str::FromStr;

use anyhow::{Error, Result};
use bip39::Mnemonic;
use bitcoin::{secp256k1::Secp256k1, OutPoint, Txid, util::bip32::{ExtendedPrivKey, DerivationPath}, Network};
use serde::{Serialize, Deserialize};
use silentpayments::receiving::Receiver;

use crate::constants::{OwnedOutput, ScanStatus, Status};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    pub sp_wallet: Receiver,
    pub birthday: u32,
    pub scan_status: ScanStatus,
    pub total_amt: u64,
    outputs: Vec<OwnedOutput>
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WalletMessage {
    pub mnemonic: String,
    pub wallet: Wallet,
    pub label: String,
}

impl Wallet {
    fn add_receiver(&mut self, receiver: Receiver) {
        // Check that we don't have a legit receiver here
        self.sp_wallet = receiver;
    }

    pub fn update_scan_height(&mut self, new: u32) {
        self.scan_status.scan_height = new;
    }

    pub fn reset_scan_height(&mut self, scan_height: u32) {
        self.scan_status.scan_height = scan_height;
        let mut to_remove: Vec<_> = vec![];
        for (i, o) in self.outputs.iter().enumerate() {
            if o.blockheight < scan_height.try_into().unwrap() {
                to_remove.push(i);
            }
        }
        for i in to_remove {
            self.outputs.remove(i);
        }
        self.total_amt = self.get_sum_owned();
    }

    pub fn get_scan_height(&self) -> u32 {
        self.scan_status.scan_height
    }

    pub fn get_sum_owned(&self) -> u64 {
        self.outputs.iter()
            .fold(0, |acc, x| acc + x.amount)
    }

    pub fn insert_outpoint(&mut self, owned: OwnedOutput) {
        self.outputs.push(owned);
    }

    pub fn list_outpoints(&self) -> Vec<OwnedOutput> {
        self.outputs.clone()
    }

    pub fn mark_outpoint_spent(&mut self, txoutpoint: OutPoint, txid: Txid) -> Result<()> {
        let output = self.outputs.iter_mut()
            .find(|o| o.txoutpoint == txoutpoint);

        if let Some(to_spend) = output {
            if to_spend.status != Status::Unspent {
                return Err(Error::msg(format!("outpoint {} is already spent", txoutpoint)));
            } else {
                to_spend.status = Status::Spent(txid);
            }
        } else {
            return Err(Error::msg("Unknown outpoint"));
        }

        Ok(())
    }
}

pub fn setup(label: String, words: Option<String>, mut birthday: u32, network: String) -> Result<String> {
    let mut seed = [0u8;64];

    let mut entropy = [0u8;33];
    // Create the empty wallet
    if let Some(m_words) = words {
        let mnemonic = Mnemonic::from_str(&m_words).expect("Invalid mnemonic");
        entropy.copy_from_slice(&mnemonic.to_entropy());
        seed.copy_from_slice(&mnemonic.to_seed(""));
    } else {
        // Create a new wallet
        let mnemonic = Mnemonic::generate(12).expect("Failed to generate mnemonics");
        entropy.copy_from_slice(&mnemonic.to_entropy());
        seed.copy_from_slice(&mnemonic.to_seed(""));
    }

    let secp = Secp256k1::new();
    let network = Network::from_str(&network)?;
    let master_key = ExtendedPrivKey::new_master(network, &seed).unwrap();
    let coin_type_derivation = if network == Network::Bitcoin { "0'" } else { "1'" };

    let scan_key_path = DerivationPath::from_str(&format!("m/352'/{}/0'/1'/0", coin_type_derivation))?;
    let spend_key_path = DerivationPath::from_str(&format!("m/352'/{}/0'/0'/0", coin_type_derivation))?;

    let scan_privkey = master_key.derive_priv(&secp, &scan_key_path)?.private_key;
    let spend_privkey = master_key.derive_priv(&secp, &spend_key_path)?.private_key;

    let receiver = Receiver::new(0, scan_privkey.public_key(&secp), spend_privkey.public_key(&secp), network != Network::Bitcoin)?;

    let wallet = Wallet {
        sp_wallet: receiver,
        birthday,
        scan_status: ScanStatus::default(),
        total_amt: u64::default(),
        outputs: vec![]
    };

    // Serialize it to JSON
    let json = serde_json::to_string(&WalletMessage {
        mnemonic: Mnemonic::from_entropy(&entropy).unwrap().to_string(),
        wallet,
        label
    })?;

    Ok(json)
}

pub fn drop_owned_outpoints() -> Result<()> {

    Ok(())
}

pub fn reset_owned_outputs_from_block_height(height: u32) -> Result<()> {

    Ok(())
}
