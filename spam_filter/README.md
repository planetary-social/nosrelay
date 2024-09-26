# Spam Cleaner

Spam Cleaner is a tool to delete events that don't comply with our policies directly from the Strfry database. Currently, it provides a command to clean the database based on a JSONL stream from `stdin`. Integration with Strfry plugins is planned. The tool creates a pool of worker tasks that analyze each event through various checks using a local Nostr connection to `ws://localhost:7777`.

You can test the filter using the following commands:

```sh
cat test.jsonl | nak event ws://localhost:7777
docker compose exec -it nosrelay bash -c './strfry scan \'{"kinds":[1]}\' | spam_cleaner'
nak req ws://localhost:7777
```

Use `spam_cleaner --dry-run` to skip deletion. View configuration options with --help. For increase debuging info prefix with `RUST_LOG=debug`.
