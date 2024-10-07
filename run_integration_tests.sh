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
    exit 1  # Exit immediately if the assertion fails
  fi
}

echo Assert we start with an empty relay
req=$(nak req -q ws://nosrelay:7777 | jq -c .)
assert_jsonl_equals "$req" "" "Initial relay state should be empty"

echo Assert initial events
key1=$(nak key generate)
pubkey1=$(nak key public "$key1")
event1=$(nak event -q -k 1 -c "content 1" --sec "$key1" ws://nosrelay:7777)

key2=$(nak key generate)
pubkey2=$(nak key public "$key2")
event2=$(nak event -q -k 1 -c "content 2" --sec "$key2" ws://nosrelay:7777)

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
$event2
$event1
EOF
)

assert_jsonl_equals "$req" "$expected_req" "Failed to assert initial events"

echo Send a vanish request pointing to a different relay
vanish_to_another_relay=$(nak event -q -k 62 -c "Delete all my events!" -t relay=wss://notexample.com --sec "$key1" ws://nosrelay:7777)

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
$vanish_to_another_relay
$event2
$event1
EOF
)

assert_jsonl_equals "$req" "$expected_req" "Failed after sending vanish request to another relay"

echo Send a vanish request pointing to this relay specifically
vanish_to_this_relay=$(nak event -q -k 62 -c "Delete all my events!" -t relay=wss://example.com --sec "$key1" ws://nosrelay:7777)
sleep 10  # Allow time for the vanish request to be processed

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=$(cat <<EOF
$event2
EOF
)

assert_jsonl_equals "$req" "$expected_req" "Failed after sending vanish request to this relay"

echo For the last event, send a global vanish request
vanish_from_all_relays=$(nak event -q -k 62 -c "Delete all my events!" -t relay=ALL_RELAYS --sec "$key2" ws://nosrelay:7777)
sleep 10  # Allow time for the vanish request to be processed

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=""

assert_jsonl_equals "$req" "$expected_req" "Failed after sending vanish request to all relays"

echo Manually send a vanish request from the command line
nak event -q -k 1 -c "content 1" --sec "$key1" ws://nosrelay:7777
push_vanish_request.ts -p $pubkey1 -r "Delete me!" -y --relay ws://nosrelay:7777
sleep 10  # Allow time for the vanish request to be processed

req=$(nak req -q ws://nosrelay:7777 | jq -c .)

expected_req=""

assert_jsonl_equals "$req" "$expected_req" "Failed after manual command line vanish request"

echo "Integration tests passed!"
exit 0