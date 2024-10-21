import type {
  Policy,
  OutputMessage,
} from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";
import { log } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";
import { PubkeyCache } from "./pubkey_cache.ts";

const REQUEST_TO_VANISH_KIND = 62;
const VANISH_STREAM_KEY = "vanish_requests";

const CACHE_MAX_SIZE = 1_000_000;

async function createBroadcastVanishRequests(
  redis: any,
  relay_url: string,
  vanishedPubkeyCache?: PubkeyCache
): Promise<Policy<void>> {
  if (!redis) {
    throw new Error("REDIS_URL environment variable is not set.");
  }

  if (!relay_url) {
    throw new Error("RELAY_URL environment variable is not set.");
  }

  let cache: PubkeyCache;
  if (!vanishedPubkeyCache) {
    cache = new PubkeyCache(redis, CACHE_MAX_SIZE);
    await cache.initialize();
  } else {
    cache = vanishedPubkeyCache;
  }

  return async (msg) => {
    const event = msg.event;
    const pubkey = event.pubkey;

    if (cache.has(pubkey)) {
      return {
        id: event.id,
        action: "shadowReject",
        msg: "",
      } as OutputMessage;
    }

    const accept: OutputMessage = {
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
      .find((relay) => relay === "all_relays" || relay === relay_url);

    if (!match) {
      return accept;
    }

    await broadcastVanishRequest(event, redis);
    await cache.add(pubkey);

    return accept;
  };
}

async function broadcastVanishRequest(event: any, redis: any) {
  log(
    `Pushing vanish request: id: ${event.id}, pubkey: ${event.pubkey}, tags: ${event.tags}, content: ${event.content}`
  );

  try {
    await redis.xadd(VANISH_STREAM_KEY, "*", event);
  } catch (error) {
    log(`Failed to push request ${event.id} to Redis Stream: ${error}`);
  }
}

export { createBroadcastVanishRequests };
