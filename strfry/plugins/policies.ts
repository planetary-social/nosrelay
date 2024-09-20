#!/bin/sh
//bin/true; exec deno run -A "$0" "$@"
import {
  antiDuplicationPolicy,
  hellthreadPolicy,
  pipeline,
  rateLimitPolicy,
  readStdin,
  writeStdout,
} from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/ban-interval/mod.ts";
import nosPolicy from "./nos_policy.ts";

const localhost = "127.0.0.1";
const eventsIp = await getEventsIp();
const one_minute = 60 * 1000;
const two_days = 2 * 24 * 60 * one_minute;

// Policies that reject faster should be at the top. So synchronous policies should be at the top.
const policies = [
  nosPolicy,
  [hellthreadPolicy, { limit: 100 }],

  // Async policies
  [antiDuplicationPolicy, { ttl: 60000, minLength: 50 }],

  // For abusers, we ban the ip after 20 requests per minute, ban ip for 2 days
  // Remember to leave the more loose rate limit policy at the top
  [
    rateLimitPolicy,
    {
      max: 20,
      interval: one_minute,
      ban_interval: two_days,
      whitelist: [localhost, eventsIp],
      // We use a different db url so that this limiter is not affected by the other limiters
      databaseUrl: "sqlite:///tmp/banning-strfry-rate-limit-policy.sqlite3",
    },
  ],

  // Normal rate limit without banning, 10 requests per minute, if the ip hit this one, it won't be banned, just rate-limited
  [
    rateLimitPolicy,
    {
      max: 10,
      interval: one_minute,
      whitelist: [localhost, eventsIp],
    },
  ],
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
