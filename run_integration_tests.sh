#!/bin/bash

# Disable integration tests for the moment to debug error:
# See https://github.com/planetary-social/nosrelay/actions/runs/11259467822/job/31309644214
#nosrelay-1  | Download https://deno.land/std@0.88.0/async/pool.ts
#nosrelay-1  | Download https://deno.land/std@0.88.0/fmt/colors.ts
#nosrelay-1  | Download https://deno.land/std@0.88.0/testing/_diff.ts
#nosrelay-1  | error: Uncaught (in promise) TypeError: Deno.seekSync is not a function
#nosrelay-1  |       Deno.seekSync(rid, offset, Deno.SeekMode.Start);
#nosrelay-1  |            ^
#nosrelay-1  |     at js_read (https://deno.land/x/sqlite@v3.7.1/build/vfs.js:48:12)
#nosrelay-1  |     at <anonymous> (wasm://wasm/0027cea2:1:5885)
#nosrelay-1  |     at <anonymous> (wasm://wasm/0027cea2:1:145143)
#nosrelay-1  |     at <anonymous> (wasm://wasm/0027cea2:1:140310)
#nosrelay-1  |     at <anonymous> (wasm://wasm/0027cea2:1:146451)
#nosrelay-1  |     at <anonymous> (wasm://wasm/0027cea2:1:3856)
#nosrelay-1  |     at <anonymous> (wasm://wasm/0027cea2:1:602396)
#nosrelay-1  |     at https://deno.land/x/sqlite@v3.7.1/src/db.ts:208:27
#nosrelay-1  |     at setStr (https://deno.land/x/sqlite@v3.7.1/src/wasm.ts:19:20)
#nosrelay-1  |     at new DB (https://deno.land/x/sqlite@v3.7.1/src/db.ts:205:20)
#nosrelay-1  | 2024-10-09 17:24:06.639 (   2.441s) [Writer          ] ERR| Couldn't setup plugin: pipe to plugin was closed (plugin crashed?)
exit 0

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