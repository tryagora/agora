# Agora Test Suite

Comprehensive testing suite for Agora including load tests, smoke tests, and chaos tests with Docker log monitoring.

## Quick Start

```bash
# Make sure services are running
docker-compose up -d

# Run smoke tests (quick validation)
./tests/run-tests.sh smoke

# Run load tests
./tests/run-tests.sh load

# Run chaos tests (try to break things)
./tests/run-tests.sh chaos

# Run everything
./tests/run-tests.sh all
```

## Test Types

### 1. Smoke Tests (`smoke_test.py`)
Quick validation that basic operations work:
- API health check
- User registration
- Server creation
- Channel creation
- Message sending
- Sync endpoint

**Time:** ~10 seconds

### 2. Load Tests (`agora_test_suite.py`)
Full performance testing:
- **Registration Spam**: Create many users concurrently
- **Server Creation Spam**: Create many servers
- **Join/Leave Spam**: Rapid join/leave operations
- **Message Spam**: Send many messages
- **Mixed Load**: Realistic concurrent usage

**Metrics tracked:**
- Latency (avg, median, min, max, stddev)
- Success rate
- Error count
- Throughput (ops/sec)

### 3. Chaos Tests (`chaos_test.py`)
Deliberately try to break things:
- Rapid-fire registrations
- Concurrent server creation
- Rapid join/leave cycles
- Malformed request handling
- Race condition testing

## Configuration

### Environment Variables

```bash
# Service endpoints
export API_URL=http://localhost:3000
export HOMESERVER=http://localhost:8448

# Load test parameters
export REGISTRATION_COUNT=100
export SERVER_COUNT=50
export LOAD_DURATION=120
export CONCURRENT_USERS=20
```

### Test Parameters

**Smoke Test:**
- Single user flow through all operations

**Load Test (default):**
- 10 user registrations
- 20 server creations
- 50 join/leave iterations
- 100 messages per room (3 rooms)
- 60 seconds mixed load with 10 concurrent users

**Load Test (stress):**
```bash
REGISTRATION_COUNT=500 SERVER_COUNT=200 LOAD_DURATION=300 CONCURRENT_USERS=50 ./tests/run-tests.sh load
```

**Chaos Test:**
- 50 rapid registrations
- Concurrent server creation from multiple users
- 20 rapid join/leave cycles per user
- Malformed requests
- Race condition attempts

## Python Direct Usage

```bash
# Install dependencies
pip install aiohttp

# Run specific test with custom parameters
python3 tests/load-tests/agora_test_suite.py \
    --api-url http://localhost:3000 \
    --homeserver http://localhost:8448 \
    --registration-count 100 \
    --server-count 50 \
    --load-duration 120 \
    --concurrent-users 20 \
    --monitor-docker

# Skip specific tests
python3 tests/load-tests/agora_test_suite.py \
    --skip registration servers \
    --load-duration 60

# Just smoke test
python3 tests/load-tests/smoke_test.py http://localhost:3000

# Just chaos
python3 tests/load-tests/chaos_test.py http://localhost:3000
```

## Docker Log Monitoring

The test suite can monitor Docker logs during tests to catch errors:

```bash
# Monitor logs during load test
python3 tests/load-tests/agora_test_suite.py --monitor-docker

# The shell script always monitors logs
./tests/run-tests.sh load
```

Logs are collected for:
- `agora_conduit` (Matrix homeserver)
- `agora_livekit` (Voice/video)
- API logs (from test output)

## Interpreting Results

### Success Metrics

**Acceptable performance:**
- Registration: < 500ms avg
- Server creation: < 300ms avg
- Message sending: < 100ms avg
- Sync: < 200ms avg
- Success rate: > 95%

**Warning signs:**
- Success rate < 90%
- Latency increasing over time
- DTLS/ICE errors in LiveKit logs
- Conduit "M_NOT_FOUND" errors

**Critical issues:**
- Server crashes (500 errors)
- Connection timeouts
- Database connection errors
- Memory exhaustion

### Common Issues

**401 Unauthorized from LiveKit:**
- Check LiveKit API key/secret match between backend and livekit.yaml
- Verify admin JWT is being generated correctly

**Connection timeouts:**
- Services may be overloaded
- Check Docker resource limits
- Reduce concurrent user count

**Matrix errors (M_NOT_FOUND, M_FORBIDDEN):**
- Usually expected during leave operations
- OK if success rate is still high

## CI/CD Integration

```yaml
# Example GitHub Actions
- name: Run Smoke Tests
  run: |
    ./tests/run-tests.sh smoke
    
- name: Run Load Tests
  env:
    REGISTRATION_COUNT: 50
    SERVER_COUNT: 20
    LOAD_DURATION: 60
  run: |
    ./tests/run-tests.sh load
```

## Adding New Tests

1. Add test method to `LoadTester` class in `agora_test_suite.py`
2. Add configuration option to argparse
3. Call from `run_test_suite`
4. Update this README

Example:
```python
async def test_my_feature(self, http_session, count):
    print(f"\n📊 Testing my feature ({count} operations)...")
    metric = TestMetrics(operation="my_feature", start_time=time.time())
    # ... test code ...
    self.session.add_metric(metric)
```

## Troubleshooting

**"API is not responding"**
- Start backend: `cargo run --manifest-path backend/api/Cargo.toml`
- Check API_URL environment variable

**"Conduit is not responding"**
- Start Docker services: `docker-compose up -d`
- Check conduit is healthy: `docker-compose ps`

**Import errors**
- Install Python dependencies: `pip install aiohttp`

**Permission denied on run-tests.sh**
- Make executable: `chmod +x tests/run-tests.sh`

## Test Data Cleanup

Tests create real users and servers. To clean up:

```bash
# Stop and remove containers (destroys data)
docker-compose down -v

# Or manually delete test users via API
# (TODO: add cleanup endpoint)
```

## Performance Baselines

Measured on:
- CPU: Intel i7-12700H
- RAM: 32GB
- Docker Desktop with WSL2
- Local development setup

| Operation | Target Avg | Max |
|-----------|-----------|-----|
| Registration | 200ms | 1000ms |
| Login | 150ms | 500ms |
| Server Creation | 300ms | 1000ms |
| Channel Creation | 200ms | 800ms |
| Message Send | 50ms | 200ms |
| Sync | 100ms | 500ms |
| Join Room | 200ms | 1000ms |
| Leave Room | 100ms | 500ms |
