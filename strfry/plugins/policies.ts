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
const eventsIp = await getEventsIp();
const syncIp = await getSyncIp();
const one_minute = 60 * 1000;
const one_hour = 60 * one_minute;
const one_day = 24 * one_hour;
const two_days = 2 * one_day;

const redis_url = Deno.env.get("REDIS_URL");
const redis_connect_options = parseURL(redis_url);
const redis = await connect(redis_connect_options);

const relay_url = Deno.env.get("RELAY_URL");
const broadcastVanishRequests = await createBroadcastVanishRequests(
  redis,
  relay_url
);

// Policies that reject faster should be at the top. So synchronous policies should be at the top.
const policies = [
  nosPolicy,
  [hellthreadPolicy, { limit: 100 }],

  // Async policies
  // Let's test with one day, if it's too much we can reduce it.
  [antiDuplicationPolicy, { ttl: one_day, minLength: 30 }],

  // For abusers, we ban the ip after 20 requests per minute, ban ip for 2 days.
  // Remember to leave the more loose rate limit policy at the top.
  [
    rateLimitPolicy,
    {
      max: 20,
      interval: one_minute,
      banInterval: two_days,
      whitelist: [localhost, eventsIp, syncIp],
      // We use a different db url so that this limiter is not affected by the other limiters.
      // The file is stored in the strfry-db folder for persistence between restarts.
      databaseUrl:
        "sqlite:///app/strfry-db/banning-strfry-rate-limit-policy.sqlite3",
    },
  ],

  // Normal rate limit without banning, 10 requests per minute, if the ip hit
  // this one, it won't be banned, just rate-limited.
  [
    rateLimitPolicy,
    {
      max: 10,
      interval: one_minute,
      whitelist: [localhost, eventsIp, syncIp],
    },
  ],

  // Broadcast vanish requests to Redis
  broadcastVanishRequests,
];

for await (const msg of readStdin()) {
  const result = await pipeline(msg, policies);
  writeStdout(result);
}

async function getEventsIp() {
  const fallbackEventsIp = "174.138.53.241";

  try {
    const resolvedIps = await Deno.resolveDns("events.nos.social", "A");
    return resolvedIps[0];
  } catch (error) {
    return fallbackEventsIp;
  }
}

async function getSyncIp() {
  const fallbackSyncIp = "159.65.45.194";

  try {
    const resolvedIps = await Deno.resolveDns("sync.nos.social", "A");
    return resolvedIps[0];
  } catch (error) {
    return fallbackSyncIp;
  }
}
