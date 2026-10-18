"""Owned Windows HTTP runner: renew, reconnect, Node jobs, stop/revoke and resume."""
import asyncio
import json
import logging
from pathlib import Path
import socket
import subprocess
import sys
import tempfile

from runner_edit_smoke import exercise, exercise_resume, client, TOKENS, structured

HERE = Path(__file__).resolve().parent


def port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


async def observe(url, token):
    async with client(url, token) as session:
        info = structured(await session.call_tool('optic_project_info', {}))
        assert info['ok'], info
        assert info['connection']['mode'] == 'supervised'
        assert info['connection']['remaining_seconds'] is None
        assert info['limits']['retained_jobs'] == 16
        return info


async def previous_state(url):
    async with client(url, TOKENS['A'][0]) as session:
        diagnostics = structured(await session.call_tool('optic_diagnostics', {}))['upstream']
        return diagnostics['previous_session']


async def prior_tool(url):
    async with client(url, TOKENS['A'][0]) as session:
        diagnostics = structured(await session.call_tool('optic_diagnostics', {}))['upstream']
        assert diagnostics['checkpoint_persistence_healthy'], diagnostics
        previous = diagnostics['previous_tool_checkpoint']
        assert not previous['trusted_for_authorization'], previous
        return previous


async def cycle(url):
    await exercise(url, node_mode=True, supervised=True, directories=True)
    before = await observe(url, TOKENS['A'][0])
    await asyncio.sleep(6.5)  # Real idle beyond the initial five-second core lease.
    after = await observe(url, TOKENS['A'][0])  # New SDK transport; same project/core.
    assert after['upstream']['expires_at_monotonic_ms'] > before['upstream']['expires_at_monotonic_ms']
    assert after['upstream']['capabilities'] == before['upstream']['capabilities']
    async with client(url, TOKENS['A'][0]) as session:
        listed = structured(await session.call_tool('optic_task_list', {}))
        assert listed['upstream']['jobs'] == [], listed
        diagnostics = structured(await session.call_tool('optic_diagnostics', {}))['upstream']
        assert diagnostics['recorded'] > 0 and not diagnostics['persisted'] and not diagnostics['pending_forwarding'], diagnostics


