use std::{
    fmt::{
        self,
        Display,
        Formatter,
        Write,
    },
    fs,
    io,
    num::NonZeroUsize,
    path::Path,
};

use astria_core::{
    brotli::decompress_bytes,
    generated::sequencerblock::v1alpha1::{
        rollup_data::Value as RawRollupDataValue,
        Deposit as RawDeposit,
        RollupData as RawRollupData,
        SubmittedMetadata as RawSubmittedMetadata,
        SubmittedMetadataList as RawSubmittedMetadataList,
        SubmittedRollupData as RawSubmittedRollupData,
        SubmittedRollupDataList as RawSubmittedRollupDataList,
    },
    primitive::v1::RollupId,
    sequencerblock::v1alpha1::{
        block::{
            Deposit,
            DepositError,
            SequencerBlockHeader,
        },
        celestia::{
            SubmittedRollupData,
            UncheckedSubmittedMetadata,
            UncheckedSubmittedRollupData,
        },
    },
};
use astria_eyre::eyre::{
    bail,
    Result,
    WrapErr,
};
use astria_merkle::audit::Proof;
use base64::{
    prelude::BASE64_STANDARD,
    Engine,
};
use clap::ValueEnum;
use colour::write_blue;
use ethers_core::types::{
    transaction::eip2930::AccessListItem,
    Transaction,
};
use indenter::indented;
use itertools::Itertools;
use prost::{
    bytes::Bytes,
    Message,
};
use serde::Serialize;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Base64-encoded blob data, or a file containing this, or stdin if `-`
    #[arg(value_name = "BLOB|PATH")]
    input: String,

    /// Configure formatting of output
    #[arg(short, long, default_value_t = Format::Display, value_enum)]
    format: Format,

    /// Display verbose output (e.g. displays full contents of transactions in rollup data)
    #[arg(short, long)]
    verbose: bool,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Format {
    Display,
    Json,
}

/// Parses `input` (a base-64-encoded string of Celestia blob data) to the given format.
///
/// # Errors
///
/// Returns an error if `input` cannot be parsed.
pub fn run(
    Args {
        input,
        format,
        verbose,
    }: Args,
) -> Result<()> {
    let parsed_blob = parse(&input, verbose)?;
    match format {
        Format::Display => println!("\n{parsed_blob}"),
        Format::Json => println!(
            "{}",
            serde_json::to_string(&parsed_blob).wrap_err("failed to json-encode")?
        ),
    }
    Ok(())
}

fn parse(input: &str, verbose: bool) -> Result<ParsedBlob> {
    let raw = get_decoded_blob_data(input)?;
    #[allow(clippy::cast_precision_loss)]
    let compressed_size = raw.len() as f32;
    let decompressed =
        Bytes::from(decompress_bytes(&raw).wrap_err("failed to decompress decoded bytes")?);
    #[allow(clippy::cast_precision_loss)]
    let decompressed_size = decompressed.len() as f32;
    let compression_ratio = decompressed_size / compressed_size;

    let list = parse_list(decompressed, verbose)?;
    let number_of_entries = list.len();
    Ok(ParsedBlob {
        list,
        number_of_entries,
        compressed_size,
        decompressed_size,
        compression_ratio,
    })
}

fn get_decoded_blob_data(input: &str) -> Result<Vec<u8>> {
    if input == "-" {
        let encoded = io::read_to_string(io::stdin().lock()).wrap_err("failed to read stdin")?;
        return BASE64_STANDARD
            .decode(encoded.trim())
            .wrap_err("failed to decode stdin data as base64");
    }

    if Path::new(input).is_file() {
        let encoded =
            fs::read_to_string(input).wrap_err_with(|| format!("failed to read file `{input}`"))?;
        return BASE64_STANDARD
            .decode(encoded.trim())
            .wrap_err_with(|| format!("failed to decode contents of `{input}` as base64"));
    }

    BASE64_STANDARD
        .decode(input.trim())
        .wrap_err("failed to decode provided blob data as base64")
}

