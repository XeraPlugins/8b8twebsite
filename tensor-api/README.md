# Tensor API

8b8tTensor, stats API.

## Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Setup

```bash
cd tensor-api/
cargo build --release
```

## Run

```bash
# Set server path if not using default ../../server
export SERVER_PATH=/path/to/server
cargo run --release
```

## Endpoints

- `GET /player/:name` - Get player stats by name or UUID
- `GET /leaderboard?limit=10` - Get leaderboard (default 10 players)
