# Jupiter Perpetuals Analytics CLI

A CLI tool for collecting analytics about Jupiter Perpetuals usage on the Solana blockchain.

## Usage

jupiter_perpetuals_analytics [OPTIONS] -r \<rpc_url\>

### Options:

- `-r <RPC_URL>`: Solana RPC URL (Required)
- `-c <CSV_PATH>`: Export to CSV (Optional)
- `-s`: Silent mode (Optional)
- `-h, --help`: Print help
- `-V, --version`: Print version

## Description

This CLI tool is designed to collect analytics data related to the usage of Jupiter Perpetuals on the Solana blockchain. It utilizes the provided Solana RPC URL to gather on-chain information.

### Export to CSV

You can use the `-c` option to export the collected analytics data to a CSV file. Provide the file path as an argument.

### Silent Mode

The `-s` option enables silent mode, suppressing unnecessary output during execution.

## Help and Version

Use the `-h` flag to print the help menu. Use the `-V` flag to print the version information.

# License

This project is licensed under the MIT License.
