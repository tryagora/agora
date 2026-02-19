# agora test suite

comprehensive testing suite for agora including load tests, smoke tests, and chaos tests with docker log monitoring.

## quick start

```bash
# make sure services are running
docker-compose up -d

# run smoke tests (quick validation)
python tests/run_tests.py smoke

# run load tests
python tests/run_tests.py load

# run chaos tests (try to break things)
python tests/run_tests.py chaos

# run delay/timing tests
python tests/run_tests.py delay

# run everything
python tests/run_tests.py all
```

on windows, you can also use the batch file:
```batch
tests\run_tests.bat smoke
```

## test types

### 1. smoke tests (smoke_test.py)
quick validation that basic operations work:
- api health check
- user registration
- server creation
- channel creation
- message sending
- sync endpoint

**time:** ~10 seconds

### 2. load tests (agora_test_suite.py)
full performance testing:
- **registration spam**: create many users concurrently
- **server creation spam**: create many servers
- **join/leave spam**: rapid join/leave operations
- **message spam**: send many messages
- **mixed load**: realistic concurrent usage

**metrics tracked:**
- latency (avg, median, min, max, stddev)
- success rate
- error count
- throughput (ops/sec)

### 3. chaos tests (chaos_test.py)
deliberately try to break things:
- rapid-fire registrations
- concurrent server creation
- rapid join/leave cycles
- malformed request handling
- race condition testing

### 4. delay tests (delay_test.py)
test timing-specific scenarios:
- voice disconnect propagation
- message sync delays
- server list refresh timing
- channel creation delays
- latency under concurrent load

## configuration

### environment variables

```bash
# service endpoints
export api_url=http://localhost:3000
export homeserver=http://localhost:8448

# load test parameters
export registration_count=100
export server_count=50
export load_duration=120
export concurrent_users=20
```

### test parameters

**smoke test:**
- single user flow through all operations

**load test (default):**
- 10 user registrations
- 20 server creations
- 50 join/leave iterations
- 100 messages per room (3 rooms)
- 60 seconds mixed load with 10 concurrent users

**load test (stress):**
```bash
python tests/run_tests.py load --registration-count 500 --server-count 200 --load-duration 300 --concurrent-users 50
```

**chaos test:**
- 50 rapid registrations
- concurrent server creation from multiple users
- 20 rapid join/leave cycles per user
- malformed requests
- race condition attempts

## python direct usage

```bash
# install dependencies
pip install aiohttp

# run specific test with custom parameters
python tests/load-tests/agora_test_suite.py \
    --api-url http://localhost:3000 \
    --homeserver http://localhost:8448 \
    --registration-count 100 \
    --server-count 50 \
    --load-duration 120 \
    --concurrent-users 20 \
    --monitor-docker

# skip specific tests
python tests/load-tests/agora_test_suite.py \
    --skip registration servers \
    --load-duration 60

# just smoke test
python tests/load-tests/smoke_test.py http://localhost:3000

# just chaos
python tests/load-tests/chaos_test.py http://localhost:3000

# just delay tests
python tests/load-tests/delay_test.py http://localhost:3000
```

## docker log monitoring

the test suite can monitor docker logs during tests to catch errors:

```bash
# monitor logs during load test
python tests/run_tests.py load --monitor-docker

# the python script always supports log monitoring
python tests/run_tests.py load
```

logs are collected for:
- `agora_conduit` (matrix homeserver)
- `agora_livekit` (voice/video)
- api logs (from test output)

## interpreting results

### success metrics

**acceptable performance:**
- registration: < 500ms avg
- server creation: < 300ms avg
- message sending: < 100ms avg
- sync: < 200ms avg
- success rate: > 95%

**warning signs:**
- success rate < 90%
- latency increasing over time
- dtls/ice errors in livekit logs
- conduit "m_not_found" errors

**critical issues:**
- server crashes (500 errors)
- connection timeouts
- database connection errors
- memory exhaustion

### common issues

**401 unauthorized from livekit:**
- check livekit api key/secret match between backend and livekit.yaml
- verify admin jwt is being generated correctly

**connection timeouts:**
- services may be overloaded
- check docker resource limits
- reduce concurrent user count

**matrix errors (m_not_found, m_forbidden):**
- usually expected during leave operations
- ok if success rate is still high

## ci/cd integration

```yaml
# example github actions
- name: run smoke tests
  run: |
    python tests/run_tests.py smoke
    
- name: run load tests
  run: |
    python tests/run_tests.py load --registration-count 50 --server-count 20 --load-duration 60
```

## adding new tests

1. add test method to `LoadTester` class in `agora_test_suite.py`
2. add configuration option to argparse
3. call from `run_test_suite`
4. update this readme

example:
```python
async def test_my_feature(self, http_session, count):
    print(f"\ntesting my feature ({count} operations)...")
    metric = TestMetrics(operation="my_feature", start_time=time.time())
    # ... test code ...
    self.session.add_metric(metric)
```

## troubleshooting

**"api is not responding"**
- start backend: `cargo run --manifest-path backend/api/cargo.toml`
- check api_url environment variable

**"conduit is not responding"**
- start docker services: `docker-compose up -d`
- check conduit is healthy: `docker-compose ps`

**import errors**
- install python dependencies: `pip install aiohttp`

**permission denied on run_tests.py**
- make executable: `chmod +x tests/run_tests.py` (linux/mac)
- on windows, use `python tests/run_tests.py` instead

## test data cleanup

tests create real users and servers. to clean up:

```bash
# stop and remove containers (destroys data)
docker-compose down -v

# or manually delete test users via api
# (todo: add cleanup endpoint)
```

## performance baselines

measured on:
- cpu: intel i7-12700h
- ram: 32gb
- docker desktop with wsl2
- local development setup

| operation | target avg | max |
|-----------|-----------|-----|
| registration | 200ms | 1000ms |
| login | 150ms | 500ms |
| server creation | 300ms | 1000ms |
| channel creation | 200ms | 800ms |
| message send | 50ms | 200ms |
| sync | 100ms | 500ms |
| join room | 200ms | 1000ms |
| leave room | 100ms | 500ms |
