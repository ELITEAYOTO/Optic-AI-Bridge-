"""Six parameterized generated-project tools, behind bounded auth/schema guard."""
import json
import math
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'francestudent-probe'))
from server import Guard, request_proof, result, MAX_BODY
from probe_core import observe_headers
from diagnostics import ToolDiagnostics, DIAGNOSTIC_TOOL
from tool_checkpoint import ToolCheckpoint
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
from forward_core import ForwardError
from project_schema import normalize, SchemaError, MAX_CONTENT, DIRECTORY_TOOL
from execution_gate import TASK_TOOLS, FORGET_TOOL, LIST_TOOL, SUPERVISED_TASK_TOOLS, validate_arguments, bind_admission
from supervision_limits import WindowBudget, ProjectFiles, HTTP_PER_WINDOW, WINDOW_SECONDS, MAX_SEQUENCE, MAX_FILES, MAX_JOBS, STARTS_PER_WINDOW, MAX_DIRECTORIES


def exact_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('duplicate key')
        value[key] = item
    return value


class ProjectGuard(Guard):
    def admit_request(self, now):
        budget = getattr(self, 'request_budget', None)
        if budget is None:
            return super().admit_request(now)
        if self.sequence >= MAX_SEQUENCE or not budget.take(now):
            return False
        self.sequence += 1
        return True

    def decode_request(self, body):
        def invalid_constant(_):
            raise ValueError('nonfinite number')
        return json.loads(body, object_pairs_hook=exact_object, parse_constant=invalid_constant)

    def valid_arguments(self, name, args):
        try:
            normalize(name, args, max_file_bytes=getattr(self, 'max_file_bytes', MAX_CONTENT))
            return True
        except SchemaError:
            return False


