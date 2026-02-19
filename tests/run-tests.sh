#!/bin/bash
#
# Agora Test Suite Runner
# Orchestrates load tests with Docker log monitoring
#

set -e

API_URL="${API_URL:-http://localhost:3000}"
HOMESERVER="${HOMESERVER:-http://localhost:8448}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🧪 AGORA TEST SUITE"
echo "===================="
echo "API URL: $API_URL"
echo "Homeserver: $HOMESERVER"
echo ""

# Function to check if services are running
check_services() {
    echo "🔍 Checking if services are running..."
    
    # Check API
    if curl -s "$API_URL/health" > /dev/null 2>&1; then
        echo "   ✅ API is responding"
    else
        echo "   ❌ API is not responding at $API_URL"
        echo "      Start it with: cargo run --manifest-path backend/api/Cargo.toml"
        exit 1
    fi
    
    # Check Conduit
    if curl -s "$HOMESERVER/_matrix/client/versions" > /dev/null 2>&1; then
        echo "   ✅ Conduit is responding"
    else
        echo "   ❌ Conduit is not responding at $HOMESERVER"
        echo "      Start it with: docker-compose up conduit"
        exit 1
    fi
    
    echo ""
}

# Function to watch Docker logs in background
watch_logs() {
    echo "🐳 Starting Docker log monitoring..."
    
    # Create temporary directory for log files
    LOG_DIR=$(mktemp -d)
    echo "   Log files: $LOG_DIR"
    
    # Start log collection in background
    docker logs -f --tail=50 agora_conduit > "$LOG_DIR/conduit.log" 2>&1 &
    CONDUIT_PID=$!
    
    docker logs -f --tail=50 agora_livekit > "$LOG_DIR/livekit.log" 2>&1 &
    LIVEKIT_PID=$!
    
    # Return log directory and PIDs
    echo "$LOG_DIR $CONDUIT_PID $LIVEKIT_PID"
}

# Function to stop log watching
stop_logs() {
    local LOG_DIR=$1
    local CONDUIT_PID=$2
    local LIVEKIT_PID=$3
    
    echo ""
    echo "📋 Collecting final logs..."
    
    # Kill log processes
    kill $CONDUIT_PID $LIVEKIT_PID 2>/dev/null || true
    
    # Show recent errors
    echo ""
    echo "🔴 Recent Conduit errors:"
    grep -i "error\|warn\|panic" "$LOG_DIR/conduit.log" | tail -20 || echo "   (no errors found)"
    
    echo ""
    echo "🔴 Recent LiveKit errors:"
    grep -i "error\|warn" "$LOG_DIR/livekit.log" | tail -20 || echo "   (no errors found)"
    
    # Cleanup
    rm -rf "$LOG_DIR"
}

# Function to run smoke test
run_smoke() {
    echo ""
    echo "🧪 Running Smoke Tests..."
    echo "-------------------------"
    python3 "$SCRIPT_DIR/smoke_test.py" "$API_URL"
}

# Function to run load tests
run_load() {
    echo ""
    echo "📊 Running Load Tests..."
    echo "-------------------------"
    
    # Parse arguments
    local ARGS=""
    if [ -n "$REGISTRATION_COUNT" ]; then
        ARGS="$ARGS --registration-count $REGISTRATION_COUNT"
    fi
    if [ -n "$SERVER_COUNT" ]; then
        ARGS="$ARGS --server-count $SERVER_COUNT"
    fi
    if [ -n "$LOAD_DURATION" ]; then
        ARGS="$ARGS --load-duration $LOAD_DURATION"
    fi
    if [ -n "$CONCURRENT_USERS" ]; then
        ARGS="$ARGS --concurrent-users $CONCURRENT_USERS"
    fi
    
    python3 "$SCRIPT_DIR/agora_test_suite.py" \
        --api-url "$API_URL" \
        --homeserver "$HOMESERVER" \
        $ARGS \
        --monitor-docker
}

# Function to run chaos tests
run_chaos() {
    echo ""
    echo "🔥 Running Chaos Tests..."
    echo "-------------------------"
    python3 "$SCRIPT_DIR/chaos_test.py" "$API_URL"
}

# Show usage
usage() {
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Commands:"
    echo "  smoke      - Quick smoke test (default)"
    echo "  load       - Full load test suite"
    echo "  chaos      - Chaos/chaos tests"
    echo "  all        - Run all tests"
    echo ""
    echo "Environment Variables:"
    echo "  API_URL              - API endpoint (default: http://localhost:3000)"
    echo "  HOMESERVER          - Matrix homeserver (default: http://localhost:8448)"
    echo "  REGISTRATION_COUNT  - Number of registrations for load test"
    echo "  SERVER_COUNT        - Number of servers to create"
    echo "  LOAD_DURATION       - Mixed load test duration in seconds"
    echo "  CONCURRENT_USERS    - Number of concurrent users"
    echo ""
    echo "Examples:"
    echo "  $0 smoke"
    echo "  $0 load"
    echo "  REGISTRATION_COUNT=100 SERVER_COUNT=50 $0 load"
    echo "  $0 all"
}

# Main
main() {
    local COMMAND=${1:-smoke}
    
    case $COMMAND in
        smoke)
            check_services
            run_smoke
            ;;
        load)
            check_services
            run_load
            ;;
        chaos)
            check_services
            run_chaos
            ;;
        all)
            check_services
            run_smoke && run_load && run_chaos
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            echo "Unknown command: $COMMAND"
            usage
            exit 1
            ;;
    esac
    
    echo ""
    echo "✅ Test suite complete!"
}

main "$@"
