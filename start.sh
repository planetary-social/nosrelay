#!/bin/bash

# Start the relay in the background
/app/strfry relay &
RELAY_PID=$!

# Start vanish subscriber
/app/vanish_subscriber &
VANISH_PID=$!

# Only start router if config exists and relay_sync_peers is configured
if [ -f "/etc/strfry-router.conf" ]; then
    echo "Starting strfry router..."
    /app/strfry router /etc/strfry-router.conf &
    ROUTER_PID=$!
fi

# Wait for any process to exit
wait -n $RELAY_PID $VANISH_PID ${ROUTER_PID:-}

EXIT_STATUS=$?

# If one process dies, kill the others
kill $RELAY_PID $VANISH_PID ${ROUTER_PID:-} 2>/dev/null

# Exit with the status of the first exited process
exit $EXIT_STATUS