#!/usr/bin/env -S deno run --allow-net --allow-env

import { connect, parseURL } from "https://deno.land/x/redis@v0.29.0/mod.ts";
import { parse } from "https://deno.land/std@0.171.0/flags/mod.ts";
import { readLines } from "https://deno.land/std@0.171.0/io/mod.ts";

// Script to manually push a vanish request to the vanish_requests stream in
// Redis. Using deno to avoid any discrepancy vs the strfry policy.  The script
// pushed an unsigned request so that we can do it based on out of band
// requests, not using the nostr network.
// The script assumes is being run locally from within the relay container so it
// expects that both REDIS_URL and RELAY_URL are set in the environment.
//
// Example usage:
//
//./push_vanish_request.ts -p 79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81234 -r 'Requested through email from trusted user'
//
// The script asks for confirmation before pushing the request to the stream.

const args = parse(Deno.args, {
  alias: {
    p: "pubkey",
    r: "reason",
    h: "help",
    y: "yes",
  },
  string: ["pubkey", "reason", "relay"],
  boolean: ["y"],
  default: {
    relay: "wss://relay.nos.social",
    yes: false,
  },
});

if (args.help) {
  showUsage();
  Deno.exit(0);
}

if (!args.pubkey) {
  console.error("Error: PUBKEY is required.");
  showUsage();
  Deno.exit(1);
}

function showUsage() {
  console.log(`Usage: push_vanish_request.ts -p PUBKEY [-r REASON] [--relay RELAY_URL]
-p, --pubkey PUBKEY         The public key (required)
-r, --reason REASON         The reason for the vanish request (optional)
-y,                         Confirm the vanish request
-h, --help                  Show this help message
`);
}

async function main() {
  const pubkey = args.pubkey;
  const reason = args.reason || "";
  const skipConfirmation = args.yes;

  const redisUrl = Deno.env.get("REDIS_URL") || "redis://localhost:6379";
  const relayUrl = Deno.env.get("RELAY_URL") || "ws://localhost:7777";
  const redis_connect_options = parseURL(redisUrl);
  const redis = await connect(redis_connect_options);

  const VANISH_STREAM_KEY = "vanish_requests";
  const REQUEST_TO_VANISH_KIND = 62;
  const CREATED_AT = Math.floor(Date.now() / 1000);

  const event = {
    kind: REQUEST_TO_VANISH_KIND,
    pubkey: pubkey,
    created_at: CREATED_AT,
    tags: [["relay", relayUrl]],
    content: reason,
  };

  const confirmed = skipConfirmation || (await getConfirmation(event));
  if (!confirmed) {
    console.log("\nCanceled");
    redis.close();
    Deno.exit(0);
  }

  try {
    const xaddResult = await redis.xadd(VANISH_STREAM_KEY, "*", event);

    console.log(
      `Vanish request pushed successfully for pubkey '${pubkey}'. Stream ID: ${JSON.stringify(
        xaddResult,
        null,
        2
      )}`
    );
    console.log(`Event: ${JSON.stringify(event, null, 2)}`);
  } catch (error) {
    console.error("Failed to push vanish request.");
    console.error(error);
    Deno.exit(1);
  } finally {
    redis.close();
  }
}

async function getConfirmation(event: any): Promise<boolean> {
  const encoder = new TextEncoder();
  await Deno.stdout.write(
    encoder.encode(
      `\nAre you sure you want to create this vanish request? \n'${JSON.stringify(
        event,
        null,
        2
      )}'? (y/N): `
    )
  );

  for await (const line of readLines(Deno.stdin)) {
    const input = line.trim().toLowerCase();
    return input === "y" || input === "yes";
  }

  return false;
}

await main();
