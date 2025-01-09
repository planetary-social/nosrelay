#!/bin/bash

/app/strfry relay &
/app/vanish_subscriber &

# Wait for any process to exit
wait -n

EXIT_STATUS=$?
pkill -P $$

# Exit with the status of the first exited process
exit $EXIT_STATUS