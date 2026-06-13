# boson-rs Benchmarks

> Comprehensive performance analysis comparing boson-rs against Redis using `redis-benchmark`.

---

## Table of Contents

- [Benchmark Environment](#benchmark-environment)

- [Performance Highlights](#performance-highlights)

- [Non-Pipelined Results](#non-pipelined-results)

- [Client Scaling Results](#client-scaling-results)

- [Pipelined Results](#pipelined-results)
- [Redis Comparison Summary](#redis-comparison-summary)

- [Safety & Reliability Features](#safety--reliability-features)

---

## Benchmark Environment

All benchmarks use `redis-benchmark` against a release binary (`cargo build --release`) on localhost. Redis benchmarks use the system Redis instance on the default port for comparison. Results represent the median of multiple runs unless noted.

### Benchmark Environment

| Component        | Value                          |
| ---------------- | ------------------------------ |
| Operating System | macOS Tahoe Beta 26.6          |
| CPU              | Apple M2 ARM64 (Apple Silicon) |
| RAM              | 16 GB Unified Memory           |
| Rust Version     | rustc 1.95.0                   |
| Redis Version    | Redis server (8.8.0)           |
| Benchmark Tool   | `redis-benchmark`              |
| Boson Port       | `6380`                         |
| Redis Port       | `6379`                         |
| Build Profile    | `release`                      |
| Runtime          | Tokio                          |
| Protocol         | RESP2                          |
| Store Backend    | DashMap + AHash                |
| Connection Limit | `1000`                         |

## Performance Highlights

| Metric                        |                          Value |
| ----------------------------- | -----------------------------: |
| Peak GET Throughput           |            **1,555,210 req/s** |
| Peak SET Throughput           |            **1,418,440 req/s** |
| Median GET Throughput         |            **1,506,024 req/s** |
| Median SET Throughput         |            **1,412,429 req/s** |
| Non-Pipelined Throughput      |                    ~156k req/s |
| Pipelining Speedup            |                         ~9–10× |
| Connection Limit              |                          1,000 |
| Protocol                      |               RESP2 Compatible |
| Runtime                       |                          Tokio |
| Store Engine                  |                DashMap + AHash |
| Peak Benchmark Configuration  | 100 Clients, Pipeline Depth 64 |
| p50 Latency @ Peak Throughput |                    ~3.7–3.9 ms |

### Non-Pipelined Results

**Benchmark Configuration**

```bash
redis-benchmark -n 100000 -c 50
```

| Command     | Redis (req/s) | Boson (req/s) | Delta |
| ----------- | ------------: | ------------: | ----: |
| PING_INLINE |       157,978 |       151,976 | −3.8% |
| PING_MBULK  |       161,290 |       156,495 | −3.0% |
| SET         |       164,474 |       156,495 | −4.9% |
| GET         |       161,812 |       156,740 | −3.1% |
| INCR        |       164,745 |       157,480 | −4.4% |

> **Summary:** Boson achieves approximately **95–97% of Redis throughput** in non-pipelined workloads, staying within **3–5%** of Redis across all tested commands.

---

### Client Scaling Results

**Benchmark Configuration**

```bash
redis-benchmark -n 100000 -c <clients> -q -t PING,SET,GET,INCR
```

Median throughput across three benchmark runs for each client count.

| Command         |       c = 50 |      c = 100 |      c = 200 |
| --------------- | -----------: | -----------: | -----------: |
| PING_INLINE     |      150,830 |      157,233 |      153,374 |
| PING_MBULK      |      156,495 |      156,495 |      153,610 |
| SET             |      156,495 |      156,006 |      152,905 |
| GET             |      156,495 |      156,495 |      152,905 |
| INCR            |      157,978 |      157,729 |      153,846 |
| **p50 Latency** | **0.167 ms** | **0.327 ms** | **0.647 ms** |

> **Observation:** Throughput remains remarkably stable as concurrency increases from **50 → 200 clients**, varying by less than ~3%. Meanwhile, median latency scales approximately linearly with client count.

---

## Pipelined Results

#### Pipeline Depth 16 — 50 Clients

**Benchmark Configuration**

```bash
redis-benchmark \
-h 127.0.0.1 \
-p 6380 \
-n 100000 \
-c 50 \
-P 16 \
-q \
-t SET,GET
```

| Command     | Boson (req/s) | Redis (req/s) | Ratio |
| ----------- | ------------: | ------------: | ----: |
| SET         |       884,956 |     1,190,476 | 0.74× |
| GET         |     1,063,829 |     1,754,386 | 0.61× |
| **SET p50** |  **0.551 ms** |             — |     — |
| **GET p50** |  **0.431 ms** |             — |     — |

#### Pipeline Depth 64 — 50 Clients

**Benchmark Configuration**

```bash
redis-benchmark \
-h 127.0.0.1 \
-p 6380 \
-n 100000 \
-c 50 \
-P 64 \
-q \
-t SET,GET
```

| Command | Boson Median (req/s) | Boson Best (req/s) | Redis (req/s) | Ratio (Best) |
| ------- | -------------------: | -----------------: | ------------: | -----------: |
| SET     |            1,123,775 |          1,266,228 |     1,587,809 |        0.80× |
| GET     |            1,219,902 |          1,471,059 |     2,858,057 |        0.51× |

> **Observation:** Increasing pipeline depth from **16 → 64** pushes Boson beyond **1.4 million requests/sec**, demonstrating efficient handling of large batches of in-flight commands. The largest gains are observed in GET-heavy workloads where network overhead becomes less significant.

---

### Pipeline Depth 64 — 100 Clients (Final Results)

**Benchmark Configuration**

```bash
redis-benchmark \
-h 127.0.0.1 \
-p <port> \
-n 1000000 \
-c 100 \
-P 64 \
-q \
-t SET,GET
```

#### Redis

| Command |  Best (req/s) | Median (req/s) | p50 Latency |
| ------- | ------------: | -------------: | ----------: |
| SET     | **1,730,104** |      1,724,138 |    ~3.41 ms |
| GET     | **2,610,966** |      2,604,167 |    ~2.21 ms |

#### Boson

| Command |  Best (req/s) | Median (req/s) | p50 Latency |
| ------- | ------------: | -------------: | ----------: |
| SET     | **1,418,440** |      1,412,429 |    ~3.91 ms |
| GET     | **1,555,210** |      1,506,024 |    ~3.71 ms |

> These results were collected on an Apple M2 system (16 GB RAM) running release builds with a pipeline depth of 64 and 100 concurrent clients.

### Redis Comparison Summary

| Operation                      | Redis (req/s) | Boson (req/s) |     Ratio |
| ------------------------------ | ------------: | ------------: | --------: |
| PING (Non-Pipelined)           |       157,978 |       151,976 |     0.96× |
| SET (Non-Pipelined)            |       164,474 |       156,495 |     0.95× |
| GET (Non-Pipelined)            |       161,812 |       156,740 |     0.97× |
| INCR (Non-Pipelined)           |       164,745 |       157,480 |     0.96× |
| SET (Pipeline 64, 100 Clients) |     1,730,104 | **1,418,440** | **0.82×** |
| GET (Pipeline 64, 100 Clients) |     2,610,966 | **1,555,210** | **0.60×** |

> Boson achieves approximately **95–97% of Redis throughput** in non-pipelined workloads and reaches **82% of Redis SET throughput** and **60% of Redis GET throughput** under heavily pipelined workloads.

## Safety & Reliability Features

### Connection Limits

A `tokio::sync::Semaphore` with 1,000 permits gates every accepted connection. If no permit is available, the connection is accepted at the TCP level (to consume the OS queue slot) and immediately rejected with a RESP error response. The permit is released via an RAII guard when the connection task exits.

### Graceful Shutdown

A `CancellationToken` is shared between the accept loop and all connection tasks. On SIGINT or SIGTERM, the token is cancelled. The accept loop exits its `select!` branch, stops accepting, and waits for the `JoinSet` to drain. Each connection task checks the token on idle to decide whether to hang up.

### Frame Size and Array Guards

The `Decoder` implementation checks the declared frame size before allocating and returns an error if it exceeds the configured limit. Array length is checked before the element vector is allocated. Both checks happen in the codec layer, before any command parsing.
