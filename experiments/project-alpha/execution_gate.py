"""Experimental profile forwarding; NOT enabled by the editing HTTP runner.

Profiles are operator-owned requests to existing Rust process tools. This layer
cannot grant execution/isolation/network authority or approve a Rust elicitation.
"""
import asyncio
import base64
import json
import re

from mcp.shared.exceptions import MCPError
from forward_core import ForwardGate, ForwardError

NAME = re.compile(r'[a-z][a-z0-9_-]{0,31}\Z', re.ASCII)
JOB = re.compile(r'[0-9a-f]{64}\Z', re.ASCII)
SAFE = {'optic.process_isolation_unavailable', 'optic.process_executable_not_allowed',
        'optic.network_runtime_unavailable', 'optic.process_profile_changed',
        'optic.process_profile_not_found', 'optic.session_inactive', 'optic.policy_denied'}
TASK_TOOLS = ('optic_task_start', 'optic_task_result', 'optic_task_read', 'optic_task_stop')


def integer(value, low, high):
    if type(value) is not int or not low <= value <= high:
        raise ValueError('execution.invalid_arguments')
    return value


def freeze_profiles(profiles):
    if type(profiles) is not dict or not 1 <= len(profiles) <= 8:
        raise ValueError('execution.invalid_profiles')
    frozen = {}
    fields = {'executable', 'args', 'cwd', 'env_allowlist', 'network',
              'timeout_ms', 'output_budget', 'memory_bytes', 'process_count'}
    for name, request in profiles.items():
        if type(name) is not str or not NAME.fullmatch(name) or type(request) is not dict or set(request) != fields:
            raise ValueError('execution.invalid_profiles')
        if (type(request['executable']) is not str or not 1 <= len(request['executable']) <= 512
                or type(request['args']) is not list or len(request['args']) > 32
                or any(type(arg) is not str or len(arg) > 2048 or '\0' in arg for arg in request['args'])
                or request['env_allowlist'] != [] or request['network'] is not False
                or request['cwd'] is not None and (type(request['cwd']) is not str or len(request['cwd']) > 512)):
            raise ValueError('execution.invalid_profiles')
        integer(request['timeout_ms'], 1, 30000)
        integer(request['output_budget'], 1, 65536)
        integer(request['memory_bytes'], 1, 512 * 1024 * 1024)
        integer(request['process_count'], 1, 1)
        blob = json.dumps(request, ensure_ascii=True, allow_nan=False)
        if len(blob.encode()) > 16384:
            raise ValueError('execution.invalid_profiles')
        frozen[name] = blob  # no mutable config survives; caller cannot change args.
    return frozen


def validate_arguments(tool, arguments, profiles):
    if type(arguments) is not dict:
        raise ValueError()
    if tool == 'optic_task_start':
        if (set(arguments) != {'profile'} or type(arguments['profile']) is not str
                or arguments['profile'] not in profiles):
            raise ValueError()
        return
    if tool not in TASK_TOOLS:
        raise ValueError()
    expected = {'job_id', 'stream', 'cursor', 'max_bytes'} if tool == 'optic_task_read' else {'job_id'}
    if (not {'job_id'} <= set(arguments) or not set(arguments) <= expected
            or tool != 'optic_task_read' and set(arguments) != expected
            or type(arguments['job_id']) is not str
            or not JOB.fullmatch(arguments['job_id'])):
        raise ValueError()
    if tool == 'optic_task_read':
        if arguments.get('stream', 'stdout') not in ('stdout', 'stderr'):
            raise ValueError()
        integer(arguments.get('cursor', 0), 0, 65536)
        integer(arguments.get('max_bytes', 1024), 1, 1024)


def bind_admission(files, execution):
    # Both gates must be freshly constructed for one exact connection binding.
    if (files.pending or execution.pending or files.stopped or execution.stopped
            or files.owner != execution.owner or files.principal != execution.principal
            or files.registry is not execution.registry or files.upstream is not execution.upstream):
        raise ValueError('execution.binding_mismatch')
    execution.pending = files.pending
    execution.file_gate = files


