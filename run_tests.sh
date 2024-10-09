#!/bin/bash

run_test() {
    local description="$1"
    shift
    echo "Running $description..."

    "$@"
    local status=$?

    if [ $status -ne 0 ]; then
        echo "❌ $description failed with status code $status."
        exit 1
    else
        echo "✅ $description passed."
    fi
}

run_test "Integration Tests" run_integration_tests.sh
run_test "Deno Tests" ./strfry/plugins/tests/run_deno_tests.sh
run_test "Rust Tests" bash -c "cd ./event_deleter && cargo test --release --lib"

echo "🎉 All tests passed successfully!"
exit 0