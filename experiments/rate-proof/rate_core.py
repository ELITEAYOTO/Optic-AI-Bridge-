"""Synthetic admission only: one shared budget, no caller-keyed state."""
import asyncio
import math
import time


class Bucket:
    def __init__(self, clock=time.monotonic, capacity=8, per_second=2):
        if (type(capacity) is not int or not 1 <= capacity <= 32
                or type(per_second) not in (int, float)
                or not math.isfinite(per_second) or not 1 / 32 <= per_second <= 32):
            raise ValueError('invalid fixed limits')
        self.clock, self.capacity, self.rate = clock, capacity, per_second
        self.last = clock()
        if not math.isfinite(self.last):
            raise ValueError('invalid monotonic clock')
        self.tokens = float(capacity)

    def take(self):
        # Called synchronously on one ASGI event loop: no await in admission.
        now = self.clock()
        if not math.isfinite(now):
            return False
        elapsed = max(0, now - self.last)
        self.last = max(self.last, now)
        self.tokens = min(self.capacity, self.tokens + elapsed * self.rate)
        if self.tokens < 1:
            return False
        self.tokens -= 1
        return True


class RateGate:
    """Run before Guard/auth/body. Never inspect headers, proxy identity or body.

    Deliberately GLOBAL: rotating arbitrary inputs cannot allocate new buckets.
    This local carrier experiment does not promise fairness or per-owner quotas.
    """
    def __init__(self, app, clock=time.monotonic, capacity=8, per_second=2):
        self.app = app
        self.bucket = Bucket(clock, capacity, per_second)
        self.loop = None

    async def __call__(self, scope, receive, send):
        if scope['type'] != 'http':
            return await self.app(scope, receive, send)
        loop = asyncio.get_running_loop()
        if self.loop is None:
            self.loop = loop
        if self.loop is not loop:
            raise RuntimeError('single event loop required')
        if not self.bucket.take():
            await send({'type': 'http.response.start', 'status': 429,
                        'headers': [(b'content-type', b'application/json'),
                                    (b'cache-control', b'no-store'),
                                    (b'retry-after', str(math.ceil(1 / self.bucket.rate)).encode('ascii'))]})
            await send({'type': 'http.response.body',
                        'body': b'{"code":"probe.rate_limited"}'})
            return
        await self.app(scope, receive, send)