class ExecutionGate(ForwardGate):
    def __init__(self, registry, identity, upstream, profiles, deadline=15):
        super().__init__(registry, identity, upstream, capacity=1, deadline=deadline)
        self.profiles = freeze_profiles(profiles)
        self.jobs = {}
        self.starts = 0
        self.file_gate = None

    def _current(self, identity):
        return (super()._current(identity)
                and (self.file_gate is None or self.file_gate._current(identity)))

    async def call(self, identity, tool, arguments):
        self._loop()
        if not self._current(identity):
            raise ForwardError('link.inactive')
        try:
            name, request = self.normalize(identity, tool, arguments)
        except ValueError:
            raise ForwardError('execution.invalid_arguments') from None
        if self.pending:
            raise ForwardError('link.busy')
        if tool == 'optic_task_start':
            if self.starts >= 16:
                raise ForwardError('execution.start_budget')
            self.starts += 1  # uncertainty/failure never replenishes the mint budget.
        task = asyncio.create_task(self._run_task(identity, name, request))
        self.pending.add(task)
        task.add_done_callback(self._complete)
        try:
            async with asyncio.timeout(self.deadline):
                return await asyncio.shield(task)
        except TimeoutError:
            raise ForwardError('execution.outcome_unknown' if name in ('process_start', 'process_stop')
                               else 'link.response_timeout') from None

    def normalize(self, identity, tool, arguments):
        validate_arguments(tool, arguments, self.profiles)
        if tool == 'optic_task_start':
            if set(arguments) != {'profile'} or type(arguments['profile']) is not str:
                raise ValueError()
            if arguments['profile'] not in self.profiles:
                raise ValueError()
            return 'process_start', json.loads(self.profiles[arguments['profile']])
        if tool not in ('optic_task_result', 'optic_task_read', 'optic_task_stop'):
            raise ValueError()
        job = arguments['job_id']
        if not JOB.fullmatch(job) or self.jobs.get(job) != (identity.principal, identity.epoch):
            raise ValueError()
        request = {'job_id': job}
        if tool == 'optic_task_read':
            request.update(stream=arguments.get('stream', 'stdout'),
                cursor=integer(arguments.get('cursor', 0), 0, 65536),
                max_bytes=integer(arguments.get('max_bytes', 1024), 1, 1024))
        return {'optic_task_result': 'process_result', 'optic_task_read': 'process_read',
                'optic_task_stop': 'process_stop'}[tool], request

    async def _run_task(self, identity, name, arguments):
        dispatched = False
        try:
            if not self._current(identity):
                raise ForwardError('link.inactive')
            dispatched = True
            response = await self.upstream(name, arguments)
            if len(json.dumps(response, ensure_ascii=True, allow_nan=False).encode()) > 16384:
                raise ValueError()
            if not self._current(identity):
                raise ForwardError('execution.outcome_unknown' if name in ('process_start', 'process_stop')
                                   else 'link.inactive')
            if name == 'process_start':
                job = response['job_id']
                if type(job) is not str or not JOB.fullmatch(job) or job in self.jobs:
                    raise ValueError()
                self.jobs[job] = (identity.principal, identity.epoch)
                return {'job_id': job}
            if name == 'process_read':
                raw = base64.b64decode(response['data'], validate=True)
                if len(raw) > arguments['max_bytes']:
                    raise ValueError()
                metadata = {key: response[key] for key in ('offset', 'next_offset', 'eof', 'truncated')}
                integer(metadata['offset'], 0, 65536)
                integer(metadata['next_offset'], metadata['offset'], 65536)
                if (metadata['offset'] != arguments['cursor']
                        or metadata['next_offset'] != metadata['offset'] + len(raw)
                        or type(metadata['eof']) is not bool or type(metadata['truncated']) is not bool):
                    raise ValueError()
                try:
                    return {**metadata, 'text': raw.decode('utf-8', errors='strict')}
                except UnicodeError:
                    return {**metadata, 'text': None, 'encoding': 'base64', 'data': response['data']}
            if name == 'process_stop':
                if type(response['requested']) is not bool:
                    raise ValueError()
                return {'requested': response['requested']}  # request, not proof of exit.
            statuses = {'running', 'exited', 'stopped', 'timed_out', 'output_limit_exceeded',
                        'termination_uncertain', 'failed'}
            if (response['status'] not in statuses or type(response['output_truncated']) is not bool
                    or response['exit_code'] is not None and type(response['exit_code']) is not int):
                raise ValueError()
            return {key: response[key] for key in ('status', 'exit_code', 'output_truncated')}
        except ForwardError:
            raise
        except MCPError as error:
            if error.message in SAFE:
                raise ForwardError(error.message) from None
            raise ForwardError('execution.outcome_unknown' if dispatched and name in ('process_start', 'process_stop')
                               else 'execution.upstream_failed') from None
        except Exception:
            raise ForwardError('execution.outcome_unknown' if dispatched and name in ('process_start', 'process_stop')
                               else 'execution.upstream_failed') from None