def run(binary, node):
    if sys.platform != 'win32':
        print('SUPERVISED RUNNER SKIP: native Windows generated editing.')
        return
    logging.disable(logging.CRITICAL)
    with tempfile.TemporaryDirectory(prefix='optic-supervised-runner-') as directory:
        selected_port = port()
        url = f'http://127.0.0.1:{selected_port}/mcp'
        arguments = [sys.executable, str(HERE / 'project_runner.py'), '--binary',
            str(Path(binary).resolve(strict=True)), '--output-root', directory,
            '--port', str(selected_port), '--supervised', '--lease-seconds', '5', '--allow-directories',
            '--read-page-bytes', '4096', '--node', str(Path(node).resolve(strict=True))]
        with tempfile.TemporaryFile() as diagnostics:
            child = subprocess.Popen(arguments, stdin=subprocess.PIPE, stdout=diagnostics,
                                     stderr=diagnostics, text=True)
            try:
                asyncio.run(cycle(url))
                roots = list(Path(directory).glob('alpha-*'))
                assert len(roots) == 1
                root = roots[0]
                assert (root / 'project/src/components/note.txt').read_bytes() == b'retained nested source'
                sources = {path.name: path.read_bytes() for path in (root / 'project/src').iterdir() if path.is_file()}
                child.stdin.write('proof-stats\n' * 24 + 'stop\n')
                child.stdin.flush()
                assert child.wait(timeout=20) == 0
                report = json.loads((root / 'last-stop.json').read_bytes())
                assert report['reason'] == 'operator' and report['revocation_confirmed'] and report['work_drained'], report
                assert (root / 'viewer-ready.json').is_file()
                from tool_checkpoint import NAME, MAX_BYTES
                retained = (root / NAME).read_bytes()
                assert len(retained) <= MAX_BYTES and b'content_version' not in retained
                previous_tool = json.loads(retained)
                assert previous_tool['phase'] == 'completed' and previous_tool['tool'] == 'optic_task_list'
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=10)
                child.stdin.close()
            resumed = subprocess.Popen(arguments + ['--resume-run', root.name], stdin=subprocess.PIPE,
                                       stdout=diagnostics, stderr=diagnostics, text=True)
            try:
                observed = asyncio.run(prior_tool(url))
                assert observed['event'] == previous_tool, observed
                asyncio.run(exercise_resume(url))
                assert (root / 'project/src/components/note.txt').read_bytes() == b'retained nested source'
                assert asyncio.run(previous_state(url))['revocation_confirmed'] is True
                assert not (root / 'viewer-ready.json').exists()
                assert sources == {path.name: path.read_bytes() for path in (root / 'project/src').iterdir() if path.is_file()}
                resumed.stdin.close()  # Closing local control is a fail-closed stop.
                assert resumed.wait(timeout=20) == 0
                report = json.loads((root / 'last-stop.json').read_bytes())
                assert report['reason'] == 'operator' and report['revocation_confirmed'], report
            finally:
                if resumed.poll() is None:
                    resumed.kill()
                    resumed.wait(timeout=10)
                resumed.stdin.close()
            revoked = subprocess.Popen(arguments + ['--resume-run', root.name], stdin=subprocess.PIPE,
                                       stdout=diagnostics, stderr=diagnostics, text=True)
            try:
                asyncio.run(exercise_resume(url))
                revoked.stdin.write('revoke-a\n')
                revoked.stdin.flush()
                assert revoked.wait(timeout=20) == 0
                report = json.loads((root / 'last-stop.json').read_bytes())
                assert report['reason'] == 'revoked' and report['revocation_confirmed'], report
            finally:
                if revoked.poll() is None:
                    revoked.kill()
                    revoked.wait(timeout=10)
                revoked.stdin.close()
            from owned_process_probe import OwnedCore
            crashed = subprocess.Popen(arguments + ['--resume-run', root.name], stdin=subprocess.PIPE,
                                       stdout=diagnostics, stderr=diagnostics, text=True)
            core = None
            try:
                asyncio.run(exercise_resume(url))
                core = OwnedCore(crashed, binary)
                core.crash_runner()  # Verified Popen descendant, never a caller PID.
                core.require_exit()
                assert not (root / 'viewer-ready.json').exists()
                assert not (root / 'last-stop.json').exists()
                crashed_tool = json.loads((root / NAME).read_bytes())
                assert crashed_tool['phase'] == 'completed'
                assert sources == {path.name: path.read_bytes() for path in (root / 'project/src').iterdir() if path.is_file()}
            finally:
                if crashed.poll() is None:
                    crashed.kill()
                    crashed.wait(timeout=10)
                crashed.stdin.close()
                if core is not None:
                    core.close()
            recovered = subprocess.Popen(arguments + ['--resume-run', root.name], stdin=subprocess.PIPE,
                                         stdout=diagnostics, stderr=diagnostics, text=True)
            try:
                asyncio.run(exercise_resume(url))
                assert asyncio.run(previous_state(url)) == {'state': 'previous_shutdown_unconfirmed'}
                observed = asyncio.run(prior_tool(url))
                assert observed['event'] == crashed_tool, observed
                assert not observed['event']['outcome_uncertain']
                recovered.stdin.write('stop\n')
                recovered.stdin.flush()
                assert recovered.wait(timeout=20) == 0
                report = json.loads((root / 'last-stop.json').read_bytes())
                assert report['revocation_confirmed'], report
            finally:
                if recovered.poll() is None:
                    recovered.kill()
                    recovered.wait(timeout=10)
                recovered.stdin.close()
    print('SUPERVISED RUNNER PASS: Node versioned fix/check/test, terminal release, real idle renewal/new transport, budget-exhausted stop, same-project resume, stdin EOF/revoke, core exit after parent death and crash recovery with bounded prior-tool observation (no replay).')


if __name__ == '__main__':
    run(sys.argv[1], sys.argv[2])
