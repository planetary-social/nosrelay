#!/bin/bash

# Run Deno tests
./strfry/plugins/tests/run_deno_tests.sh &
DENOTEST_PID=$!

# Run Rust tests. In release mode to reuse the build cache.
( cd ./event_deleter && cargo test --release --lib) &
RUSTTEST_PID=$!

# Run Integration tests
run_integration_tests.sh &
INTEGRATIONTEST_PID=$!

# Wait for Deno tests to finish and capture the status
wait $DENOTEST_PID
DENOTEST_STATUS=$?

# Wait for Rust tests to finish and capture the status
wait $RUSTTEST_PID
RUSTTEST_STATUS=$?

# Wait for Integration tests to finish and capture the status
wait $INTEGRATIONTEST_PID
INTEGRATIONTEST_STATUS=$?

# Kill any remaining background processes (optional, for cleanup)
pkill -P $$

# Check test statuses
if [ $DENOTEST_STATUS -ne 0 ]; then
    echo "Deno tests failed."
fi

if [ $RUSTTEST_STATUS -ne 0 ]; then
    echo "Rust tests failed."
fi

if [ $INTEGRATIONTEST_STATUS -ne 0 ]; then
    echo "Integration tests failed."
fi

# Exit with failure if any test failed
if [ $DENOTEST_STATUS -ne 0 ] || [ $RUSTTEST_STATUS -ne 0 ] || [ $INTEGRATIONTEST_STATUS -ne 0 ]; then
    exit 1
else
    echo "All tests passed."
    exit 0
fi
