import { assertEquals } from "https://deno.land/std@0.181.0/testing/asserts.ts";
import { buildEvent, buildInputMessage } from "./test.ts";
import { createBroadcastVanishRequests } from "../broadcast_vanish_requests.ts";
import type { Event } from "https://raw.githubusercontent.com/planetary-social/strfry-policies/refs/heads/nos-changes/mod.ts";

class RedisMock {
  called: boolean = false;

  async xadd(streamKey: string, id: string, event: Event): Promise<void> {
    this.called = true;
  }
}

Deno.test({
  name: "pushes a vanish request with global relay filter",
  fn: async () => {
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        kind: 62,
        tags: [
          ["relay", "ALL_RELAYS"],
          ["relay", "notexample.com"],
        ],
      }),
    });

    const redisMock = new RedisMock();
    const broadcastVanishRequests = createBroadcastVanishRequests(
      redisMock,
      "example.com"
    );

    assertEquals((await broadcastVanishRequests(msg)).action, "accept");
    assertEquals(redisMock.called, true);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "pushes a vanish request with specific relay filter",
  fn: async () => {
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        kind: 62,
        tags: [
          ["relay", "example.com"],
          ["relay", "notexample.com"],
        ],
      }),
    });

    const redisMock = new RedisMock();
    const broadcastVanishRequests = createBroadcastVanishRequests(
      redisMock,
      "example.com"
    );

    assertEquals((await broadcastVanishRequests(msg)).action, "accept");
    assertEquals(redisMock.called, true);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "doesn't push a vanish request with no matching relay filter",
  fn: async () => {
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        kind: 62,
        tags: [["relay", "notexample.com"]],
      }),
    });

    const redisMock = new RedisMock();
    const broadcastVanishRequests = createBroadcastVanishRequests(
      redisMock,
      "example.com"
    );

    assertEquals((await broadcastVanishRequests(msg)).action, "accept");
    assertEquals(redisMock.called, false);
  },
  sanitizeResources: false,
});

Deno.test({
  name: "doesn't push when kind is not a vanish request",
  fn: async () => {
    const msg = buildInputMessage({
      sourceType: "IP4",
      sourceInfo: "1.1.1.1",
      event: buildEvent({
        kind: 1,
        tags: [
          ["relay", "ALL_RELAYS"],
          ["relay", "example.com"],
        ],
      }),
    });

    const redisMock = new RedisMock();
    const broadcastVanishRequests = createBroadcastVanishRequests(
      redisMock,
      "example.com"
    );
    assertEquals((await broadcastVanishRequests(msg)).action, "accept");
    assertEquals(redisMock.called, false);
  },
  sanitizeResources: false,
});
