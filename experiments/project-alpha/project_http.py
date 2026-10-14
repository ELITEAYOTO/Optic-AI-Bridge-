"""Six parameterized generated-project tools, behind bounded auth/schema guard."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'francestudent-probe'))
from server import Guard, request_proof, result, MAX_BODY
from probe_core import observe_headers
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
from forward_core import ForwardError
from project_schema import normalize, SchemaError


def exact_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('duplicate key')
        value[key] = item
    return value


class ProjectGuard(Guard):
    def decode_request(self, body):
        def invalid_constant(_):
            raise ValueError('nonfinite number')
        return json.loads(body, object_pairs_hook=exact_object, parse_constant=invalid_constant)

    def valid_arguments(self, name, args):
        try:
            normalize(name, args)
            return True
        except SchemaError:
            return False


def make_project_app(gate, state, emit=print, allow_random_tunnel=False):
    mcp = MCPServer('Optic-Project-Alpha', instructions=(
        'Develop a public disposable project in src using real Rust file tools. '
        'All chats on this connection share one project and grants. No process, '
        'network, delete, Git or project selection. Read returns UTF-8 text plus '
        'base64; content_version is supplied only for full reads from offset0. '
        'Create with expected_kind=absent; replace with expected_kind=content '
        'and exact expected_version. Never retry an outcome_unknown write blindly. '
        'Send calls one at a time; link.busy means the request was not admitted. '
        'Patch offsets/removals count UTF-8 bytes, not characters.'))

    async def call(name, arguments):
        proof = request_proof.get()
        try:
            payload = await gate.call(proof['owner'] if proof else None, name, arguments)
            if len(json.dumps(payload, ensure_ascii=True, allow_nan=False).encode()) > 65536:
                raise ForwardError('project.output_bound')
            return result(name, {'ok': True, 'generated_project_only': True, 'upstream': payload})
        except ForwardError as error:
            return result(name, {'ok': False, 'code': str(error)})

    @mcp.tool()
    async def optic_project_info() -> dict:
        """Inspect the one shared generated project and locally granted capabilities."""
        return await call('optic_project_info', {})

    @mcp.tool()
    async def optic_fs_list(path: str = 'src', cursor: int = 0, limit: int = 32) -> dict:
        """List one src directory, at most32 entries. Pass next_cursor for the next page."""
        return await call('optic_fs_list', {'path': path, 'cursor': cursor, 'limit': limit})

    @mcp.tool()
    async def optic_fs_read(path: str, offset: int = 0, max_bytes: int = 8192) -> dict:
        """Read up to8192 bytes. Full offset0 reads return the exact version for editing."""
        return await call('optic_fs_read', {'path': path, 'offset': offset, 'max_bytes': max_bytes})

    @mcp.tool()
    async def optic_fs_write(path: str, content: str, expected_kind: str, expected_version: str | None = None) -> dict:
        """Write UTF-8 text <=8192bytes in src. absent=create; content requires exact expected_version."""
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
        """Literal text search in the first8192bytes of one src file, max16 hits. No regex or tree scan."""
        return await call('optic_fs_search', {'path': path, 'text': text, 'limit': limit})

    def observe(headers):
        _, safe = observe_headers(headers)
        values = [value for key, value in headers if key.lower() == b'authorization']
        identity = state.authenticate(values[0]) if len(values) == 1 else None
        accepted = gate._current(identity)
        safe['matches_dummy'] = accepted
        return identity if accepted else None, safe

    class Recheck:
        def __init__(self, app):
            self.app = app

        async def __call__(self, scope, receive, send):
            if scope['type'] == 'http':
                proof = request_proof.get()
                if not proof or not gate._current(proof['owner']):
                    return await Guard.reject(send, 401)
            return await self.app(scope, receive, send)

    sdk = mcp.streamable_http_app(stateless_http=True, json_response=True, max_request_body_size=MAX_BODY,
        transport_security=TransportSecuritySettings(enable_dns_rebinding_protection=False)
            if allow_random_tunnel else TransportSecuritySettings(
                allowed_hosts=['127.0.0.1:*', 'localhost:*'],
                allowed_origins=['http://127.0.0.1:*', 'http://localhost:*']))
    return ProjectGuard(Recheck(sdk), header_observer=observe, emit=emit,
                        zero_arg_tools=(), handle_tool=None, timed_diagnostics=True)
