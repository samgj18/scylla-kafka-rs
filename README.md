# Scylla & Kafka

This is just a simple toy project to see how Scylla and Kafka can be used with rust clients.

## Prerequisites

- Docker
- Docker Compose
- Rust


## Running

```bash
docker-compose up -d && docker exec scylla-node1 cqlsh -f /migrations.txt
```

In one terminal:

```bash
scylla-kafka-rs read-mutant -g "my-group"
```

In another terminal:

```bash
scylla-kafka-rs add-mutant -n Michael -l Seinfield -a "21 Wallabe Street"
```