fn parse_list(decompressed: Bytes, verbose: bool) -> Result<ParsedList> {
    // Try to parse as a list of `SequencerBlockMetadata`.
    if let Some(metadata_list) = RawSubmittedMetadataList::decode(decompressed.clone())
        .ok()
        .and_then(|metadata_list| {
            metadata_list
                .entries
                .into_iter()
                .map(|raw_metadata| UncheckedSubmittedMetadata::try_from_raw(raw_metadata).ok())
                .collect::<Option<Vec<_>>>()
        })
    {
        return Ok(if verbose {
            metadata_list
                .iter()
                .map(VerboseSequencerBlockMetadata::new)
                .collect()
        } else {
            metadata_list
                .iter()
                .map(BriefSequencerBlockMetadata::new)
                .collect()
        });
    }

    // Try to parse as a list of `RollupData`.
    if let Some(rollup_data_list) = RawSubmittedRollupDataList::decode(decompressed.clone())
        .ok()
        .and_then(|rollup_data_list| {
            rollup_data_list
                .entries
                .into_iter()
                .map(|raw_rollup_data| SubmittedRollupData::try_from_raw(raw_rollup_data).ok())
                .collect::<Option<Vec<_>>>()
        })
    {
        return Ok(if verbose {
            rollup_data_list
                .into_iter()
                .map(|rollup_data| VerboseRollupData::new(&rollup_data.into_unchecked()))
                .collect()
        } else {
            rollup_data_list
                .into_iter()
                .map(|rollup_data| BriefRollupData::new(&rollup_data.into_unchecked()))
                .collect()
        });
    }

    // Try to parse as a single `SequencerBlockMetadata`.
    if let Some(metadata) = RawSubmittedMetadata::decode(decompressed.clone())
        .ok()
        .and_then(|raw_metadata| UncheckedSubmittedMetadata::try_from_raw(raw_metadata).ok())
    {
        return Ok(if verbose {
            ParsedList::VerboseSequencer(vec![VerboseSequencerBlockMetadata::new(&metadata)])
        } else {
            ParsedList::BriefSequencer(vec![BriefSequencerBlockMetadata::new(&metadata)])
        });
    }

    // Try to parse as a single `RollupData`.
    if let Some(rollup_data) = RawSubmittedRollupData::decode(decompressed)
        .ok()
        .and_then(|raw_rollup_data| SubmittedRollupData::try_from_raw(raw_rollup_data).ok())
    {
        return Ok(if verbose {
            ParsedList::VerboseRollup(vec![VerboseRollupData::new(&rollup_data.into_unchecked())])
        } else {
            ParsedList::BriefRollup(vec![BriefRollupData::new(&rollup_data.into_unchecked())])
        });
    }

    bail!("failed to decode as a list of sequencer metadata or rollup data")
}

#[derive(Serialize, Debug)]
struct PrintableSequencerBlockHeader {
    chain_id: String,
    height: u64,
    time: String,
    rollup_transactions_root: String,
    data_hash: String,
    proposer_address: String,
}

impl From<&SequencerBlockHeader> for PrintableSequencerBlockHeader {
    fn from(header: &SequencerBlockHeader) -> Self {
        Self {
            chain_id: header.chain_id().to_string(),
            height: header.height().value(),
            time: header.time().to_string(),
            rollup_transactions_root: BASE64_STANDARD.encode(header.rollup_transactions_root()),
            data_hash: BASE64_STANDARD.encode(header.data_hash()),
            proposer_address: BASE64_STANDARD.encode(header.proposer_address()),
        }
    }
}

impl Display for PrintableSequencerBlockHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "chain id", &self.chain_id)?;
        colored_ln(f, "height", self.height)?;
        colored_ln(f, "time", &self.time)?;
        colored_ln(
            f,
            "rollup transactions root",
            &self.rollup_transactions_root,
        )?;
        colored_ln(f, "data hash", &self.data_hash)?;
        colored(f, "proposer address", &self.proposer_address)
    }
}

#[derive(Serialize, Debug)]
struct PrintableMerkleProof {
    audit_path: String,
    leaf_index: usize,
    tree_size: NonZeroUsize,
}

impl From<&Proof> for PrintableMerkleProof {
    fn from(proof: &Proof) -> Self {
        Self {
            audit_path: BASE64_STANDARD.encode(proof.audit_path()),
            leaf_index: proof.leaf_index(),
            tree_size: proof.tree_size(),
        }
    }
}

impl Display for PrintableMerkleProof {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "audit path", &self.audit_path)?;
        colored_ln(f, "leaf index", self.leaf_index)?;
        colored(f, "tree size", self.tree_size)
    }
}

#[derive(Serialize, Debug)]
struct BriefSequencerBlockMetadata {
    sequencer_block_hash: String,
    sequencer_block_header: PrintableSequencerBlockHeader,
    rollup_ids: Vec<String>,
}

