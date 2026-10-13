"""Bounded experimental read-only forwarding; no HTTP, workspace or authority minting."""
import asyncio
import math


CALLS = {
    'optic_session_info': ('session_info', {}),
    'optic_fixture_list': ('fs_list', {'path': 'public', 'limit': 4}),
    'optic_fixture_read': ('fs_read', {'path': 'public/hello.txt', 'max_bytes': 1024}),
}


class ForwardError(Exception):
    pass


class ForwardGate:
    """One loop, one locally selected dummy principal, fixed fixture calls only.

    A caller timeout/disconnect does not cancel an admitted upstream read or free
    its slot. No waiter queue exists. Local stop blocks admission before draining.
    Credential state is rechecked before dispatch and before returning data.
    This does not revoke an actual Rust session or cancel upstream effects.
    """
    def __init__(self, registry, identity, upstream, capacity=8, deadline=15):
        if (type(capacity) is not int or not 1 <= capacity <= 8
                or type(deadline) not in (int, float)
                or not math.isfinite(deadline) or not 0 < deadline <= 15):
            raise ValueError('link.invalid_limits')
        if not registry.current(identity):
            raise ValueError('link.invalid_binding')
        self.registry, self.principal, self.owner = registry, identity.principal, identity.owner
        self.upstream, self.capacity, self.deadline = upstream, capacity, deadline
        self.pending = set()
        self.stopped = False
        self.loop = None

    def _loop(self):
        loop = asyncio.get_running_loop()
        if self.loop is None:
            self.loop = loop
        elif self.loop is not loop:
            raise ForwardError('link.loop_mismatch')

    def _current(self, identity):
        return (not self.stopped and self.registry.current(identity)
                and identity.owner == self.owner and identity.principal == self.principal)

    async def call(self, identity, tool):
        self._loop()
        if not self._current(identity):
            raise ForwardError('link.inactive')
        if type(tool) is not str or tool not in CALLS:
            raise ForwardError('link.invalid_tool')
        if len(self.pending) >= self.capacity:
            raise ForwardError('link.busy')
        # No await between capacity check and reservation on the owning loop.
        task = asyncio.create_task(self._run(identity, tool))
        self.pending.add(task)
        task.add_done_callback(self._complete)
        try:
            async with asyncio.timeout(self.deadline):
                return await asyncio.shield(task)
        except TimeoutError:
            raise ForwardError('link.response_timeout') from None

    async def _run(self, identity, tool):
        try:
            if not self._current(identity):
                raise ForwardError('link.inactive')
            name, arguments = CALLS[tool]
            result = await self.upstream(name, dict(arguments))
            if not self._current(identity):
                raise ForwardError('link.inactive')
            return result
        except ForwardError:
            raise
        except Exception:
            # SDK/parser exceptions can carry paths or credentials.
            raise ForwardError('link.upstream_failed') from None

    def _complete(self, task):
        self.pending.discard(task)
        if not task.cancelled():
            task.exception()  # Retrieve failures after caller disconnect/timeout.

    async def stop(self, drain_seconds=5):
        self._loop()
        if (type(drain_seconds) not in (int, float) or not math.isfinite(drain_seconds)
                or not 0 <= drain_seconds <= 5):
            raise ForwardError('link.invalid_drain')
        self.stopped = True
        if not self.pending:
            return True
        _, pending = await asyncio.wait(tuple(self.pending), timeout=drain_seconds)
        # False is uncertain cleanup, never permission to reset/reuse this gate.
        return not pending
