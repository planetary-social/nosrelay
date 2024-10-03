#!/bin/bash

assert_jsonl_equals() {
  local jsonl_data="$1"
  local expected_data="$2"
  local message="$3"

  local sorted_jsonl_data=$(echo "$jsonl_data" | sort)
  local sorted_expected_data=$(echo "$expected_data" | sort)

  if [[ "$sorted_jsonl_data" == "$sorted_expected_data" ]]; then
    return 0
  else
    echo "Assertion failed: $message"
    echo "Expected:"
    echo "$sorted_expected_data"
    echo "Got:"
    echo "$sorted_jsonl_data"
    return 1
  fi
}


test_status=0

key1=$(nak key generate)
pubkey1=$(nak key public $key1)
event1=$(nak event -q -k 1 -c "content 1" --sec $key1 ws://nosrelay:7777)

key2=$(nak key generate)
pubkey2=$(nak key public $key2)
event2=$(nak event -q -k 1 -c "content 2" --sec $key2 ws://nosrelay:7777)

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
$event2
$event1
EOF
)

# Assert initial events
assert_jsonl_equals "$req" "$expected_req" "Failed to assert initial events"
test_status=$((test_status + $?))

# Send a vanish request pointing to a different relay
vanish_to_another_relay=$(nak event -q -k 62 -c "Delete all my events!" -t relay=wss://notexample.com --sec $key1 ws://nosrelay:7777)

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
$vanish_to_another_relay
$event2
$event1
EOF
)

assert_jsonl_equals "$req" "$expected_req" "Failed after sending vanish request to another relay"
test_status=$((test_status + $?))

# Send a vanish request pointing to this relay specifically
vanish_to_this_relay=$(nak event -q -k 62 -c "Delete all my events!" -t relay=wss://example.com --sec $key1 ws://nosrelay:7777)
sleep 10

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
$event2
EOF
)

assert_jsonl_equals "$req" "$expected_req" "Failed after sending vanish request to this relay"
test_status=$((test_status + $?))

# For the last event, send a global vanish request
vanish_from_all_relays=$(nak event -q -k 62 -c "Delete all my events!" -t relay=ALL_RELAYS --sec $key2 ws://nosrelay:7777)
sleep 10

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
EOF
)

assert_jsonl_equals "$req" "$expected_req" "Failed after sending vanish request to all relays"
test_status=$((test_status + $?))

if [ $test_status -eq 0 ]; then
  echo "Integration tests passed!"
else
  echo "Integration tests failed."
fi

exit $test_status