impl BriefSequencerBlockMetadata {
    fn new(metadata: &UncheckedSubmittedMetadata) -> Self {
        let rollup_ids = metadata
            .rollup_ids
            .iter()
            .map(RollupId::to_string)
            .collect();
        BriefSequencerBlockMetadata {
            sequencer_block_hash: BASE64_STANDARD.encode(metadata.block_hash),
            sequencer_block_header: PrintableSequencerBlockHeader::from(&metadata.header),
            rollup_ids,
        }
    }
}

impl Display for BriefSequencerBlockMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "sequencer block hash", &self.sequencer_block_hash)?;
        colored_label_ln(f, "sequencer block header")?;
        writeln!(indent(f), "{}", self.sequencer_block_header)?;
        if self.rollup_ids.is_empty() {
            colored_label(f, "rollup ids")?;
        } else {
            colored_label_ln(f, "rollup ids")?;
            write!(indent(f), "{}", self.rollup_ids.iter().join("\n"))?;
        }
        Ok(())
    }
}

#[derive(Serialize, Debug)]
struct VerboseSequencerBlockMetadata {
    sequencer_block_hash: String,
    sequencer_block_header: PrintableSequencerBlockHeader,
    rollup_ids: Vec<String>,
    rollup_transactions_proof: PrintableMerkleProof,
    rollup_ids_proof: PrintableMerkleProof,
}

impl VerboseSequencerBlockMetadata {
    fn new(metadata: &UncheckedSubmittedMetadata) -> Self {
        let rollup_ids = metadata
            .rollup_ids
            .iter()
            .map(RollupId::to_string)
            .collect();
        VerboseSequencerBlockMetadata {
            sequencer_block_hash: BASE64_STANDARD.encode(metadata.block_hash),
            sequencer_block_header: PrintableSequencerBlockHeader::from(&metadata.header),
            rollup_ids,
            rollup_transactions_proof: PrintableMerkleProof::from(
                &metadata.rollup_transactions_proof,
            ),
            rollup_ids_proof: PrintableMerkleProof::from(&metadata.rollup_ids_proof),
        }
    }
}

impl Display for VerboseSequencerBlockMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "sequencer block hash", &self.sequencer_block_hash)?;
        colored_label_ln(f, "sequencer block header")?;
        writeln!(indent(f), "{}", self.sequencer_block_header)?;
        colored_label_ln(f, "rollup ids")?;
        if !self.rollup_ids.is_empty() {
            writeln!(indent(f), "{}", self.rollup_ids.iter().join("\n"))?;
        }
        colored_label_ln(f, "rollup transactions proof")?;
        writeln!(indent(f), "{}", self.rollup_transactions_proof)?;
        colored_label_ln(f, "rollup ids proof")?;
        write!(indent(f), "{}", self.rollup_ids_proof)
    }
}

#[derive(Serialize, Debug)]
struct BriefRollupData {
    sequencer_block_hash: String,
    rollup_id: String,
    transaction_count: usize,
}

impl BriefRollupData {
    fn new(rollup_data: &UncheckedSubmittedRollupData) -> Self {
        BriefRollupData {
            sequencer_block_hash: BASE64_STANDARD.encode(rollup_data.sequencer_block_hash),
            rollup_id: rollup_data.rollup_id.to_string(),
            transaction_count: rollup_data.transactions.len(),
        }
    }
}

impl Display for BriefRollupData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "sequencer block hash", &self.sequencer_block_hash)?;
        colored_ln(f, "rollup id", &self.rollup_id)?;
        colored(f, "transaction count", self.transaction_count)
    }
}

#[derive(Serialize, Debug)]
struct PrintableAccessListItem {
    address: String,
    storage_keys: Vec<String>,
}

impl From<&AccessListItem> for PrintableAccessListItem {
    fn from(item: &AccessListItem) -> Self {
        Self {
            address: format!("{:?}", item.address),
            storage_keys: item
                .storage_keys
                .iter()
                .map(|storage_key| format!("{storage_key:?}"))
                .collect(),
        }
    }
}

impl Display for PrintableAccessListItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "address", &self.address)?;
        if self.storage_keys.is_empty() {
            colored_label(f, "storage keys")
        } else {
            colored_label_ln(f, "storage keys")?;
            write!(indent(f), "{}", self.storage_keys.iter().join("\n"))
        }
    }
}

