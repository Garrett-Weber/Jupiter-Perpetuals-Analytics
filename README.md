# Jupiter Perpetuals Analytics CLI

A CLI tool for collecting analytics about Jupiter Perpetuals usage on the Solana blockchain.

## Usage

jupiter_perpetuals_analytics [OPTIONS] -r \<rpc_url\>

## Description

This CLI tool is designed to collect analytics data related to the usage of Jupiter Perpetuals on the Solana blockchain. It utilizes the provided Solana RPC URL to gather on-chain information.

## Example

```
./jupiter_perpetuals_analytics -r https://solana-rpc-url
Unix time: 1705360030
Total pool value: $50,021,639
Total traders unrealized paper P&L: $-6,510
Total traders fees: $329,550
Total traders unrealized real P&L $-336,060
Total value of positions: $30,953,417
Total value of collateral: $4,766,342
Average leverage at entry: 6.4823
Average effective leverage: 6.4942
Long trades: 9406 ($27,119,544)
Short trades: 999 ($3,833,873)
L/S ratio: 9.4154 (7.0737)
Winning trades: 4410 Losing trades: 5995
Most profitable open trade: 4mVANoGPtVsZ4FXyNCpkmt4owbGEaiKGskvHFRHrRVQK Open P&L: $71,459 Entry Price $54.91 Side: Long Mint So11111111111111111111111111111111111111112
Most unprofitable open trade: Gihk4TajSrkqToFvyV377eShgnL37sTsMTtFuVGvx3P6 Open P&L: $-18,140 Entry Price $2195.01 Side: Short Mint 7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs
```

### Options:

- `-r <RPC_URL>`: Solana RPC URL (Required)
- `-c <CSV_PATH>`: Export to CSV (Optional)
- `-s`: Silent mode (Optional)
- `-h, --help`: Print help
- `-V, --version`: Print version

### Export to CSV

You can use the `-c` option to export the collected analytics data to a CSV file. Provide the file path as an argument.

### Silent Mode

The `-s` option enables silent mode, suppressing unnecessary output during execution.

## Help and Version

Use the `-h` flag to print the help menu. Use the `-V` flag to print the version information.

# Disclaimer

This software is provided as is, without any warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, and non-infringement. In no event shall the authors be liable for any claim, damages, or other liability.

# License

This project is licensed under the MIT License.
