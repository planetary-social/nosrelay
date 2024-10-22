// In process cache to avoid accepting pubkeys that were already removed from
// the relay. We store CACHE_MAX_SIZE items in redis so restarts don't lose the
// cache. This assumes that after CACHE_MAX_SIZE items are processed, the
// oldest ones are no longer relevant and probably not used anymore in the
// network to avoid to have an infinite cache.
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
