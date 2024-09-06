#!/bin/sh
//bin/true; exec deno run -A "$0" "$@"
import {
  antiDuplicationPolicy,
  hellthreadPolicy,
  pipeline,
  rateLimitPolicy,
  readStdin,
  writeStdout,
} from "https://gitlab.com/soapbox-pub/strfry-policies/-/raw/develop/mod.ts";
import nosPolicy from "./nos_policy.ts";

const localhost = "127.0.0.1";
const eventsIp = getEventsIp();

// Policies that reject faster should be at the top. So synchronous policies should be at the top.
const policies = [
  nosPolicy,
  [hellthreadPolicy, { limit: 100 }],
  // Async policies
  [antiDuplicationPolicy, { ttl: 60000, minLength: 50 }],
  [rateLimitPolicy, { whitelist: [localhost, eventsIp] }],
];

for await (const msg of readStdin()) {
  const result = await pipeline(msg, policies);
  if (result.msg.startsWith("rate-limited")) {
    result.msg = `${result.msg}: ${msg.sourceInfo}`;
  }
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
