# Metrics

Starknet Devnet can expose Prometheus-compatible metrics for monitoring and observability. This feature allows you to track RPC calls, block creation, transactions, and upstream forking calls.

## Enabling Metrics

To enable metrics, start Devnet with the `--metrics-host` parameter:

```bash
$ starknet-devnet --metrics-host <IP_ADDRESS>
```

By default, the metrics server will listen on port `9090`. You can customize the port with the `--metrics-port` parameter:

```bash
$ starknet-devnet --metrics-host 127.0.0.1 --metrics-port 8080
```

Or using environment variables:

```bash
$ METRICS_HOST=127.0.0.1 METRICS_PORT=8080 starknet-devnet
```

If running with Docker:

```bash
$ docker run -e METRICS_HOST=0.0.0.0 -e METRICS_PORT=9090 -p 9090:9090 shardlabs/starknet-devnet-rs
```

## Accessing Metrics

Once the metrics server is running, you can access the metrics endpoint at:

```
http://<metrics-host>:<metrics-port>/metrics
```

For example:

```bash
$ curl http://127.0.0.1:9090/metrics
```

The metrics are exposed in Prometheus text format, which can be scraped by Prometheus or other compatible monitoring systems.

## Available Metrics

### RPC Metrics

#### `rpc_call_duration_seconds`

**Type:** Histogram

**Description:** Duration of RPC calls in seconds

**Labels:**

- `method`: The RPC method name (e.g., `starknet_getBlockWithTxs`, `starknet_call`)

**Buckets:** 0.00005, 0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.015, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0 seconds

#### `rpc_call_count`

**Type:** Counter

**Description:** Total number of RPC calls

**Labels:**

- `method`: The RPC method name
- `status`: Either `success` or `error`

### Starknet Core Metrics

#### `starknet_transaction_count`

**Type:** Counter

**Description:** Total number of transactions in Starknet

This counter is incremented when a transaction is added to the network and decremented when blocks are aborted.

#### `starknet_block_count`

**Type:** Counter

**Description:** Total number of blocks in Starknet

This counter is incremented when a new block is created and decremented when blocks are aborted.

#### `starknet_block_creation_duration_seconds`

**Type:** Histogram

**Description:** Duration of block creation in seconds

**Buckets:** 0.00005, 0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01 seconds

This metric tracks how long it takes to generate a new block and transition from the pre-confirmed state.

### Upstream Forking Metrics

These metrics are only relevant when running Devnet in [forking mode](./forking.md).

#### `starknet_upstream_call_duration_seconds`

**Type:** Histogram

**Description:** Duration of upstream forking origin calls in seconds

**Labels:**

- `method`: The RPC method called on the upstream network
- `status`: Either `success` or `error`

**Buckets:** 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0 seconds

#### `starknet_upstream_call_count`

**Type:** Counter

**Description:** Total number of upstream forking origin calls

**Labels:**

- `method`: The RPC method called on the upstream network
- `status`: Either `success` or `error`

These metrics help monitor the performance and reliability of interactions with the forked network.

## Integrating with Prometheus

To scrape these metrics with Prometheus, add the following job to your `prometheus.yml` configuration:

```yaml
scrape_configs:
  - job_name: 'starknet-devnet'
    static_configs:
      - targets: ['localhost:9090']
```

Adjust the target address to match your Devnet metrics server configuration.

## Example Queries

Here are some example PromQL queries you can use:

### Average RPC call duration by method

```promql
rate(rpc_call_duration_seconds_sum[5m]) / rate(rpc_call_duration_seconds_count[5m])
```

### RPC call rate by method

```promql
rate(rpc_call_count[5m])
```

### RPC error rate

```promql
rate(rpc_call_count{status="error"}[5m]) / rate(rpc_call_count[5m])
```

### Block creation rate

```promql
rate(starknet_block_count[5m])
```

### Transaction throughput

```promql
rate(starknet_transaction_count[5m])
```

### Upstream call error rate (forking mode)

```promql
rate(starknet_upstream_call_count{status="error"}[5m]) / rate(starknet_upstream_call_count[5m])
```

### 95th percentile block creation time

```promql
histogram_quantile(0.95, rate(starknet_block_creation_duration_seconds_bucket[5m]))
```

## Visualization with Grafana

You can visualize these metrics using Grafana by:

1. Adding Prometheus as a data source
2. Creating dashboards with panels using the PromQL queries above
3. Setting up alerts based on metric thresholds
