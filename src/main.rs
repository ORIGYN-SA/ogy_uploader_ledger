use std::{fs::File, io::Read};

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use clap::Parser;
use garcon::Delay;
use ic_agent::{agent::UpdateBuilder, identity::Secp256k1Identity, Agent, Identity};
use openssl::ec::EcKey;
use rusqlite::{Connection, Result};
use serde::Serialize;

#[derive(Parser, Debug)]
#[clap(author)]
struct Args {
    db_path: String,
    seed_file_path: String,
    canister_principal: String,
    icp_endpoint: String,
    chunk_size: usize,
}

#[derive(Debug, Serialize, Deserialize, CandidType, Clone)]
struct Block {
    // hash: Vec<u8>,
    block: Vec<u8>,
    // parent_hash: Option<Vec<u8>>,
    // idx: i32,
    // verified: bool,
}

fn create_identity(seed_file_path: String) -> impl Identity {
    // open file at seed_file_path as bytes
    let mut file = File::open(seed_file_path).expect("Unable to open seed file");
    let mut seed_bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut seed_bytes)
        .expect("Unable to read seed file");
    let private_key = EcKey::private_key_from_pem(&seed_bytes).expect("Unable to read private key");
    Secp256k1Identity::from_private_key(private_key)
}

async fn send_blocks(
    blocks: &[Vec<u8>],
    update_builder: &mut UpdateBuilder<'_>,
    waiter: &Delay,
) -> bool {
    let response = update_builder
        .with_arg(&Encode!(&blocks).unwrap())
        .call_and_wait(waiter.to_owned())
        .await
        .expect(format!("block upload failed").as_str());
    Decode!(response.as_slice(), ()).is_ok()
}

fn get_blocks(conn: &Connection) -> Result<Vec<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT * FROM blocks order by idx")?;
    let blocks: Vec<Vec<u8>> = stmt
        .query_map([], |row| {
            // Ok(Block {
            //     // hash: row.get(0)?,
            //     block: row.get(1)?,
            //     // parent_hash: row.get(2)?,
            //     // idx: row.get(3)?,
            //     // verified: row.get(4)?,
            // })
            Ok(row.get(1)?)
        })?
        .collect::<Result<Vec<Vec<u8>>>>()?;
    Ok(blocks)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let conn = Connection::open("src/rosetta-data/db.sqlite").expect("Unable to open database");
    let blocks = get_blocks(&conn).expect("Unable to get blocks");
    let chunked_blocks = blocks.chunks(args.chunk_size);

    let agent = Agent::builder()
        .with_url(args.icp_endpoint)
        .with_identity(create_identity(args.seed_file_path))
        .build()
        .unwrap();

    agent.fetch_root_key().await;
    let mut update_builder = agent.update(
        &Principal::from_text(args.canister_principal).expect("Unable to parse canister principal"),
        "replace_blocks",
    );

    let waiter = garcon::Delay::builder()
        .throttle(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(60 * 5))
        .build();

    let mut current_block_chunk = 0;
    for blocks in chunked_blocks {
        current_block_chunk += 1;
        let result = send_blocks(&blocks, &mut update_builder, &waiter).await;
        if !result {
            println!("Failed to send blocks");
            println!("Chunk {:?}", current_block_chunk);
            println!("{:?}", blocks);
            break;
        }
        println!("Sent chunk {}", current_block_chunk);
        println!("Sent {} blocks", current_block_chunk * blocks.len());
    }

    println!("Done");
}