#[derive(Serialize, Debug)]
struct RollupTransaction {
    hash: String,
    nonce: String,
    block_hash: Option<String>,
    block_number: Option<u64>,
    transaction_index: Option<u64>,
    from: String,
    to: Option<String>,
    value: String,
    gas_price: Option<String>,
    gas: String,
    input: String,
    v: u64,
    r: String,
    s: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_type: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_list: Option<Vec<PrintableAccessListItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_priority_fee_per_gas: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_fee_per_gas: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chain_id: Option<String>,
    other: String,
}

impl From<Transaction> for RollupTransaction {
    fn from(tx: Transaction) -> Self {
        Self {
            hash: format!("{:?}", tx.hash),
            nonce: tx.nonce.to_string(),
            block_hash: tx.block_hash.map(|hash| format!("{hash:?}")),
            block_number: tx.block_number.map(|v| v.as_u64()),
            transaction_index: tx.transaction_index.map(|v| v.as_u64()),
            from: format!("{:?}", tx.from),
            to: tx.to.map(|to| format!("{to:?}")),
            value: tx.value.to_string(),
            gas_price: tx.gas_price.map(|v| v.to_string()),
            gas: tx.gas.to_string(),
            input: BASE64_STANDARD.encode(&tx.input),
            v: tx.v.as_u64(),
            r: tx.r.to_string(),
            s: tx.s.to_string(),
            transaction_type: tx.transaction_type.map(|v| v.as_u64()),
            access_list: tx
                .access_list
                .map(|list| list.0.iter().map(PrintableAccessListItem::from).collect()),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas.map(|v| v.to_string()),
            max_fee_per_gas: tx.max_fee_per_gas.map(|v| v.to_string()),
            chain_id: tx.chain_id.map(|v| v.to_string()),
            other: serde_json::to_string(&tx.other).unwrap_or_default(),
        }
    }
}

impl Display for RollupTransaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "hash", &self.hash)?;
        colored_ln(f, "nonce", &self.nonce)?;
        colored_ln(f, "block hash", none_or_value(&self.block_hash))?;
        colored_ln(f, "block number", none_or_value(&self.block_number))?;
        colored_ln(
            f,
            "transaction index",
            none_or_value(&self.transaction_index),
        )?;
        colored_ln(f, "from", &self.from)?;
        colored_ln(f, "to", none_or_value(&self.to))?;
        colored_ln(f, "value", &self.value)?;
        colored_ln(f, "gas price", none_or_value(&self.gas_price))?;
        colored_ln(f, "gas", &self.gas)?;
        colored_ln(f, "input", &self.input)?;
        colored_ln(f, "v", self.v)?;
        colored_ln(f, "r", &self.r)?;
        colored_ln(f, "s", &self.s)?;
        if let Some(transaction_type) = self.transaction_type {
            colored_ln(f, "transaction type", transaction_type)?;
        }
        if let Some(access_list) = &self.access_list {
            colored_label_ln(f, "access list")?;
            if !access_list.is_empty() {
                writeln!(indent(f), "{}", access_list.iter().join("\n"))?;
            }
        }
        if let Some(max_priority_fee_per_gas) = &self.max_priority_fee_per_gas {
            colored_ln(f, "max priority fee per gas", max_priority_fee_per_gas)?;
        }
        if let Some(max_fee_per_gas) = &self.max_fee_per_gas {
            colored_ln(f, "max fee per gas", max_fee_per_gas)?;
        }
        if let Some(chain_id) = &self.chain_id {
            colored_ln(f, "chain id", chain_id)?;
        }
        colored(f, "other", &self.other)
    }
}

#[derive(Serialize, Debug)]
struct PrintableDeposit {
    bridge_address: String,
    rollup_id: String,
    amount: u128,
    asset: String,
    destination_chain_address: String,
}

impl TryFrom<&RawDeposit> for PrintableDeposit {
    type Error = DepositError;

    fn try_from(raw_deposit: &RawDeposit) -> Result<Self, Self::Error> {
        let deposit = Deposit::try_from_raw(raw_deposit.clone())?;
        Ok(PrintableDeposit {
            bridge_address: deposit.bridge_address().to_string(),
            rollup_id: deposit.rollup_id().to_string(),
            amount: deposit.amount(),
            asset: deposit.asset().to_string(),
            destination_chain_address: deposit.destination_chain_address().to_string(),
        })
    }
}

impl Display for PrintableDeposit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "bridge address", &self.bridge_address)?;
        colored_ln(f, "rollup id", &self.rollup_id)?;
        colored_ln(f, "amount", self.amount)?;
        colored_ln(f, "asset id", &self.asset)?;
        colored(
            f,
            "destination chain address",
            &self.destination_chain_address,
        )
    }
}

