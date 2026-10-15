"""Bounded reproducible MCP receipt proof; never logs URLs, auth or raw results."""
import argparse
import asyncio
from datetime import datetime, timezone
from contextvars import ContextVar
import hashlib
import json
import logging
from pathlib import Path
import re
import sys
import time
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
for directory in ('real-link-admission', 'credential-lifecycle'):
    sys.path.insert(0, str(HERE.parent / directory))
from contextlib import asynccontextmanager
from http_smoke import headers
import httpx2
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client
from lifecycle_core import TOKENS
from project_results import structured

HTTP_STATUS_TRACE = ContextVar('diagnostic_http_status', default=None)


@asynccontextmanager
async def client(url, token):
    statuses = []
    trace = HTTP_STATUS_TRACE.set(statuses)
    async def observe(response):
        if len(statuses) < 32:
            statuses.append(response.status_code)
    try:
        async with httpx2.AsyncClient(headers=headers(token), timeout=20, follow_redirects=False,
                                     event_hooks={'response': [observe]}) as http:
            async with streamable_http_client(url, http_client=http) as (read, write):
                async with ClientSession(read, write, read_timeout_seconds=20) as session:
                    await session.initialize()
                    yield session
    finally:
        HTTP_STATUS_TRACE.reset(trace)


def endpoint(value):
    url = urlsplit(value)
    if (url.username or url.password or url.query or url.fragment or url.path != '/mcp'
            or not url.hostname or not (url.scheme == 'https'
                or url.scheme == 'http' and url.hostname in ('127.0.0.1', 'localhost'))):
        raise ValueError('diagnostic.invalid_url')
    return value


async def measure(url, quick=False, idle=False):
    records = []
    run_ids = set()
    sizes, delays, repetitions = ((128, 1024), (0,), 1) if quick else ((128, 1024, 4096, 8192), (0, 1, 3, 10), 3)
    expected_tools = {'diagnostic_sample'}
    # One session preserves the actual client idle/reconnect behavior. This is
    # the same JSON transport baseline; no fabricated progress or heartbeats.
    try:
        async with client(endpoint(url), TOKENS['A'][0]) as session:
            if {tool.name for tool in (await session.list_tools()).tools} != expected_tools:
                return {'version': 1, 'scope': 'synthetic-only', 'records': [], 'failure': 'wrong_tool_surface'}
            for repeat in range(repetitions):
                for size in sizes:
                    for delay in delays:
                        records.append(await sample(session, size, delay, repeat, run_ids))
                        if not records[-1]['ok']:
                            return {'version': 1, 'scope': 'synthetic-only', 'records': records, 'failure': 'sample_failed'}
            if idle:
                for gap in (15, 45, 75, 100):
                    await asyncio.sleep(gap)
                    record = await sample(session, 128, 0, 0, run_ids)
                    record['idle_seconds'] = gap
                    records.append(record)
                    if not record['ok']:
                        break
    except Exception:
        # Unexpected client/HTTP/SDK errors are unknown transport failures,
        # never echoed because exception messages may include auth-bearing URLs.
        return {'version': 1, 'scope': 'synthetic-only', 'records': records, 'failure': 'connection_or_protocol_error'}
    return {'version': 1, 'scope': 'synthetic-only', 'records': records,
            'failure': None if all(row['ok'] for row in records) else 'sample_failed'}


async def sample(session, size, delay, repeat, run_ids):
    start = time.monotonic()
    record = {'size_bytes': size, 'delay_seconds': delay, 'repeat': repeat,
              'started_utc': datetime.now(timezone.utc).isoformat(timespec='milliseconds'), 'ok': False}
    statuses = HTTP_STATUS_TRACE.get()
    if statuses is not None:
        statuses.clear()
    received = False
    try:
        async with asyncio.timeout(16):
            raw = await session.call_tool('diagnostic_sample', {'size_bytes': size, 'delay_seconds': delay})
        received = True
        value = structured(raw)
        proof = value['probe_invocation']
        text = value['text']
        run_id = value['diagnostic_run']
        if (value.get('ok') is not True or value.get('authority') != 'none'
                or not isinstance(text, str) or len(text) != size or text != 'd' * size
                or value.get('size_bytes') != size or value.get('delay_seconds') != delay
                or value.get('sha256') != hashlib.sha256(text.encode('ascii')).hexdigest()
                or not isinstance(run_id, str) or re.fullmatch('[0-9a-f]{16}', run_id) is None
                or not isinstance(value.get('build_sha256'), str) or re.fullmatch('[0-9a-f]{64}', value['build_sha256']) is None
                or not isinstance(proof.get('marker'), str) or re.fullmatch('[0-9a-f]{16}', proof['marker']) is None
                or not isinstance(proof.get('server_instance'), str) or re.fullmatch('[0-9a-f]{12}', proof['server_instance']) is None
                or type(proof.get('request_sequence')) is not int or not 1 <= proof['request_sequence'] <= 512):
            raise ValueError('diagnostic.reply_mismatch')
        run_ids.add(run_id)
        if len(run_ids) != 1:
            raise ValueError('diagnostic.server_changed')
        record.update(ok=True, diagnostic_run=run_id, marker=proof['marker'],
                      build_sha256=value['build_sha256'],
                      server_instance=proof['server_instance'], request_sequence=proof['request_sequence'],
                      verified_bytes=len(text.encode('ascii')))
    except TimeoutError:
        record['failure'] = 'client_deadline'
    except Exception:
        record['failure'] = 'reply_verification_failed' if received else 'tool_or_transport_error'
    record['http_statuses'] = list(statuses) if statuses is not None else []
    record['received_utc'] = datetime.now(timezone.utc).isoformat(timespec='milliseconds')
    record['client_elapsed_ms'] = round((time.monotonic() - start) * 1000, 3)
    return record


def save_report(path, report):
    raw = json.dumps(report, sort_keys=True, indent=2).encode('ascii')
    if len(raw) > 131072 or len(report['records']) > 64:
        raise ValueError('diagnostic.report_bound')
    # Explicit operator file, create-only: no automatic overwrite or unbounded log.
    with Path(path).open('xb') as stream:
        stream.write(raw)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', type=endpoint, required=True)
    parser.add_argument('--report', required=True)
    parser.add_argument('--quick', action='store_true')
    parser.add_argument('--idle', action='store_true')
    args = parser.parse_args()
    logging.disable(logging.CRITICAL)
    report = asyncio.run(measure(args.url, args.quick, args.idle))
    save_report(args.report, report)
    print(json.dumps({'samples': len(report['records']), 'passed': sum(row['ok'] for row in report['records']),
                      'failure': report['failure']}))
    raise SystemExit(0 if report['failure'] is None and all(row['ok'] for row in report['records']) else 1)