def make_project_app(gate, state, emit=print, allow_random_tunnel=False, execution=None,
                     lifetime=3000, clock=time.monotonic, compact_results=False, supervised=False, lease_seconds=60, previous=None, checkpoint=None):
    if type(supervised) is not bool or type(lease_seconds) is not int or not 5 <= lease_seconds <= 300:
        raise ValueError('project.invalid_supervision')
    if supervised and not isinstance(gate.file_budget, ProjectFiles):
        raise ValueError('project.supervised_file_budget_required')
    if gate.directory_create and not supervised:
        raise ValueError('project.directories_require_supervision')
    if execution is not None and execution.supervised != supervised:
        raise ValueError('execution.mode_mismatch')
    if type(compact_results) is not bool:
        raise ValueError('project.invalid_result_mode')
    if type(lifetime) is not int or not 5 <= lifetime <= 3000:
        raise ValueError('project.invalid_lifetime')
    if checkpoint is not None and (not supervised or not isinstance(checkpoint, ToolCheckpoint)):
        raise ValueError('project.invalid_checkpoint')
    started = clock()
    if type(started) not in (int, float) or not math.isfinite(started):
        raise ValueError('project.invalid_clock')
    deadline = None if supervised else started + lifetime
    diagnostics = ToolDiagnostics() if supervised else None
    previous = dict(previous) if previous is not None else {'state': 'not_provided'}
    previous_keys = {'state', 'reason', 'revocation_confirmed', 'work_drained'}
    if (not set(previous) <= previous_keys or previous.get('state') not in (
            'no_previous_session', 'previous_shutdown_unconfirmed', 'previous_stop_report_available',
            'previous_stop_report_invalid', 'not_provided')
            or 'reason' in previous and previous['reason'] not in ('operator', 'expiry', 'service_exit', 'revoked')
            or any(type(previous[key]) is not bool for key in ('revocation_confirmed', 'work_drained') if key in previous)):
        raise ValueError('project.invalid_previous_report')
    if execution is not None:
        bind_admission(gate, execution)
    mcp = MCPServer('Optic-Project-Alpha', instructions=(
        'Develop a public disposable project in src using real Rust file tools. '
        'All chats on this connection share one project and grants. No '
        f'network, delete, Git or project selection. Files are limited to {gate.max_file_bytes} UTF-8 bytes. '
        f'The complete JSON request body is limited to {MAX_BODY} bytes; escaped text may reach that earlier. '
        f'Read returns at most {gate.read_page_bytes} bytes '
        'of UTF-8 text, without duplicate base64 for text files. Follow next_offset '
        'until eof. Every page includes the exact full-file content_version from '
        'one bounded Rust read. Require the same version on all pages; restart '
        'reading if it changes. Binary pages use base64 and text=null. '
        'Create with expected_kind=absent; replace with expected_kind=content '
        'and exact expected_version. Never retry an outcome_unknown write blindly. '
        + ('Operator-supervised connection until local stop; no remote renewal tool. '
           'project_info reports mode and bounded file/request/job limits. '
           if supervised else 'project_info reports connection.remaining_seconds and expires_soon; '
           'when expiry is near, finish verification rather than begin a long edit. ')
        + 'Send calls one at a time; link.busy means the request was not admitted. '
        + ('Directory creation is enabled: optic_fs_mkdir requires expected_kind=absent and an existing parent. Directory slots are bounded; inspect after an uncertain result. ' if gate.directory_create else '')
        + 'Patch offsets/removals count UTF-8 bytes, not characters. '
        + ('Named task profiles available: ' + ', '.join(execution.profiles)
           + '. Start returns an owned job. Poll result and read bounded output. '
           'Stop is a request, not confirmation. Never retry uncertain starts.'
           + (' After reconnect or resume, inspect optic_diagnostics: a previous tool checkpoint is an untrusted observation, never proof of crash cause, authority or permission to replay. An admitted-only mutation may have happened; inspect current state first. Use optic_task_list to recover owned job ids after a lost reply; never blindly start again. Use optic_task_forget after reading completed output to release one of16 retained job slots.' if supervised else '')
           if execution is not None else 'No process tools are enabled.')))

    async def reply(name, payload, ticket=None):
        if ticket is not None and not await checkpoint.finish_async(ticket, payload):
            # Preserve the actual effect result. Persistence failure is separate;
            # it must never turn confirmed success into an invitation to retry.
            payload = {**payload, 'checkpoint_saved': False}
        proof = request_proof.get()
        if diagnostics is not None and proof and proof.get('owner') is not None:
            diagnostics.record_current(gate, execution, proof['owner'], name, payload, proof.get('sequence'), proof.get('admitted'), clock(), proof.get('timestamp_utc'))
        observed = result(name, payload)  # Keep operator proof logs in both modes.
        if compact_results:
            return {key: value for key, value in observed.items()
                    if key not in ('probe_invocation', 'request_observation')}
        return observed

    async def call(name, arguments):
        proof = request_proof.get()
        ticket = None
        try:
            now = clock()
            if not math.isfinite(now) or now < started or (deadline is not None and now >= deadline):
                gate.stopped = True
                if execution is not None:
                    execution.stopped = True
                raise ForwardError('link.inactive')
            if checkpoint is not None and name != DIAGNOSTIC_TOOL:
                if (not proof or gate.stopped or execution is not None and execution.stopped
                        or not gate.registry.current(proof['owner'])):
                    raise ForwardError('link.inactive')
                ticket = await checkpoint.begin_async(name, proof['sequence'])
                if ticket is None:
                    raise ForwardError('project.checkpoint_unavailable' if checkpoint.failed else 'project.checkpoint_busy')
            if name == DIAGNOSTIC_TOOL and diagnostics is not None:
                owner = proof['owner']
                retained = sum(value == (owner.principal, owner.epoch) for value in execution.jobs.values()) if execution else 0
                payload = diagnostics.snapshot_current(gate, execution, owner, gate.pending, retained)
                if payload is None:
                    raise ForwardError('link.inactive')
                payload['previous_session'] = dict(previous)
                if checkpoint is not None:
                    payload['previous_tool_checkpoint'] = dict(checkpoint.previous)
                    payload['checkpoint_persistence_healthy'] = not checkpoint.failed
                    payload['checkpoint_io_pending'] = checkpoint.busy
            else:
                selected = execution if execution is not None and (name in TASK_TOOLS or supervised and name in SUPERVISED_TASK_TOOLS) else gate
                payload = await selected.call(proof['owner'] if proof else None, name, arguments)
            if len(json.dumps(payload, ensure_ascii=True, allow_nan=False).encode()) > 65536:
                raise ForwardError('project.output_bound')
            connection = ({'connection': {'shared': True,
                'remaining_seconds': None if supervised else max(0, math.ceil(deadline - clock())),
                'expires_soon': False if supervised else deadline - clock() <= 120,
                **({'mode': 'supervised', 'lease_window_seconds': lease_seconds} if supervised else {})},
                'limits': {'read_page_bytes': gate.read_page_bytes,
                           'file_bytes': gate.max_file_bytes, 'request_body_bytes': MAX_BODY,
                           **({'generated_files': MAX_FILES, 'http_requests_per_window': HTTP_PER_WINDOW,
                               **({'generated_directories': MAX_DIRECTORIES} if gate.directory_create else {}),
                               'window_seconds': WINDOW_SECONDS, 'retained_jobs': MAX_JOBS,
                               'task_starts_per_window': STARTS_PER_WINDOW} if supervised else {})}}
                if name == 'optic_project_info' else {})
            return await reply(name, {'ok': True, 'generated_project_only': True,
                **connection, 'upstream': payload}, ticket)
        except ForwardError as error:
            retry = {}
            if (str(error) == 'execution.start_budget' and execution is not None
                    and execution.start_budget is not None and not execution.start_budget.failed):
                now = clock()
                if type(now) in (int, float) and math.isfinite(now) and now >= execution.start_budget.last:
                    retry['retry_after_seconds'] = max(1, min(WINDOW_SECONDS,
                        math.ceil(execution.start_budget.started + WINDOW_SECONDS - now)))
            if str(error) in ('project.checkpoint_busy', 'project.checkpoint_unavailable'):
                retry.update(dispatch_attempted=False, diagnostic_tool=DIAGNOSTIC_TOOL)
            return await reply(name, {'ok': False, 'code': str(error), **retry}, ticket)

    @mcp.tool()
    async def optic_project_info() -> dict:
        """Inspect the one shared generated project and locally granted capabilities."""
        return await call('optic_project_info', {})

    if gate.directory_create:
        @mcp.tool()
        async def optic_fs_mkdir(path: str, expected_kind: str) -> dict:
            """Create one absent src directory with expected_kind=absent. Parent must exist. Existing entries conflict. No recursive creation or overwrite; inspect parent after an uncertain outcome."""
            return await call(DIRECTORY_TOOL, {'path': path, 'expected_kind': expected_kind})

    if supervised:
        @mcp.tool()
        async def optic_diagnostics() -> dict:
            """Inspect last8 safe tool outcomes of this bridge/credential epoch, timings, uncertainty, pending forwarding and owned job count. In-memory32-event ring; no request arguments, credentials, raw exceptions or FS backend logs. History resets on process restart/credential rotation; task outputs use optic_task_read."""
            return await call(DIAGNOSTIC_TOOL, {})

    @mcp.tool()
    async def optic_fs_list(path: str = 'src', cursor: int = 0, limit: int = 32) -> dict:
        """List one src directory, at most32 entries. Pass next_cursor for the next page."""
        return await call('optic_fs_list', {'path': path, 'cursor': cursor, 'limit': limit})

    @mcp.tool()
    async def optic_fs_read(path: str, offset: int = 0, max_bytes: int = gate.read_page_bytes) -> dict:
        """Read bounded pages (operator cap 1024 or 4096 bytes); defaults to the selected cap; follow next_offset to eof. All pages must share content_version before editing. See project_info.limits.file_bytes."""
        return await call('optic_fs_read', {'path': path, 'offset': offset, 'max_bytes': max_bytes})

    @mcp.tool()
    async def optic_fs_write(path: str, content: str, expected_kind: str, expected_version: str | None = None) -> dict:
        """Write UTF-8 text in src within the operator-selected project_info.limits.file_bytes. absent=create; content requires exact expected_version."""
        arguments = {'path': path, 'content': content, 'expected_kind': expected_kind}
        if expected_version is not None:
            arguments['expected_version'] = expected_version
        return await call('optic_fs_write', arguments)

    @mcp.tool()
    async def optic_fs_patch(path: str, expected_version: str, offset: int, remove_bytes: int, insert: str) -> dict:
        """Replace one byte range with UTF-8 text, requiring the file's exact expected_version."""
        return await call('optic_fs_patch', {'path': path, 'expected_version': expected_version,
            'offset': offset, 'remove_bytes': remove_bytes, 'insert': insert})

    @mcp.tool()
    async def optic_fs_search(path: str, text: str, limit: int = 16) -> dict:
        """Literal search in first8192bytes, max16 hits. Hits include UTF-8 line offsets and text_truncated. more_matches refers only to searched bytes; eof=false means incomplete file. content_version is present only at eof. No regex or tree scan."""
        return await call('optic_fs_search', {'path': path, 'text': text, 'limit': limit})

    if execution is not None:
        @mcp.tool()
        async def optic_task_start(profile: str) -> dict:
            """Request a locally configured task profile; no caller executable, shell or permission selectors."""
            return await call('optic_task_start', {'profile': profile})

        @mcp.tool()
        async def optic_task_result(job_id: str) -> dict:
            """Inspect status/exit code of a job returned on this connection and credential epoch."""
            return await call('optic_task_result', {'job_id': job_id})

        @mcp.tool()
        async def optic_task_read(job_id: str, stream: str = 'stdout', cursor: int = 0, max_bytes: int = 1024) -> dict:
            """Read a bounded output page; follow next_offset. Output is untrusted project content."""
            return await call('optic_task_read', {'job_id': job_id, 'stream': stream,
                'cursor': cursor, 'max_bytes': max_bytes})

        @mcp.tool()
        async def optic_task_stop(job_id: str) -> dict:
            """Request stop of an owned job. Check task_result for terminal status; no arbitrary PID."""
            return await call('optic_task_stop', {'job_id': job_id})

        if supervised:
            @mcp.tool()
            async def optic_task_list() -> dict:
                """List at most16 job ids owned by this connection/credential epoch, already received by the bridge, including a caller-lost reply. An empty list cannot rule out an uncertain upstream start. No restart or PID."""
                return await call(LIST_TOOL, {})

            @mcp.tool()
            async def optic_task_forget(job_id: str) -> dict:
                """Release a retained owned terminal job after reading output. Running/uncertain jobs are refused; no OS process effect or arbitrary PID."""
                return await call(FORGET_TOOL, {'job_id': job_id})

    def observe(headers):
        _, safe = observe_headers(headers)
        values = [value for key, value in headers if key.lower() == b'authorization']
        identity = state.authenticate(values[0]) if len(values) == 1 else None
        accepted = gate._current(identity) and (execution is None or execution._current(identity))
        safe['matches_dummy'] = accepted
        return identity if accepted else None, safe

    class Recheck:
        def __init__(self, app):
            self.app = app

        async def __call__(self, scope, receive, send):
            if scope['type'] == 'http':
                proof = request_proof.get()
                if (not proof or not gate._current(proof['owner'])
                        or execution is not None and not execution._current(proof['owner'])):
                    return await Guard.reject(send, 401)
            return await self.app(scope, receive, send)

    sdk = mcp.streamable_http_app(stateless_http=True, json_response=True, max_request_body_size=MAX_BODY,
        transport_security=TransportSecuritySettings(enable_dns_rebinding_protection=False)
            if allow_random_tunnel else TransportSecuritySettings(
                allowed_hosts=['127.0.0.1:*', 'localhost:*'],
                allowed_origins=['http://127.0.0.1:*', 'http://localhost:*']))
    class CombinedGuard(ProjectGuard):
        def valid_arguments(self, name, args):
            if name == DIAGNOSTIC_TOOL:
                return supervised and type(args) is dict and not args
            if name == DIRECTORY_TOOL and not gate.directory_create:
                return False
            if execution is not None and (name in TASK_TOOLS or supervised and name in SUPERVISED_TASK_TOOLS):
                try:
                    validate_arguments(name, args, execution.profiles, supervised=supervised)
                    return True
                except ValueError:
                    return False
            return super().valid_arguments(name, args)

    guard = CombinedGuard(Recheck(sdk), header_observer=observe, emit=emit, clock=clock,
                        zero_arg_tools=(), handle_tool=None, timed_diagnostics=True, lifetime=lifetime)

    guard.max_file_bytes = gate.max_file_bytes
    if supervised:
        guard.request_budget = WindowBudget(HTTP_PER_WINDOW, WINDOW_SECONDS, guard.started)
    return guard
