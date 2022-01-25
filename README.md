# T0
An experimental high performance real-time in-memory distributed timeseries database.

In current, it is just a demo.

## Targets
- kindly inspire by InfluxDB IOx, Google Monarch, ScyllaDB and much more open-source projects.
- use new experimental features to make the fastest insertion / query TSDB
- OLAP / OLTP fusion
  - support massive get / set operation for recent mutable data.
  - immutable old data & use Apache Arrow / Parquet ecosystem
    - boost analytical query (push-down more piplinable calculator with SIMD)
    - support zero-copy transportation to easily integrate to other analytical project
- load-on-demand component & easily scaling on component level
- optional WAL / distribution backup
- columnar format & rich-type column


## Core Concepts
- Data must has a timestamp
- Tracking data transmutation of unique object in various of times (timeseries)
  - Has a set of labels to identify unique object in different timestamps
  - Has a set of data(scalar) that are computable
- Data distribution is related to the time
  - Can always find a minimal size of time interval (unit), in this level, data in each fragment of a single unique object are continuous
  - Can always find a large enough size of timer interval, in this level, both unique object and data are sparse
- Insertions are always happened in recent rather than before
- OLTP / OLAP fusion
  - Recent data for single data query: alerting, monitoring
  - History data for analysis: attribution analysis, machine learning
- Data aggregation are always group by timestamp
- Data can be merged with neighboring data on time
- No transaction required

## Installation
### Dependencies
- rustc (1.60.0-nightly+)
- clang (13.0.0+)
### Build
```
git clone https://github.com/Homebrew-TSDB-Club/t0.git
cargo build --release
```

### Get Started
```
./target/release/t0 --address=0.0.0.0:1108 --server-cores=24 --storage-cores=16
```

### Example
[Prometheus Remote Write](https://github.com/Homebrew-TSDB-Club/T0/blob/main/tests/prometheus/src/remote_write.rs)

## Features
- [ ] core
  - [ ] coroutine runtime
    - [x] CPU core-affinity coroutine runtime
    - [x] epoll I/O
    - [ ] Linux aio / io_uring API
    - [ ] query language
      - [x] uniform logical expression
      - [x] PromQL parser
      - [ ] custom query language syntax & parser
    - [x] asynchronous & multiplexing server
      - [x] Tokio(work-stealing coroutine) based HTTP/2(gRPC) server
      - [x] core-affinity coroutine based HTTP/2(gRPC) server
      - [ ] FlatBuffers over QUIC
  - [x] function level tracing
  - [ ] load-on-demand component: insertion / storage / query / config
  - [ ] decentralized federation deployment
  - [ ] self metrics
- [ ] insertion
  - [x] Prometheus remote write protocol
  - [ ] custom protocol over FlatBuffers
- [ ] storage
  - [x] column-oriented
  - [x] rich-type column
  - [x] chunking data
  - [ ] data format
    - [x] mutable in-memory chunk with custom format
    - [ ] immutable in-memory Apache Arrow format
    - [ ] immutable Apache Parquet format file storage 
  - [x] shared-nothing insertion based on CPU core-affinity coroutine
  - [x] query calculator push-down
    - [x] projection
    - [x] filter
    - [x] time range
    - [x] limit
    - [ ] pipeline compute
  - [ ] data archive pipeline: mutable -> immutable -> file
- [ ] query
  - [x] basic PromQL support
  - [ ] transport
    - [x] Apache Arrow Flight over HTTP/2(gRPC)
    - [ ] DPDK / RDMA
  - [x] shared-nothing mutable chunk query