// allow: not performance-critical.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Debug)]
enum RollupDataDetails {
    #[serde(rename = "rollup_transaction")]
    Transaction(RollupTransaction),
    #[serde(rename = "deposit")]
    Deposit(PrintableDeposit),
    /// Tx doesn't decode as `RawRollupData`.  Wrapped value is base-64-encoded input data.
    #[serde(rename = "not_tx_or_deposit")]
    NotTxOrDeposit(String),
    /// Tx parses as `RawRollupData` but its value is empty.
    #[serde(rename = "empty_rollup_data")]
    EmptyBytes,
    /// Tx parses as `RawRollupData::SequencedData`, but its value doesn't decode as an ethers
    /// `Transaction`.  Wrapped value is base-64-encoded input data.
    #[serde(rename = "unknown_rollup_transaction_type")]
    UnknownTransaction(String),
    /// Tx parses as `RawRollupData::Deposit`, but its value doesn't decode as a `Deposit`.
    /// Wrapped value is decoding error and the debug contents of the raw (protobuf) deposit.
    #[serde(rename = "unparseable_deposit")]
    UnparseableDeposit(String),
}

impl From<&Vec<u8>> for RollupDataDetails {
    fn from(value: &Vec<u8>) -> Self {
        let Ok(raw_rollup_data) = RawRollupData::decode(Bytes::from(value.clone())) else {
            return RollupDataDetails::NotTxOrDeposit(BASE64_STANDARD.encode(value));
        };
        match raw_rollup_data.value {
            None => RollupDataDetails::EmptyBytes,
            Some(RawRollupDataValue::SequencedData(tx_data)) => {
                let Ok(tx) = rlp::decode::<Transaction>(&tx_data) else {
                    return RollupDataDetails::UnknownTransaction(BASE64_STANDARD.encode(&tx_data));
                };
                RollupDataDetails::Transaction(RollupTransaction::from(tx))
            }
            Some(RawRollupDataValue::Deposit(raw_deposit)) => {
                match PrintableDeposit::try_from(&raw_deposit) {
                    Ok(printable_deposit) => RollupDataDetails::Deposit(printable_deposit),
                    Err(error) => {
                        RollupDataDetails::UnparseableDeposit(format!("{raw_deposit:?}: {error}"))
                    }
                }
            }
        }
    }
}

impl Display for RollupDataDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RollupDataDetails::Transaction(txn) => {
                colored_label_ln(f, "transaction")?;
                write!(indent(f), "{txn}")
            }
            RollupDataDetails::Deposit(deposit) => {
                colored_label_ln(f, "deposit")?;
                write!(indent(f), "{deposit}")
            }
            RollupDataDetails::NotTxOrDeposit(value) => colored(f, "not tx or deposit", value),
            RollupDataDetails::EmptyBytes => {
                write!(f, "empty rollup data")
            }
            RollupDataDetails::UnknownTransaction(value) => {
                colored(f, "unknown rollup transaction type", value)
            }
            RollupDataDetails::UnparseableDeposit(error) => {
                colored(f, "unparseable deposit", error)
            }
        }
    }
}

#[derive(Serialize, Debug)]
struct VerboseRollupData {
    sequencer_block_hash: String,
    rollup_id: String,
    transactions_and_deposits: Vec<RollupDataDetails>,
    item_count: usize,
    proof: PrintableMerkleProof,
}

impl VerboseRollupData {
    fn new(rollup_data: &UncheckedSubmittedRollupData) -> Self {
        let transactions_and_deposits: Vec<_> = rollup_data
            .transactions
            .iter()
            .map(RollupDataDetails::from)
            .collect();
        let item_count = transactions_and_deposits.len();
        VerboseRollupData {
            sequencer_block_hash: BASE64_STANDARD.encode(rollup_data.sequencer_block_hash),
            rollup_id: rollup_data.rollup_id.to_string(),
            transactions_and_deposits,
            item_count,
            proof: PrintableMerkleProof::from(&rollup_data.proof),
        }
    }
}

impl Display for VerboseRollupData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        colored_ln(f, "sequencer block hash", &self.sequencer_block_hash)?;
        colored_ln(f, "rollup id", &self.rollup_id)?;
        for (index, item) in self.transactions_and_deposits.iter().enumerate() {
            colored_label_ln(f, &format!("item {index}"))?;
            writeln!(indent(f), "{item}")?;
        }
        colored_ln(f, "item count", self.item_count)?;
        colored_label_ln(f, "proof")?;
        write!(indent(f), "{}", self.proof)
    }
}

