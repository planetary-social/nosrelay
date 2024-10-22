import { assertEquals } from "https://deno.land/std@0.181.0/testing/asserts.ts";
import { buildEvent, buildInputMessage } from "./test.ts";
import { createBroadcastVanishRequests } from "../broadcast_vanish_requests.ts";
import { PubkeyCache } from "../pubkey_cache.ts";
import type { Event } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";

class RedisMock {
  called: boolean = false;
  zset: Map<string, number> = new Map();

  async xadd(streamKey: string, id: string, event: Event): Promise<void> {
    this.called = true;
  }

  async zrevrange(key: string, start: number, stop: number): Promise<string[]> {
    const entries = Array.from(this.zset.entries());
    entries.sort((a, b) => b[1] - a[1]);
    const sliced = entries.slice(start, stop + 1);
    return sliced.map(([pubkey, _score]) => pubkey);
  }

  async zcard(key: string): Promise<number> {
    return this.zset.size;
  }

  async zremrangebyrank(
    key: string,
    start: number,
    stop: number
  ): Promise<void> {
    const entries = Array.from(this.zset.entries());
    entries.sort((a, b) => a[1] - b[1]);
    const toRemove = entries.slice(start, stop + 1);
    for (const [pubkey, _score] of toRemove) {
      this.zset.delete(pubkey);
    }
  }

  async zscore(key: string, member: string): Promise<number | null> {
    const score = this.zset.get(member);
    return score !== undefined ? score : null;
  }

  async zadd(key: string, score: number, member: string): Promise<void> {
    this.zset.set(member, score);
  }

  async zrem(key: string, member: string): Promise<void> {
    this.zset.delete(member);
  }
}

async function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

Deno.test({
  name: "pushes a vanish request and then shadowRejects on duplicate pubkey",
  fn: async () => {
    const pubkey = "pubkey123";
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        pubkey: pubkey,
        kind: 62,
        tags: [
          ["relay", "ALL_RELAYS"],
          ["relay", "notexample.com"],
        ],
      }),
    });

    const redisMock = new RedisMock();
    const pubkeyCache = new PubkeyCache(redisMock, 1000);
    await pubkeyCache.initialize();

    const broadcastVanishRequests = await createBroadcastVanishRequests(
      redisMock,
      "example.com",
      pubkeyCache
    );

    const result1 = await broadcastVanishRequests(msg);
    assertEquals(result1.action, "accept");
    assertEquals(redisMock.called, true);

    redisMock.called = false;

    const result2 = await broadcastVanishRequests(msg);
    assertEquals(result2.action, "shadowReject");
    assertEquals(redisMock.called, false);

    // Some time to let the logs flush
    await wait(100);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "pushes a vanish request with specific relay filter and then shadowRejects on duplicate pubkey",
  fn: async () => {
    const pubkey = "pubkey456";
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        pubkey: pubkey,
        kind: 62,
        tags: [
          ["relay", "example.com"],
          ["relay", "notexample.com"],
        ],
      }),
    });

    const redisMock = new RedisMock();
    const pubkeyCache = new PubkeyCache(redisMock, 1000);
    await pubkeyCache.initialize();

    const broadcastVanishRequests = await createBroadcastVanishRequests(
      redisMock,
      "example.com",
      pubkeyCache
    );

    const result1 = await broadcastVanishRequests(msg);
    assertEquals(result1.action, "accept");
    assertEquals(redisMock.called, true);

    redisMock.called = false;

    const result2 = await broadcastVanishRequests(msg);
    assertEquals(result2.action, "shadowReject");

    // Some time to let the logs flush
    await wait(100);
    assertEquals(redisMock.called, false);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "doesn't push a vanish request with no matching relay filter",
  fn: async () => {
    const pubkey = "pubkey789";
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        pubkey: pubkey,
        kind: 62,
        tags: [["relay", "notexample.com"]],
      }),
    });

    const redisMock = new RedisMock();
    const pubkeyCache = new PubkeyCache(redisMock, 1000);
    await pubkeyCache.initialize();

    const broadcastVanishRequests = await createBroadcastVanishRequests(
      redisMock,
      "example.com",
      pubkeyCache
    );

    const result = await broadcastVanishRequests(msg);
    assertEquals(result.action, "accept");
    assertEquals(redisMock.called, false);

    const result2 = await broadcastVanishRequests(msg);
    assertEquals(result2.action, "accept");
    assertEquals(redisMock.called, false);

    // Some time to let the logs flush
    await wait(100);
    assertEquals(redisMock.called, false);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "doesn't push when kind is not a vanish request",
  fn: async () => {
    const pubkey = "pubkey101112";
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        pubkey: pubkey,
        kind: 1,
        tags: [
          ["relay", "ALL_RELAYS"],
          ["relay", "example.com"],
        ],
      }),
    });

    const redisMock = new RedisMock();
    const pubkeyCache = new PubkeyCache(redisMock, 1000);
    await pubkeyCache.initialize();

    const broadcastVanishRequests = await createBroadcastVanishRequests(
      redisMock,
      "example.com",
      pubkeyCache
    );

    const result = await broadcastVanishRequests(msg);
    assertEquals(result.action, "accept");
    assertEquals(redisMock.called, false);

    const result2 = await broadcastVanishRequests(msg);
    assertEquals(result2.action, "accept");
    assertEquals(redisMock.called, false);

    // Some time to let the logs flush
    await wait(100);
    assertEquals(redisMock.called, false);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "shadowRejects when pubkey is pre-loaded in cache",
  fn: async () => {
    const pubkey = "vanishedPubkey";
    const redisMock = new RedisMock();

    const now = Date.now();
    await redisMock.zadd(PubkeyCache.ZSET_KEY, now, pubkey);

    const pubkeyCache = new PubkeyCache(redisMock, 1000);
    await pubkeyCache.initialize();

    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        pubkey: pubkey,
        kind: 62,
        tags: [["relay", "ALL_RELAYS"]],
      }),
    });

    const broadcastVanishRequests = await createBroadcastVanishRequests(
      redisMock,
      "example.com",
      pubkeyCache
    );

    // Since the pubkey is pre-loaded, it should shadowReject
    const result = await broadcastVanishRequests(msg);
    assertEquals(result.action, "shadowReject");
    assertEquals(redisMock.called, false);

    // Some time to let the logs flush
    await wait(100);
    assertEquals(redisMock.called, false);
  },
  sanitizeResources: false,
});
