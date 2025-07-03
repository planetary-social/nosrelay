##
## Default strfry config for relayable/strfry Docker
##

# Directory that contains the strfry LMDB database (restart required)
db = "./strfry-db/"

dbParams {
    # Maximum number of threads/processes that can simultaneously have LMDB transactions open (restart required)
    maxreaders = 2024

    # Size of mmap() to use when loading LMDB (default is 10TB, does *not* correspond to disk-space used) (restart required)
    mapsize = 10995116277760
}

relay {
    # Interface to listen on. Use 0.0.0.0 to listen on all interfaces (restart required)
    bind = "0.0.0.0"

    # Port to open for the nostr websocket protocol (restart required)
    port = 7777

    # Set OS-limit on maximum number of open files/sockets (if 0, don't attempt to set) (restart required)
    nofiles = 1000000

    # HTTP header that contains the client's real IP, before reverse proxying (ie x-real-ip) (MUST be all lower-case)
    realIpHeader = "x-forwarded-for"

    info {
        # NIP-11: Name of this server. Short/descriptive (< 30 characters)
        name = "nos.social strfry relay"

        # NIP-11: Detailed information about relay, free-form
        description = "This is a strfry instance handled by nos.social"

        # NIP-11: Administrative nostr pubkey, for contact purposes
        pubkey = "89ef92b9ebe6dc1e4ea398f6477f227e95429627b0a33dc89b640e137b256be5"

        # NIP-11: Alternative administrative contact (email, website, etc)
        contact = "https://nos.social"

        # List of supported lists as JSON array, or empty string to use default. Example: [1,2]
        nips = "[1,2,4,9,11,12,16,20,22,28,33,40,62]"
    }

    # Maximum accepted incoming websocket frame size (should be larger than max event and yesstr msg) (restart required)
    maxWebsocketPayloadSize = 262144

    # Websocket-level PING message frequency (should be less than any reverse proxy idle timeouts) (restart required)
    autoPingSeconds = 55

    # If TCP keep-alive should be enabled (detect dropped connections to upstream reverse proxy)
    enableTcpKeepalive = true

    # How much uninterrupted CPU time a REQ query should get during its DB scan
    queryTimesliceBudgetMicroseconds = 10000

    # Maximum records that can be returned per filter
    maxFilterLimit = 500

    # Maximum number of subscriptions (concurrent REQs) a connection can have open at any time
    maxSubsPerConnection = 100

    writePolicy {
        # If non-empty, path to an executable script that implements the writePolicy plugin logic
        plugin = "/app/plugins/policies.ts"

        # Number of seconds to search backwards for lookback events when starting the writePolicy plugin (0 for no lookback)
        lookbackSeconds = 0
    }

    compression {
        # Use permessage-deflate compression if supported by client. Reduces bandwidth, but slight increase in CPU (restart required)
        enabled = true

        # Maintain a sliding window buffer for each connection. Improves compression, but uses more memory (restart required)
        slidingWindow = true
    }

    logging {
        # Dump all incoming messages
        dumpInAll = false

        # Dump all incoming EVENT messages
        dumpInEvents = false

        # Dump all incoming REQ/CLOSE messages
        dumpInReqs = false

        # Log performance metrics for initial REQ database scans
        dbScanPerf = false
    }

    numThreads {
        # Ingester threads: route incoming requests, validate events/sigs (restart required)
        ingester = 5

        # reqWorker threads: Handle initial DB scan for events (restart required)
        reqWorker = 5

        # reqMonitor threads: Handle filtering of new events (restart required)
        reqMonitor = 3
   }
}

events {
    # One of the users we found was
    # following 2546 users, and the JSON for that was 190k
    # https://primal.net/p/npub1nlch0l8fj86x4ew2f5kxdn4q3vwmprkz0v7hsm9vcry62xee683qmqq7ay
    # Explore the followers DB periodically to find the largest user, compare
    # that with other services and check if the limits are still reasonable
    # If you change this, ensure it aligns to events relay filters:
    # https://github.com/planetary-social/nos-event-service/blob/e7aacd5d402f9cd8be8aee7ad64e3ed211357f31/service/app/handler_process_saved_event.go#L21-L26

    # Maximum size of normalised JSON, in bytes
    maxEventSize = 196608

    # Events newer than this will be rejected
    rejectEventsNewerThanSeconds = 900

    # Events older than this will be rejected
    rejectEventsOlderThanSeconds = 94608000

    # Ephemeral events older than this will be rejected
    rejectEphemeralEventsOlderThanSeconds = 60

    # Ephemeral events will be deleted from the DB when older than this
    ephemeralEventsLifetimeSeconds = 300

    # Maximum number of tags allowed
    maxNumTags = 3000

    # Maximum size for tag values, in bytes
    maxTagValSize = 1024
}


