use crate::rpc_log::RpcLog;
use alloy_consensus::{BlockBody, ReceiptWithBloom};
use alloy_primitives::Log;
use eyre::Result;
use gnosis_primitives::header::GnosisHeader;
use reth_era::{
    e2s_types::E2sError,
    era1_file::Era1Reader,
    execution_types::BlockTuple,
    DecodeCompressed
};
use reth_primitives::TransactionSigned;
use std::{error::Error, fs::File, path::Path};
use reth_ethereum_primitives::Receipt;

mod rpc_log;

type ReceiptType = ReceiptWithBloom<Receipt>;

fn decode_block<E>(
    block: Result<BlockTuple, E>,
) -> eyre::Result<(
    GnosisHeader,
    BlockBody<TransactionSigned, GnosisHeader>,
    Vec<ReceiptType>,
)>
where
    E: From<E2sError> + Error + Send + Sync + 'static,
{
    let block = block?;

    let header: GnosisHeader = block.header.decode()?;
    let body = block.body.decode()?;
    // println!("Receipts: {}", block.receipts.data.len());
    let receipts: Vec<ReceiptType> = block.receipts.decode().unwrap_or_else(|_| {
        // If decoding fails, we can return an empty vector or handle the error as needed
        println!("Failed to decode receipts, returning empty vector");
        vec![]
    });
    // let receipts: Vec<ReceiptWithBloom> = vec![];

    println!("Height: {}", header.number);

    Ok((header, body, receipts))
}

fn get_topic(log: &Log, index: usize) -> Vec<u8> {
    log.data
        .topics()
        .get(index)
        .map_or_else(|| vec![], |topic| topic.to_vec())
}

fn main() -> Result<()> {
    let era_num = "00000";

    let era_startswith = format!("gnosis-{}", era_num);
    let era_directory = "/Users/deb/Gnosis/gnosis_downloader/downloads";
    // find all files in the era directory that start with the era_startswith, ideally this would return just one file
    let era_files: Vec<_> = std::fs::read_dir(era_directory)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().map_or(false, |s| s.starts_with(&era_startswith)))
        .collect();

    let file = File::open(era_files.get(0).map(|e| e.path()).unwrap_or_else(|| {
        panic!("No era file found starting with the specified prefix")
    }))?;

    // let file = File::open(Path::new("gnosis-04800-9d927140.era1"))?;
    let reader = Era1Reader::new(file);

    let mut i = 0;

    for block in reader.iter().map(decode_block) {
        println!("Block: {}", i);
        let (header, body, receipts) = block?;
        println!("Txs: {}, Receipts: {}", body.transactions.len(), receipts.len());
        if body.transactions.len() != receipts.len() {
            panic!("Warning: Mismatch between number of transactions and receipts for block {}", header.number);
        }
        let block_hash = header.hash_slow();

        for (transaction_index, receipt) in receipts.iter().enumerate() {
            let transaction_hash = body.transactions[transaction_index].tx_hash();

            for (log_index, log) in receipt.receipt.logs.iter().enumerate() {
                let rpc_log = RpcLog {
                    address: log.address.to_vec(),
                    topic_0: get_topic(log, 0),
                    topic_1: get_topic(log, 1),
                    topic_2: get_topic(log, 2),
                    topic_3: get_topic(log, 3),
                    data: log.data.data.to_vec(),
                    block_number: header.number,
                    transaction_hash: transaction_hash.to_vec(),
                    transaction_index: transaction_index as u64,
                    block_hash: block_hash.to_vec(),
                    log_index: log_index as u64,
                };

                // println!("{}", rpc_log);
            }
        }
        i += 1;
        if i > 50 {
            break;
        }
        // break;
    }

    Ok(())
}
