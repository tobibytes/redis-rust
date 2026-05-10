# redis-rust

A minimal async Redis server in Rust. Speaks the [RESP](https://redis.io/docs/latest/develop/reference/protocol-spec/) protocol over TCP, handles connections concurrently with Tokio, and shares an in-memory store across them. Works with the real `redis-cli`.

Built from the [CodeCrafters "Build Your Own Redis"](https://codecrafters.io/challenges/redis) challenge.

## Commands

| Command | Behavior |
| --- | --- |
| `PING [msg]` | Replies `+PONG` (or echoes `msg` as a bulk string) |
| `ECHO <msg>` | Echoes the argument as a bulk string |
| `SET <key> <value>` | Stores the value, replies `+OK` |
| `GET <key>` | Returns the stored value, or `$-1\r\n` (nil) if missing |

## How it works

- **Tokio runtime** — every accepted connection is handed to a spawned task; a single `TcpListener` fans out concurrent clients.
- **Shared state** — keys live in an `Arc<Mutex<HashMap<String, String>>>` so all tasks read/write the same store.
- **RESP parser** — hand-written: reads `*N\r\n` for the array length, then `$len\r\n<data>\r\n` for each bulk-string argument.
- **No external Redis libs** — the protocol is implemented from the spec up.

## Run it

```sh
cargo build --release
./your_program.sh
# or: cargo run --release
```

Then in another shell:

```sh
redis-cli -p 6379 PING                # PONG
redis-cli -p 6379 SET hello world     # OK
redis-cli -p 6379 GET hello           # "world"
redis-cli -p 6379 ECHO "ping me"      # "ping me"
```

## Stack

Rust 2021 · `tokio` (full) · `bytes` · `anyhow`
