#!/bin/bash
# health check script for agora services

set -e

echo "checking agora services..."

# check conduit
echo -n "conduit (matrix homeserver): "
if curl -s http://localhost:8448/_matrix/client/versions > /dev/null 2>&1; then
    echo "✓ running"
else
    echo "✗ not responding"
fi

# check postgres
echo -n "postgresql: "
if nc -z localhost 5432 2>/dev/null; then
    echo "✓ running"
else
    echo "✗ not responding"
fi

# check redis
echo -n "redis: "
if redis-cli ping 2>/dev/null | grep -q "pong"; then
    echo "✓ running"
else
    echo "✗ not responding"
fi

# check livekit
echo -n "livekit: "
if curl -s http://localhost:7880 > /dev/null 2>&1; then
    echo "✓ running"
else
    echo "✗ not responding"
fi

# check axum api
echo -n "agora api: "
if curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo "✓ running"
else
    echo "✗ not responding"
fi

echo "done"
