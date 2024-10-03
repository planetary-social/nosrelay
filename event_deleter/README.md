# Vanish Listener

Vanish Listener is a tool that listens for vanish requests on a Redis stream and processes them by deleting the corresponding events from the Strfry database. It continuously monitors the `vanish_requests` stream in Redis and handles incoming deletion requests in real-time.

# Spam Cleaner

Spam Cleaner is a tool to delete events that don't comply with our policies directly from the Strfry database. Currently, it provides a command to clean the database based on a JSONL stream from `stdin`. Integration with Strfry plugins is planned. The tool creates a pool of worker tasks that analyze each event through various checks using a local Nostr connection to `ws://localhost:7777`.

You can test the filter using the following commands:

```sh
cat test.jsonl | nak event ws://localhost:7777
docker compose exec -it nosrelay bash -c './strfry scan \'{"kinds":[1]}\' | spam_cleaner'
nak req ws://localhost:7777
```

You can further control the flow between pipes using `pv`:
```
 ./strfry scan --pause 1 '{"kinds":[1,30023],"since":1724711684,"limit":100000}' |  pv -L 100k -q |spam_cleaner --dry-run
 ```

Use `spam_cleaner --dry-run` to skip deletion. View configuration options with --help. For increase debuging info prefix with `RUST_LOG=debug`.

Remember it's a good idea to backup the db first:
```
./strfry export | zstd -c > backup.jsonl.zst
```