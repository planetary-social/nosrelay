// In-process cache to avoid accepting pubkeys that were already removed from
// the relay. We store `CACHE_MAX_SIZE` items in Redis so restarts don't lose
// the cache. This assumes that after `CACHE_MAX_SIZE` items are processed, the
// oldest ones are no longer relevant and probably not used anymore in the
// network, to avoid having an infinite cache.
//
// The `Set` is used for quick O(1) in-process lookups when we need to read from
// the cache. This ensures that there is very little lag because these lookups
// are extremely fast, much faster than Redis itself. The `Queue` allows us to
// remove the oldest entries when we exceed the maximum cache size to control
// memory usage. Both the `Set` and `Queue` are in-process data structures because
// Strfry blocks until it receives a response from a plugin, so write operations
// need to be as optimized as possible.
//
// While Redis is great, it's still too slow to be called on each write
// operation; it's asynchronous and remote compared to synchronous local
// queries, which are orders of magnitude faster. Therefore, the main use for
// Redis in this case is to persist the cache across restarts. By storing the
// cache in Redis, we ensure that the application retains important state
// information even after it restarts, without compromising the performance of
// relay write operations.
class PubkeyCache {
  private maxSize: number;
  private queue: string[] = [];
  private set: Set<string> = new Set();
  private redis: any;
  public static readonly ZSET_KEY = "processed_pubkeys_zset";

  constructor(redis: any, maxSize: number) {
    this.redis = redis;
    this.maxSize = maxSize;
  }

  async initialize(): Promise<void> {
    const pubkeys = await this.redis.zrevrange(
      PubkeyCache.ZSET_KEY,
      0,
      this.maxSize - 1
    );

    for (const pubkey of pubkeys) {
      this.queue.push(pubkey);
      this.set.add(pubkey);
    }

    const totalCount = await this.redis.zcard(PubkeyCache.ZSET_KEY);
    if (totalCount > this.maxSize) {
      await this.redis.zremrangebyrank(PubkeyCache.ZSET_KEY, this.maxSize, -1);
    }
  }

  has(pubkey: string): boolean {
    return this.set.has(pubkey);
  }

  async add(pubkey: string): Promise<void> {
    if (!this.set.has(pubkey)) {
      this.queue.push(pubkey);
      this.set.add(pubkey);

      const now = Date.now();
      await this.redis.zadd(PubkeyCache.ZSET_KEY, now, pubkey);

      if (this.queue.length > this.maxSize) {
        const oldestPubkey = this.queue.shift();
        if (oldestPubkey !== undefined) {
          this.set.delete(oldestPubkey);
          await this.redis.zrem(PubkeyCache.ZSET_KEY, oldestPubkey);
        }
      }
    }
  }
}

export { PubkeyCache };
