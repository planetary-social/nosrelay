import type { Policy } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/export_log/mod.ts";
import { log } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/export_log/mod.ts";
import { connect, parseURL } from "https://deno.land/x/redis/mod.ts";

const REQUEST_TO_VANISH_KIND = 62;
const REDIS_URL = Deno.env.get("REDIS_URL");
const RELAY_URL = Deno.env.get("RELAY_URL");
const REDIS_CONNECT_OPTIONS = parseURL(REDIS_URL);
const REDIS = await connect(REDIS_CONNECT_OPTIONS);
const STREAM_KEY = "vanish_requests";
const ONE_WEEK_MS = 7 * 24 * 60 * 60 * 1000; // One week in milliseconds

if (!REDIS_URL) {
  throw new Error("REDIS_URL environment variable is not set.");
}

if (!RELAY_URL) {
  throw new Error("RELAY_URL environment variable is not set.");
}

const broadcastVanishRequests: Policy<void> = async (msg) => {
  const event = msg.event;
  const accept: { id: string; action: string; msg: string } = {
    id: event.id,
    action: "accept",
    msg: "",
  };

  if (event.kind !== REQUEST_TO_VANISH_KIND) {
    return accept;
  }

  const match = event.tags
    .filter((tag) => tag["0"].toLowerCase().trim() === "relay")
    .map((tag) => tag["1"].toLowerCase().trim())
    .find((relay) => relay === "all_relays" || relay === RELAY_URL);

  if (!match) {
    return accept;
  }

  await broadcastVanishRequest(event);

  return accept;
};

async function broadcastVanishRequest(event: any) {
  log(
    `Pushing vanish request: id: ${event.id}, pubkey: ${event.pubkey}, tags: ${event.tags}, content: ${event.content}`
  );

  try {
    await REDIS.xadd(STREAM_KEY, "*", event);
  } catch (error) {
    log(`Failed to push request  ${event.id} to Redis Stream: ${error}`);
  }
}

export default broadcastVanishRequests;
