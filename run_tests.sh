#!/bin/bash

./strfry/plugins/tests/run_tests.sh &
DENOTEST_PID=$!

( cd ./event_deleter && cargo test ) &
RUSTTEST_PID=$!

wait $DENOTEST_PID
DENOTEST_STATUS=$?

wait $RUSTTEST_PID
RUSTTEST_STATUS=$?

pkill -P $$

if [ $DENOTEST_STATUS -ne 0 ]; then
    echo "Deno tests failed."
fi

if [ $RUSTTEST_STATUS -ne 0 ]; then
    echo "Rust tests failed."
fi

if [ $DENOTEST_STATUS -ne 0 ] || [ $RUSTTEST_STATUS -ne 0 ]; then
    exit 1
else
    echo "All tests passed."
    exit 0
fi
