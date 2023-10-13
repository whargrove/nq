# nq

A Rust implementation of the networkQuality server spec.

## Build

```bash
docker build -t nq .
```

## Run

```bash
docker run -e PORT=8080 -p 8080:8080 --name nq nq
```

## Manual Test

```bash
curl --http2-prior-knowledge -vvv http://nq.kentave.net:8080/api/v1/config | jq .
```

## WIP: Using `networkQuality`

_This doesn't work yet! It fails with a timeout._

```bash
networkQuality -k -C http://nq.kentave.net:8080/api/v1/config
```
