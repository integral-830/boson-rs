# boson-rs

> **A Redis-compatible, async in-memory key-value server written in Rust.**

[![CI](https://img.shields.io/github/actions/workflow/status/integral-830/boson-rs/ci.yml?branch=main&label=CI&logo=github)](https://github.com/integral-830/boson-rs/actions) [![Rust](https://img.shields.io/badge/rust-1.77%2B-orange?logo=rust)](https://www.rust-lang.org/) [![Docker](https://img.shields.io/badge/docker-ready-2496ED?logo=docker)](https://github.com/integral-830/boson-rs/blob/main/Dockerfile) [![Tokio](https://img.shields.io/badge/async-tokio-purple)](https://tokio.rs/)

boson-rs is a Redis-compatible server written in Rust and powered by Tokio's asynchronous runtime. Built entirely from scratch, the project explores the core building blocks of modern backend systems: network protocols, concurrency, memory management, benchmarking, and performance tuning. The goal is not to compete with Redis, but to understand the engineering decisions that make systems like it fast, reliable, and scalable.

---

> **📄 Documentation**

| Document                           | Contents                                                                                                   |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| [architecture.md](architecture.md) | System diagrams, component reference, protocol support matrix, command reference, graceful shutdown design |
| [benchmarks.md](benchmarks.md)     | Full benchmark environment, non-pipelined & pipelined results, Redis comparison, client scaling analysis   |

## Table of Contents

- [Project Overview](#project-overview)

- [Key Features](#key-features)

- [Performance Highlights](#performance-highlights)

- [Project Structure](#project-structure)

- [Getting Started](#getting-started)

- [Testing](#testing)

- [License](#license)

---

## Project Overview

### What boson-rs Is

boson-rs is an asynchronous TCP server that speaks the Redis Serialization Protocol (RESP). Clients that work with Redis — including `redis-cli` and `redis-benchmark` — connect to boson-rs without modification. Internally, all data lives in a concurrent in-memory hash map backed by [DashMap](https://github.com/xacrimon/dashmap).

### Goals

- Implement RESP parsing and encoding from scratch

- Build a correct, concurrent async server using Tokio

- Apply and measure reliability patterns (connection limiting, shutdown)

- Benchmark against Redis and understand where and why gaps exist

- Profile under load with Samply

- Learn what observability costs at the hot path

### Current Scope

boson-rs currently handles `PING`, `ECHO`, `GET`, `SET`, `INCR`, `DEL`, `EXISTS`, `EXPIRE`, `TTL`, `COMMAND DOCS`, and `CONFIG GET`. It supports pipelining, connection limiting, graceful shutdown, idle timeouts, and frame size guards.

### Non-Goals

- Production deployment

- Persistence or replication

- Full Redis command coverage

- Cluster mode

---

## Key Features

### Implemented

- [x] RESP2 protocol — full parser and encoder
- [x] Async TCP server using Tokio
- [x] Custom framed codec via `tokio-util::codec`
- [x] Command dispatch: `PING`, `ECHO`, `GET`, `SET`, `INCR`, `DEL`,`EXISTS`
- [x] Docs (`CONFIG GET`, `COMMAND DOCS`)(Currently it returns empty array. No doc implementaions.)
- [x] Key expiry (`TTL`, `EXPIRE`)
- [x] Pipelining support (tested at depth 16 and 64)
- [x] Connection limiting via `tokio::sync::Semaphore` (max 1,000 clients)
- [x] Graceful shutdown via `CancellationToken` + `JoinSet`
- [x] Maximum frame size guard (512 MB)
- [x] Maximum array length guard (1,000,000 elements)
- [x] AHash-backed DashMap for lower key-hashing overhead
- [x] Multi-stage Docker build
- [x] GitHub Actions CI (fmt → clippy → test → release build)
- [x] Integration tests
- [x] Property-based tests

### Planned

- [ ] Persistence (RDB / AOF)
- [ ] Pub/Sub
- [ ] Transactions (`MULTI` / `EXEC`)
- [ ] Replication
- [ ] Cluster mode
- [ ] Structured observability (low-overhead counters + opt-in histograms)
- [ ] Expanded command coverage

## Performance Highlights

All benchmarks were run with `redis-benchmark` on localhost against a release build.

| Category                       |                       Value |
| ------------------------------ | --------------------------: |
| Peak GET Throughput            |         **1,503,759 req/s** |
| Peak SET Throughput            |         **1,426,534 req/s** |
| GET Throughput (Pipeline 16)   |             1,060,000 req/s |
| SET Throughput (Pipeline 16)   |               885,000 req/s |
| Non-Pipelined Throughput       |       150,000–160,000 req/s |
| p50 Latency (100 clients, P64) |                  3.7–3.9 ms |
| Maximum Concurrent Connections |                       1,000 |
| Pipelining Improvement         |                       8–10× |
| Measured Metrics Overhead      | ~30% (Prometheus Histogram) |

---

> For the complete benchmark suite including detailed comparisons against Redis, see [benchmarks.md](benchmarks.md).

---

## Project Structure

```
boson/
├── src/
│   ├── main.rs        # Entry point: binds TcpListener, starts accept loop, wires shutdown
│   ├── server.rs      # Accept loop, Semaphore acquisition, tokio::spawn per connection
│   ├── handler.rs     # Per-connection task: drives Framed read/write loop, idle timeout
│   ├── codec.rs       # RespCodec: implements Decoder + Encoder for tokio-util::codec
│   ├── cmd.rs         # Command types, parse_command(), argument validation
│   ├── exec.rs        # execute(): dispatches parsed commands to store operations
│   └── store.rs       # Arc<Store> wrapping DashMap<String, Bytes> with AHash
├── tests/
│   ├── integration.rs   # Full server spin-up tests via TCP
│   └── prop_codec.rs      # proptest-based property tests for codec and store
├── Dockerfile         # Multi-stage build: builder + minimal runtime image
├── .github/
│   └── workflows/
│       └── ci.yml     # fmt → clippy → test → release build
├── Cargo.toml
└── README.md

```

## Getting Started

Follow the steps below to build and run Boson locally.

### Prerequisites

| Tool      | Version                         | Purpose                          |
| --------- | ------------------------------- | -------------------------------- |
| Rust      | **1.85+**                       | Core language and toolchain      |
| Cargo     | Bundled with Rust               | Build system and package manager |
| Git       | Latest                          | Clone and manage the repository  |
| Docker    | 24+ _(optional)_                | Containerized deployment         |
| redis-cli | Any recent version _(optional)_ | Manual testing and validation    |

> Boson is developed and tested primarily on macOS using the stable Rust toolchain.

---

### 1. Clone the Repository

```bash
git clone https://github.com/integral-830/boson-rs.git
cd boson-rs
```

---

### 2. Build the Project

Development build:

```bash
cargo build
```

Optimized release build:

```bash
cargo build --release
```

---

### 3. Run the Server

Using Cargo:

```bash
RUST_LOG=warn cargo run --release
```

```bash
# With logging
RUST_LOG=debug cargo run --release
```

Or directly from the generated binary:

```bash
RUST_LOG=warn ./target/release/boson-rs
```

By default, Boson listens on:

```text
127.0.0.1:6380
```

---

### 4. Verify the Server

Using `redis-cli`:

```bash
redis-cli -p 6380
```

Test basic commands:

```redis
PING
PONG

SET hello world
OK

GET hello
"world"

INCR counter
(integer) 1
```

---

### 5. Run the Test Suite

Run all tests:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

Run Clippy:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Format verification:

```bash
cargo fmt --check
```

---

### 6. Run Benchmarks

Non-pipelined benchmark:

```bash
redis-benchmark \
-h 127.0.0.1 \
-p 6380 \
-c 50 \
-n 100000 \
-q \
-t SET,GET
```

High-throughput pipelined benchmark:

```bash
redis-benchmark \
-h 127.0.0.1 \
-p 6380 \
-c 100 \
-P 64 \
-n 1000000 \
-q \
-t SET,GET
```

---

### 7. Run with Docker

Build the image:

```bash
docker build -t boson .
```

Run the container:

```bash
docker run -p 6380:6380 boson
```

Verify connectivity:

```bash
redis-cli -p 6380 ping
```

Expected output:

```text
PONG
```

## Testing

Boson is validated through a combination of unit tests, integration tests, property-based tests, protocol compliance checks, and performance stress testing.

### Test Coverage

| Category                  | Description                                                                                                     | Location                    |
| ------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------- |
| Unit Tests                | RESP codec encoding/decoding, command parsing, datastore operations, expiration handling, and utility functions | `src/*.rs` (`#[cfg(test)]`) |
| Integration Tests         | Full server startup, TCP communication, RESP command execution, and end-to-end verification                     | `tests/integration/`        |
| Property-Based Tests      | Randomized input generation using `proptest` to verify codec round-trips and datastore invariants               | `tests/property/`           |
| Benchmark Validation      | Throughput, latency, pipelining, concurrency scaling, and Redis compatibility verification                      | Manual benchmark suite      |
| Protocol Compliance Tests | RESP2 frame validation, malformed request handling, frame-size limits, and parser correctness                   | Unit + Integration Tests    |

---

### Correctness Verification

| Test                       | Condition                                       | Result                                          |
| -------------------------- | ----------------------------------------------- | ----------------------------------------------- |
| Connection Limit           | Attempt connection #1001 with limit set to 1000 | ✅ `ERR max clients reached`                    |
| Graceful Shutdown          | Send `SIGINT` while handling active requests    | ✅ All in-flight requests complete before exit  |
| Idle Timeout               | Leave a connection idle for 60 seconds          | ✅ Connection closed cleanly                    |
| Frame Size Limit           | Send frame larger than 512 MB                   | ✅ `ERR frame too large` returned               |
| Array Length Limit         | Send array larger than 1,000,000 elements       | ✅ Rejected before allocation                   |
| RESP Parser Validation     | Malformed RESP frames                           | ✅ Error response generated                     |
| Missing Key Lookup         | `GET` on non-existent key                       | ✅ RESP Null Bulk String returned               |
| Key Expiration             | Access expired key after TTL elapsed            | ✅ Key removed and treated as missing           |
| Pipeline Depth 16          | `redis-benchmark -P 16`                         | ✅ Correct responses under pipelined load       |
| Pipeline Depth 64          | `redis-benchmark -P 64`                         | ✅ Correct responses under heavy pipelined load |
| Concurrent Clients         | 50–200 simultaneous benchmark clients           | ✅ Stable throughput and correctness            |
| Redis Client Compatibility | `redis-cli` interoperability                    | ✅ Compatible with standard Redis tooling       |

---

### Continuous Integration

Every push and pull request automatically executes:

| Stage                         | Purpose                              |
| ----------------------------- | ------------------------------------ |
| `cargo fmt --check`           | Formatting verification              |
| `cargo clippy -- -D warnings` | Static analysis and lint enforcement |
| `cargo test --all`            | Unit and integration testing         |
| `cargo build --release`       | Release build validation             |

All checks must pass before changes are merged.

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration

# Property tests
cargo test --test prop

# With output
cargo test -- --nocapture

```

## License

```
MIT License

Copyright (c) 2026 Ayush Verma

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

```
