# boson-rs Architecture

> Deep dive into boson-rs's system design, protocol implementation, and component breakdown.

---

## Table of Contents

- [High-Level Architecture](#high-level-architecture)
  - [System Overview](#system-overview)
  - [Connection Lifecycle](#connection-lifecycle)
- [Graceful Shutdown Design](#graceful-shutdown-design)
- [Protocol Support](#protocol-support)
- [Supported Commands](#supported-commands)

---

## High-Level Architecture

### System Overview

```
+----------------------------------------------------------------------+
|                           boson-rs Server                            |
+----------------------------------------------------------------------+

                            SIGINT / SIGTERM
                                   |
                                   ▼
                          CancellationToken
                             |         |
                             |         |
                             ▼         ▼

+----------------------------------------------------------------------+
|                           Accept Loop                                |
+----------------------------------------------------------------------+
|                                                                      |
|  TcpListener                                                         |
|       |                                                              |
|       ▼                                                              |
|  accept()                                                            |
|       |                                                              |
|       ▼                                                              |
|  Semaphore (1000 permits)                                            |
|       |                                                              |
|       ▼                                                              |
|  try_acquire_owned()                                                 |
|       |                                                              |
|       ▼                                                              |
|  tokio::spawn() ───────────────────────────────► JoinSet             |
|       |                                      (graceful drain)        |
|       ▼                                                              |
+----------------------------------------------------------------------+

                              |
                              ▼

+----------------------------------------------------------------------+
|                         Connection Handler                           |
+----------------------------------------------------------------------+
|                                                                      |
|  handle()                                                            |
|       |                                                              |
|       ▼                                                              |
|  Framed<TcpStream, RespCodec>                                        |
|       |                                                              |
|       ▼                                                              |
|  RespCodec::decode()                                                 |
|       |                                                              |
|       ▼                                                              |
|  RespValue                                                           |
|       |                                                              |
|       ▼                                                              |
|  dispatch()                                                          |
|       |                                                              |
|       ▼                                                              |
|  parse_command()                                                     |
|       |                                                              |
|       ▼                                                              |
|  execute()                                                           |
|       |                                                              |
|       ▼                                                              |
|  Arc<Store>                                                          |
|       |                                                              |
|       ▼                                                              |
|  Store                                                               |
|       |                                                              |
|       ▼                                                              |
|  DashMap<String, Bytes> (AHash)                                      |
|                                                                      |
|       ▲                                                              |
|       |                                                              |
|  RespCodec::encode()                                                 |
|       ▲                                                              |
|       |                                                              |
|  RespValue (response)                                                |
|                                                                      |
+----------------------------------------------------------------------+
```

### Connection Lifecycle

```
  Client Connect
        |
        ▼
 Acquire Semaphore Permit
        |
        ▼
  Spawn Handler Task
        |
        ▼
  Process Commands
        |
        ▼
 Connection Closed / Shutdown
        |
        ▼
   Handler Exits
        |
        ▼
 Permit Dropped Automatically
```

## Graceful Shutdown Design

```
   SIGINT / SIGTERM
         |
         ▼
   CancellationToken.cancel()
         |
         ▼
   Accept Loop exits
   (select! cancellation branch fires)
         |
         ▼
   No new connections accepted
         |
         ▼
   JoinSet::join_next() drains
   (awaits all in-flight connection tasks)
         |
         ▼
   Each connection task completes
   its current request, then exits
         |
         ▼
   Semaphore permits released
         |
         ▼
   Process exits cleanly
```

The shutdown path ensures no connections are forcibly dropped mid-request. The `CancellationToken` propagates to the accept loop. `JoinSet` provides a structured way to await all outstanding tasks before the process exits, avoiding orphaned background work.

---

## Protocol Support

boson-rs currently implements the RESP2 (Redis Serialization Protocol v2) wire protocol used by Redis clients.

| RESP Type         | Prefix  | Status           | Notes                                        |
| ----------------- | ------- | ---------------- | -------------------------------------------- |
| Simple String     | `+`     | ✅ Supported     | Used for responses such as `OK` and `PONG`   |
| Error             | `-`     | ✅ Supported     | Used for protocol and command errors         |
| Integer           | `:`     | ✅ Supported     | Used for numeric responses such as `INCR`    |
| Bulk String       | `$`     | ✅ Supported     | Used for string values and command arguments |
| Array             | `*`     | ✅ Supported     | Used for command framing and argument lists  |
| Null Bulk String  | `$-1`   | ✅ Supported     | Returned for missing keys                    |
| Empty Bulk String | `$0`    | ✅ Supported     | Encoded as a zero-length bulk string         |
| Empty Array       | `*0`    | ✅ Supported     | Encoded as a zero-length RESP array          |
| Nested Arrays     | `*`     | ✅ Supported     | Supported by the codec implementation        |
| RESP3 Types       | Various | ❌ Not Supported | RESP3 is currently out of scope              |

---

## Supported Commands

### Implemented

| Command        | Syntax                   | Description                                   | Status         |
| -------------- | ------------------------ | --------------------------------------------- | -------------- |
| `PING`         | `PING [message]`         | Returns `PONG` or echoes the provided message | ✅ Implemented |
| `ECHO`         | `ECHO message`           | Returns the provided message unchanged        | ✅ Implemented |
| `SET`          | `SET key value`          | Stores a key-value pair                       | ✅ Implemented |
| `GET`          | `GET key`                | Retrieves the value associated with a key     | ✅ Implemented |
| `DEL`          | `DEL key [key ...]`      | Deletes one or more keys                      | ✅ Implemented |
| `EXISTS`       | `EXISTS key [key ...]`   | Checks whether one or more keys exist         | ✅ Implemented |
| `INCR`         | `INCR key`               | Atomically increments an integer value        | ✅ Implemented |
| `EXPIRE`       | `EXPIRE key seconds`     | Sets a time-to-live (TTL) on a key            | ✅ Implemented |
| `TTL`          | `TTL key`                | Returns the remaining time-to-live of a key   | ✅ Implemented |
| `CONFIG GET`   | `CONFIG GET parameter`   | Returns empty array for now no impl           | ✅ Implemented |
| `COMMAND DOCS` | `COMMAND DOCS [command]` | Returns empty array for now no impl           | ✅ Implemented |

### Planned

| Command     | Syntax                            | Description                                   | Status     |
| ----------- | --------------------------------- | --------------------------------------------- | ---------- |
| `MGET`      | `MGET key [key ...]`              | Retrieves multiple values in a single request | ⏳ Planned |
| `MSET`      | `MSET key value [key value ...]`  | Sets multiple key-value pairs atomically      | ⏳ Planned |
| `LPUSH`     | `LPUSH key value [value ...]`     | Pushes elements to the head of a list         | ⏳ Planned |
| `RPUSH`     | `RPUSH key value [value ...]`     | Pushes elements to the tail of a list         | ⏳ Planned |
| `SUBSCRIBE` | `SUBSCRIBE channel [channel ...]` | Subscribes a client to Pub/Sub channels       | ⏳ Planned |
| `PUBLISH`   | `PUBLISH channel message`         | Publishes a message to a channel              | ⏳ Planned |
| `MULTI`     | `MULTI`                           | Begins a transaction block                    | ⏳ Planned |
| `EXEC`      | `EXEC`                            | Executes queued transaction commands          | ⏳ Planned |
| `INFO`      | `INFO`                            | Returns runtime and server statistics         | ⏳ Planned |
| `FLUSHDB`   | `FLUSHDB`                         | Removes all keys from the database            | ⏳ Planned |
