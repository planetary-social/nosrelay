#!/bin/sh
//bin/true; exec deno run -A "$0" "$@"
import {
  antiDuplicationPolicy,
  hellthreadPolicy,
  pipeline,
  rateLimitPolicy,
  readStdin,
  writeStdout,
} from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";
import nosPolicy from "./nos_policy.ts";
import { createBroadcastVanishRequests } from "./broadcast_vanish_requests.ts";
import { connect, parseURL } from "https://deno.land/x/redis/mod.ts";

const localhost = "127.0.0.1";
const redis_url = Deno.env.get("REDIS_URL");
const redis_connect_options = parseURL(redis_url);
const redis = await connect(redis_connect_options);

const relay_url = Deno.env.get("RELAY_URL");
const broadcastVanishRequests = createBroadcastVanishRequests(redis, relay_url);

// Policies that reject faster should be at the top. So synchronous policies should be at the top.
const policies = [
  nosPolicy,
  [hellthreadPolicy, { limit: 100 }],
  // Async policies
  [antiDuplicationPolicy, { ttl: 60000, minLength: 50 }],
  [rateLimitPolicy, { whitelist: [localhost] }],
  broadcastVanishRequests,
];

for await (const msg of readStdin()) {
  const result = await pipeline(msg, policies);
  writeStdout(result);
}
