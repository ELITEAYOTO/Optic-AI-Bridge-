"""One selected real session; serialized retained forwarding and bounded results."""
import asyncio
import base64

from mcp.shared.exceptions import MCPError
from forward_core import ForwardGate, ForwardError
from project_schema import normalize, SchemaError, MAX_CONTENT, version

SAFE_UPSTREAM_ERRORS = {
    'optic.precondition_failed', 'optic.policy_denied', 'optic.session_inactive',
    'optic.invalid_workspace_path', 'optic.path_not_found', 'optic.not_file',
    'optic.not_directory', 'optic.invalid_patch', 'optic.fs_read_limit_exceeded',
    'optic.fs_list_limit_exceeded', 'optic.fs_mutation_limit_exceeded',
    'optic.mutation_platform_unsupported', 'optic.invalid_mutation_target',
}
MUTATIONS = {'optic_fs_write', 'optic_fs_patch'}


class ProjectGate(ForwardGate):
    def __init__(self, registry, identity, upstream, deadline=15, read_page_bytes=1024):
        if type(read_page_bytes) is not int or read_page_bytes not in (1024, 4096):
            raise ValueError("project.invalid_read_page_bytes")
        self.read_page_bytes = read_page_bytes
        super().__init__(registry, identity, upstream, capacity=1, deadline=deadline)

    async def call(self, identity, tool, arguments):
        self._loop()
        if not self._current(identity):
            raise ForwardError('link.inactive')
        try:
            name, args = normalize(tool, arguments)
        except SchemaError:
            raise ForwardError('project.invalid_arguments') from None
        if self.pending:
            raise ForwardError('link.busy')
        search = (arguments['text'], arguments.get('limit', 16)) if tool == 'optic_fs_search' else None
        task = asyncio.create_task(self._run_project(identity, tool, name, args, search))
        self.pending.add(task)
        task.add_done_callback(self._complete)
        try:
            async with asyncio.timeout(self.deadline):
                return await asyncio.shield(task)
        except TimeoutError:
            raise ForwardError('project.outcome_unknown' if tool in MUTATIONS else 'link.response_timeout') from None

    async def _run_project(self, identity, tool, name, arguments, search):
        dispatched = False
        try:
            if not self._current(identity):
                raise ForwardError('link.inactive')
            if tool == 'optic_fs_patch':
                current = await self.upstream('fs_read', {'path': arguments['path'],
                    'offset': 0, 'max_bytes': MAX_CONTENT + 1})
                raw = base64.b64decode(current['data'], validate=True)
                inserted = base64.b64decode(arguments['insert_base64'], validate=True)
                if (not current['eof'] or len(raw) > MAX_CONTENT
                        or arguments['offset'] + arguments['remove_bytes'] > len(raw)
                        or len(raw) - arguments['remove_bytes'] + len(inserted) > MAX_CONTENT):
                    raise ForwardError('project.patch_limit')
                # Recheck after the preparatory read; Rust still owns the exact
                # mutation precondition and must compare the supplied version.
                if not self._current(identity):
                    raise ForwardError('link.inactive')
            dispatched = True
            read_arguments = ({'path': arguments['path'], 'offset': 0,
                'max_bytes': MAX_CONTENT + 1} if tool == 'optic_fs_read' else arguments)
            response = await self.upstream(name, read_arguments)
            if not self._current(identity):
                raise ForwardError('project.outcome_unknown' if tool in MUTATIONS else 'link.inactive')
            if tool in ('optic_fs_read', 'optic_fs_search'):
                raw = base64.b64decode(response['data'], validate=True)
                if tool == 'optic_fs_read':
                    if not response['eof'] or len(raw) > MAX_CONTENT:
                        raise ForwardError('project.read_limit')
                    observed_version = version(response.get('content_version'))
                    offset = arguments['offset']
                    end = min(len(raw), offset + min(arguments['max_bytes'], self.read_page_bytes))
                    if offset > len(raw):
                        raise ForwardError('project.invalid_read_offset')
                    try:
                        raw.decode('utf-8', errors='strict')
                    except UnicodeError:
                        page = {'text': None, 'encoding': 'base64',
                            'data': base64.b64encode(raw[offset:end]).decode('ascii')}
                    else:
                        # A bad text offset is not evidence of a binary file.
                        try:
                            raw[:offset].decode('utf-8', errors='strict')
                        except UnicodeError:
                            raise ForwardError('project.invalid_read_offset') from None
                        while end > offset:
                            try:
                                raw[offset:end].decode('utf-8', errors='strict')
                                break
                            except UnicodeError:
                                end -= 1
                        if end == offset and offset < len(raw):
                            raise ForwardError('project.read_character_bound')
                        page = {'text': raw[offset:end].decode('utf-8')}
                    return {'path': response['path'],
                        'content_version': observed_version,
                        'offset': offset, 'next_offset': end, 'eof': end == len(raw),
                        'file_bytes': len(raw), **page}
                try:
                    content = raw.decode('utf-8', errors='strict')
                except UnicodeError:
                    if search:
                        raise ForwardError('project.text_encoding') from None
                    return {**response, 'text': None}
                if search:
                    needle, limit = search
                    hits, offset, more_matches = [], 0, False
                    for index, full_line in enumerate(content.splitlines(keepends=True), 1):
                        line = full_line.rstrip('\r\n\v\f\x1c\x1d\x1e\x85\u2028\u2029')
                        next_offset = offset + len(full_line.encode('utf-8'))
                        if needle in line:
                            if len(hits) == limit:
                                more_matches = True
                                break
                            hits.append({'line': index, 'text': line[:256],
                                         'text_truncated': len(line) > 256,
                                         'offset': offset, 'next_offset': next_offset})
                        offset = next_offset
                    return {'path': response['path'], 'matches': hits,
                            'content_version': (version(response.get('content_version'))
                                                if response['eof'] else None),
                            'more_matches': more_matches,
                            'searched_bytes': len(raw), 'eof': response['eof']}
                return {**response, 'text': content}
            return response
        except MCPError as error:
            code = error.message if error.message in SAFE_UPSTREAM_ERRORS else (
                'project.outcome_unknown' if dispatched and tool in MUTATIONS else 'project.upstream_failed')
            raise ForwardError(code) from None
        except ForwardError:
            raise
        except Exception:
            raise ForwardError('project.outcome_unknown' if dispatched and tool in MUTATIONS else 'project.upstream_failed') from None
