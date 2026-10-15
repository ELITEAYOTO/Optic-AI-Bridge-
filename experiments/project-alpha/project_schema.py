"""Finite alpha tool boundary; all file paths stay in generated src."""
import base64
import re

TOOLS = ('optic_project_info', 'optic_fs_list', 'optic_fs_read',
         'optic_fs_write', 'optic_fs_patch', 'optic_fs_search')
MAX_CONTENT = 8192
DIRECTORY_TOOL = 'optic_fs_mkdir'
MAX_READ = 8192
PATH = re.compile(r'src(?:/[A-Za-z0-9_-][A-Za-z0-9_.-]{0,63}){0,7}\Z', re.ASCII)
VERSION = re.compile(r'[0-9a-f]{64}\Z', re.ASCII)
RESERVED = {'con', 'prn', 'aux', 'nul', *(f'com{i}' for i in range(1, 10)), *(f'lpt{i}' for i in range(1, 10))}


class SchemaError(Exception):
    pass


def integer(value, minimum, maximum):
    if type(value) is not int or not minimum <= value <= maximum:
        raise SchemaError('project.invalid_arguments')
    return value


def path(value, file=False):
    if (type(value) is not str or len(value) > 512 or not PATH.fullmatch(value)
            or (file and value == 'src') or any(segment.endswith('.')
                or segment.split('.')[0].lower() in RESERVED for segment in value.split('/'))):
        raise SchemaError('project.invalid_arguments')
    return value


def text(value, limit, empty=True):
    if type(value) is not str or len(value) > limit or (not empty and not value):
        raise SchemaError('project.invalid_arguments')
    try:
        encoded = value.encode('utf-8', errors='strict')
    except UnicodeError:
        raise SchemaError('project.invalid_arguments') from None
    if len(encoded) > limit:
        raise SchemaError('project.invalid_arguments')
    return encoded


def version(value):
    if type(value) is not str or not VERSION.fullmatch(value):
        raise SchemaError('project.invalid_arguments')
    return value


def normalize(tool, args):
    if type(tool) is not str or tool not in (*TOOLS, DIRECTORY_TOOL) or type(args) is not dict:
        raise SchemaError('project.invalid_arguments')
    required, optional = {
        'optic_project_info': (set(), set()),
        'optic_fs_list': (set(), {'path', 'cursor', 'limit'}),
        'optic_fs_read': ({'path'}, {'offset', 'max_bytes'}),
        'optic_fs_write': ({'path', 'content', 'expected_kind'}, {'expected_version'}),
        'optic_fs_patch': ({'path', 'expected_version', 'offset', 'remove_bytes', 'insert'}, set()),
        'optic_fs_search': ({'path', 'text'}, {'limit'}),
        DIRECTORY_TOOL: ({'path', 'expected_kind'}, set()),
    }[tool]
    if not required <= set(args) or not set(args) <= required | optional:
        raise SchemaError('project.invalid_arguments')
    if tool == 'optic_project_info':
        return 'session_info', {}
    if tool == DIRECTORY_TOOL:
        target = path(args['path'])
        if args['expected_kind'] != 'absent':
            raise SchemaError('project.invalid_arguments')
        return 'fs_create_directory', {'path': target, 'expected': 'absent'}
    if tool == 'optic_fs_list':
        return 'fs_list', {'path': path(args.get('path', 'src')),
            'cursor': integer(args.get('cursor', 0), 0, 4096),
            'limit': integer(args.get('limit', 32), 1, 32)}
    target = path(args['path'], file=True)
    if tool == 'optic_fs_read':
        return 'fs_read', {'path': target, 'offset': integer(args.get('offset', 0), 0, MAX_CONTENT),
            'max_bytes': integer(args.get('max_bytes', MAX_READ), 1, MAX_READ)}
    if tool == 'optic_fs_write':
        expected_kind = args['expected_kind']
        if expected_kind == 'absent' and args.get('expected_version') is None:
            expected = {'kind': 'absent'}
        elif expected_kind == 'content' and 'expected_version' in args:
            expected = {'kind': 'content', 'version_hex': version(args['expected_version'])}
        else:
            raise SchemaError('project.invalid_arguments')
        return 'fs_write', {'path': target, 'content_base64': base64.b64encode(
            text(args['content'], MAX_CONTENT)).decode('ascii'), 'expected': expected}
    if tool == 'optic_fs_patch':
        return 'fs_apply_patch', {'path': target, 'expected_version': version(args['expected_version']),
            'offset': integer(args['offset'], 0, MAX_CONTENT),
            'remove_bytes': integer(args['remove_bytes'], 0, MAX_CONTENT),
            'insert_base64': base64.b64encode(text(args['insert'], MAX_CONTENT)).decode('ascii')}
    text(args['text'], 128, empty=False)
    integer(args.get('limit', 16), 1, 16)
    return 'fs_read', {'path': target, 'offset': 0, 'max_bytes': MAX_READ}
