# Live Bootcamp (Let's Get Rusty) Capstone

An adaptive HTTP load balancer written in Rust, built on [Hyper](https://hyper.rs/) and
[Tokio](https://tokio.rs/). It accepts traffic on one port, forwards it to a pool of backend
workers, health-checks those workers, and switches between load balancing algorithms based on
how busy they are.

```text
                                         ┌─▶ worker :3001
client ──▶ load balancer :3000 ──────────┼─▶ worker :3002
                 │                       └─▶ worker :3003
                 └─ on fixed interval: GET /ping on each worker, then re-pick the algorithm
```

- **Load balancing:** uses round-robin by default, and least-connections when any host exceeds a connection count cutover.
- **Health checks:** pings each worker's `/ping` endpoint concurrently with a short timeout.
- **Adaptive switching:** after each health check, a decision step picks the algorithm by
  comparing each host's connection count to a configurable cutover.
- **Test workers:** the `worker` crate simulates work by sleeping for a random time, with a
  different upper limit per worker, so slow hosts are easy to reproduce.

See [Known limitations](#known-limitations) for the current gaps.

### Advanced Rust techniques

- **`Arc::strong_count` as a connection counter.** Least-connections needs an in-flight count
  per host, but nothing here keeps one explicitly. Each connection and request holds a clone of
  the host's `Arc`, so the reference count rises and falls naturally with traffic. Because the decrement
  happens in `Drop`, it can't be skipped by an early return or a dropped client
  ([`pool.rs`](load-balancer/src/pool.rs)).
- **Type erasure for response bodies.** The proxy returns either a streaming upstream body or an
  empty generated one. Boxing both as `BoxBody<Bytes, hyper::Error>` gives `forward` a single
  return type.
- **Streaming without buffering.** Request and response bodies pass through as Hyper body
  streams rather than being collected into memory. `Client<HttpConnector, Incoming>` carries the
  inbound body type straight through to the upstream request.
- **Structured concurrency with precise matching.** Each health check round spawns its pings
  into a `JoinSet`, wraps each in `tokio::time::timeout`, and waits with `join_all`. Any timeout, transport failure, or bad status falls through to a single _ arm.
  ([`monitoring.rs`](load-balancer/src/monitoring.rs)).
- **Const generics for bounded metrics.** `CircularBuffer<100, u128>` puts the capacity in the
  type, so per-host response time history has a fixed size and never reallocates
  ([`host.rs`](load-balancer/src/host.rs)).
- **Library and binary split.** Both crates keep their logic in a library create behind a thin
  `main.rs`.
  This lets the integration tests run real workers and a load balancer in one process, through a
  `path` dev-dependency.

## Project layout

| Path | Purpose |
| --- | --- |
| `load-balancer/src/lib.rs` | Accept loop and startup |
| `load-balancer/src/host.rs` | Host address and response time history |
| `load-balancer/src/main.rs` | Entry point (with compile time config) |
| `load-balancer/src/pool.rs` | Host pool, algorithm selection, and unit tests |
| `load-balancer/src/monitoring.rs` | Periodic health checks |
| `load-balancer/src/forwarding.rs` | Request proxying and response time recording |
| `load-balancer/tests/` | Integration tests |
| `worker/` | Simulated backend, Dockerfile, and Compose file |
| `generate-load.sh` | Fixed-rate traffic generator |

`load-balancer` and `worker` are separate crates with no shared Cargo workspace, so run `cargo`
commands from inside the relevant directory.

## Local development

### Prerequisites
Docker with Docker Compose, to run the workers in containers

### Run the stack

The balancer listens on port 3000 and expects workers on ports 3001–3003, so make sure those
ports are free.

1. Start the workers:

   ```bash
   cd worker
   docker compose up --build
   ```

2. In a second terminal, start the load balancer:

   ```bash
   cd load-balancer
   cargo run
   ```

3. In a third terminal, send a request:

   ```bash
   curl -i http://localhost:3000/work
   ```

   You should get `200 OK` with the body `Done!`.

Start the workers first. The balancer only adds hosts during a health check, so if it starts
before they're ready, it returns `503` until the next check, up to 10 seconds later.

### Run the tests

```bash
cd load-balancer
cargo test
```

The integration test bind to various 3xxx ports, so stop any running stack first.

## Load testing with `generate-load.sh`

The script sends requests at a fixed rate until you stop it with Ctrl+C. Start the stack first,
then run it from the repository root:

```bash
./generate-load.sh [TPS] [URL]
```

| Argument | Default | Description |
| --- | --- | --- |
| `TPS` | `10` | Requests per second, as a positive integer |
| `URL` | `http://localhost:3000/work` | Target URL |

It prints one status code per response. Responses can arrive out of order, because requests run
concurrently.

| Code | Meaning |
| --- | --- |
| `200` | The worker completed the request |
| `503` | The balancer had no hosts, or forwarding to the chosen host failed |

To summarize a run, write the codes to a file and count them:

```bash
timeout 30 ./generate-load.sh 20 > codes.txt
sort codes.txt | uniq -c
```

Stopping the script can cut off requests in progress. The balancer logs these as
`Error serving connection: hyper::Error(IncompleteMessage)`, which is expected.

### Scenarios to try

Each scenario assumes the stack from [Run the stack](#run-the-stack). The Compose workers have
limits of 100 ms, 1000 ms, and 2000 ms.

**Round-robin under light load**

```bash
./generate-load.sh 10
```

The balancer logs `RoundRobin` after each health check. Requests split evenly: a 25 second run
sent about 84 requests to each worker. Check the distribution with `docker compose logs`, or
by counting `Working for` lines in each worker's output.

**Switching to least-connections**

```bash
./generate-load.sh 40
```

Requests build up on the slower workers until one exceeds the cutover, and the balancer then
logs `LeastConnections`. In one 25 second run, the fastest worker handled about 540 requests 
(because it clears its connections soonest) and the other two about 230 each. Expect the log to
alternate between the two algorithms, since spreading the load can bring the count back below
the cutover by the next check.

**Worker failure**

```bash
# From the worker directory, while the load script is running
docker compose stop worker-3
```

Requests that hit that host will fail, but should begin to succeed after the health checks 
identify the bad host and removes it from the pool.

## Known limitations
- **Algorithm decisions are periodic.** The balancer only re-picks the algorithm during a health
  check, so it can take up to 10 seconds (currently) to react, and short bursts between checks 
  go unseen.
- **Hosts are chosen per connection.** All requests on a keep-alive connection go to the same
  worker. `generate-load.sh` starts a new `curl` for each request, so it isn't affected, but
  clients that reuse connections will be.
- **Response times aren't used yet.** The last 100 response times per host are recorded, but
  decisions are based only on connection counts. They are meant for a not-yet-implemented load 
  balancing algorithms.
- **Load balancer configuration is compile-time only.** Changing a setting means editing `main.rs`
  and rebuilding, and workers must be on `127.0.0.1`.