#[derive(Serialize, Debug)]
enum ParsedList {
    #[serde(rename = "sequencer_metadata_list")]
    BriefSequencer(Vec<BriefSequencerBlockMetadata>),
    #[serde(rename = "sequencer_metadata_list")]
    VerboseSequencer(Vec<VerboseSequencerBlockMetadata>),
    #[serde(rename = "rollup_data_list")]
    BriefRollup(Vec<BriefRollupData>),
    #[serde(rename = "rollup_data_list")]
    VerboseRollup(Vec<VerboseRollupData>),
}

impl ParsedList {
    fn len(&self) -> usize {
        match self {
            ParsedList::BriefSequencer(list) => list.len(),
            ParsedList::VerboseSequencer(list) => list.len(),
            ParsedList::BriefRollup(list) => list.len(),
            ParsedList::VerboseRollup(list) => list.len(),
        }
    }
}

impl FromIterator<BriefSequencerBlockMetadata> for ParsedList {
    fn from_iter<I: IntoIterator<Item = BriefSequencerBlockMetadata>>(iter: I) -> Self {
        Self::BriefSequencer(Vec::from_iter(iter))
    }
}

impl FromIterator<VerboseSequencerBlockMetadata> for ParsedList {
    fn from_iter<I: IntoIterator<Item = VerboseSequencerBlockMetadata>>(iter: I) -> Self {
        Self::VerboseSequencer(Vec::from_iter(iter))
    }
}

impl FromIterator<BriefRollupData> for ParsedList {
    fn from_iter<I: IntoIterator<Item = BriefRollupData>>(iter: I) -> Self {
        Self::BriefRollup(Vec::from_iter(iter))
    }
}

impl FromIterator<VerboseRollupData> for ParsedList {
    fn from_iter<I: IntoIterator<Item = VerboseRollupData>>(iter: I) -> Self {
        Self::VerboseRollup(Vec::from_iter(iter))
    }
}

impl Display for ParsedList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParsedList::BriefSequencer(list) => {
                for (index, item) in list.iter().enumerate() {
                    colored_label_ln(f, &format!("sequencer metadata {index}"))?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
            ParsedList::VerboseSequencer(list) => {
                for (index, item) in list.iter().enumerate() {
                    colored_label_ln(f, &format!("sequencer metadata {index}"))?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
            ParsedList::BriefRollup(list) => {
                for (index, item) in list.iter().enumerate() {
                    colored_label_ln(f, &format!("rollup data {index}"))?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
            ParsedList::VerboseRollup(list) => {
                for (index, item) in list.iter().enumerate() {
                    colored_label_ln(f, &format!("rollup data {index}"))?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Serialize, Debug)]
struct ParsedBlob {
    #[serde(flatten)]
    list: ParsedList,
    number_of_entries: usize,
    #[serde(rename = "compressed_size_bytes")]
    compressed_size: f32,
    #[serde(rename = "decompressed_size_bytes")]
    decompressed_size: f32,
    compression_ratio: f32,
}

impl Display for ParsedBlob {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.list)?;
        colored_ln(f, "number of entries", self.number_of_entries)?;
        colored(f, "compressed size", self.compressed_size)?;
        writeln!(f, " bytes")?;
        colored(f, "decompressed size", self.decompressed_size)?;
        writeln!(f, " bytes")?;
        colored(f, "compression ratio", self.compression_ratio)
    }
}

fn indent<'a, 'b>(f: &'a mut Formatter<'b>) -> indenter::Indented<'a, Formatter<'b>> {
    indented(f).with_str("    ")
}

fn none_or_value<T: ToString>(maybe_value: &Option<T>) -> String {
    maybe_value
        .as_ref()
        .map_or("none".to_string(), T::to_string)
}

fn colored_label(f: &mut Formatter<'_>, label: &str) -> fmt::Result {
    write_blue!(f, "{label}")?;
    write!(f, ":")
}

fn colored_label_ln(f: &mut Formatter<'_>, label: &str) -> fmt::Result {
    write_blue!(f, "{label}")?;
    writeln!(f, ":")
}

fn colored<T: Display>(f: &mut Formatter<'_>, label: &str, item: T) -> fmt::Result {
    write_blue!(f, "{label}")?;
    write!(f, ": {item}")
}

fn colored_ln<T: Display>(f: &mut Formatter<'_>, label: &str, item: T) -> fmt::Result {
    write_blue!(f, "{label}")?;
    writeln!(f, ": {item}")
}
