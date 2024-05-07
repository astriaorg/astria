use std::{
    fmt::{
        self,
        Display,
        Formatter,
        Write,
    },
    num::NonZeroUsize,
};

use astria_core::{
    brotli::decompress_bytes,
    generated::sequencerblock::v1alpha1::{
        SubmittedMetadata as RawSubmittedMetadata,
        SubmittedMetadataList as RawSubmittedMetadataList,
        SubmittedRollupData as RawSubmittedRollupData,
        SubmittedRollupDataList as RawSubmittedRollupDataList,
    },
    primitive::v1::RollupId,
    sequencerblock::v1alpha1::{
        block::SequencerBlockHeader,
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
use indenter::indented;
use itertools::Itertools;
use prost::{
    bytes::Bytes,
    Message,
};
use serde::Serialize;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Base64-encoded blob data
    #[arg(value_name = "BLOB")]
    input: String,

    /// Configure formatting of output
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_value_t = Format::Display,
        default_missing_value = "always",
        value_enum
    )]
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
    let parsed_list = parse(input, verbose)?;
    match format {
        Format::Display => println!("\n{parsed_list}"),
        Format::Json => println!(
            "{}",
            serde_json::to_string(&parsed_list).wrap_err("failed to json-encode")?
        ),
    }
    Ok(())
}

fn parse(input: String, verbose: bool) -> Result<ParsedList> {
    let raw = BASE64_STANDARD
        .decode(input)
        .wrap_err("failed to decode as base64")?;
    let decompressed =
        Bytes::from(decompress_bytes(&raw).wrap_err("failed to decompress decoded bytes")?);

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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "chain id: {}", self.chain_id)?;
        writeln!(formatter, "height: {}", self.height)?;
        writeln!(formatter, "time: {}", self.time)?;
        writeln!(
            formatter,
            "rollup transactions root: {}",
            self.rollup_transactions_root
        )?;
        writeln!(formatter, "data hash: {}", self.data_hash)?;
        write!(formatter, "proposer address: {}", self.proposer_address)
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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "audit path: {}", self.audit_path)?;
        writeln!(formatter, "leaf index: {}", self.leaf_index)?;
        write!(formatter, "tree size: {}", self.tree_size)
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
        writeln!(f, "sequencer block hash: {}", self.sequencer_block_hash)?;
        writeln!(f, "sequencer block header:")?;
        writeln!(indent(f), "{}", self.sequencer_block_header)?;
        if self.rollup_ids.is_empty() {
            write!(f, "rollup ids:")?;
        } else {
            writeln!(f, "rollup ids:")?;
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
        writeln!(f, "sequencer block hash: {}", self.sequencer_block_hash)?;
        writeln!(f, "sequencer block header:")?;
        writeln!(indent(f), "{}", self.sequencer_block_header)?;
        writeln!(f, "rollup ids:")?;
        if !self.rollup_ids.is_empty() {
            writeln!(indent(f), "{}", self.rollup_ids.iter().join("\n"))?;
        }
        writeln!(f, "rollup transactions proof:")?;
        writeln!(indent(f), "{}", self.rollup_transactions_proof)?;
        writeln!(f, "rollup ids proof:")?;
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
        writeln!(f, "sequencer block hash: {}", self.sequencer_block_hash)?;
        writeln!(f, "rollup id: {}", self.rollup_id)?;
        write!(f, "transaction count: {}", self.transaction_count)
    }
}

#[derive(Serialize, Debug)]
struct VerboseRollupData {
    sequencer_block_hash: String,
    rollup_id: String,
    transactions: Vec<String>,
    proof: PrintableMerkleProof,
}

impl VerboseRollupData {
    fn new(rollup_data: &UncheckedSubmittedRollupData) -> Self {
        let transactions = rollup_data
            .transactions
            .iter()
            .map(|txn| BASE64_STANDARD.encode(txn))
            .collect();
        VerboseRollupData {
            sequencer_block_hash: BASE64_STANDARD.encode(rollup_data.sequencer_block_hash),
            rollup_id: rollup_data.rollup_id.to_string(),
            transactions,
            proof: PrintableMerkleProof::from(&rollup_data.proof),
        }
    }
}

impl Display for VerboseRollupData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "sequencer block hash: {}", self.sequencer_block_hash)?;
        writeln!(f, "rollup id: {}", self.rollup_id)?;
        writeln!(f, "transactions:")?;
        if !self.transactions.is_empty() {
            writeln!(indent(f), "{}", self.transactions.iter().join("\n"))?;
        }
        writeln!(f, "proof:")?;
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
                    writeln!(f, "sequencer metadata {index}:")?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
            ParsedList::VerboseSequencer(list) => {
                for (index, item) in list.iter().enumerate() {
                    writeln!(f, "sequencer metadata {index}:")?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
            ParsedList::BriefRollup(list) => {
                for (index, item) in list.iter().enumerate() {
                    writeln!(f, "rollup data {index}:")?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
            ParsedList::VerboseRollup(list) => {
                for (index, item) in list.iter().enumerate() {
                    writeln!(f, "rollup data {index}:")?;
                    writeln!(indent(f), "{item}")?;
                }
                Ok(())
            }
        }
    }
}

fn indent<'a, 'b>(f: &'a mut Formatter<'b>) -> indenter::Indented<'a, Formatter<'b>> {
    indented(f).with_str("    ")
}
