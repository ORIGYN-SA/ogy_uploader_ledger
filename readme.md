#Block Uploader

Tool for uploading sqlite stored blocks to ledger.

# Usage
Expects the following arguments:


*db_path*: Path to the sqlite database containing the blocks.

*seed_file_path*: Path to the file containing the seed for authenticated identity.

*canister_principal*: Principal of the canister to upload the blocks to.

*icp_endpoint*: eg localhost, https://boundary.ic0.app

*chunk_size*: Number of blocks to upload at a time.

# Run
`cargo build`

`./target/debug/ogy_block_uploader <DB_PATH> <SEED_FILE_PATH> <CANISTER_PRINCIPAL> <ICP_ENDPOINT> <CHUNK_SIZE>`