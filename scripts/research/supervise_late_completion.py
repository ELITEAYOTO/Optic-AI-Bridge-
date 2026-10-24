"""Hosted-only fixed fixtures. Outer cleanup is never credited as success."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

CASES = ('stdout', 'stderr', 'both', 'stdin', 'fast', 'broken-pipe',
         'late-output-limit', 'late-broken-pipe')

def wait(predicate, seconds, message):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        if predicate():
            return
        time.sleep(.005)
    raise TimeoutError(message)

def group_members(group):
    # Synthetic group leader remains alive during this observation. Record only
    # a count, never command lines or unrelated process identifiers.
    count = 0
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / 'stat').read_text()
            fields = stat[stat.rfind(')')+2:].split()
            if int(fields[2]) == group:
                count += 1
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    return count

def worker(args):
    root = Path(args.output).resolve()
    wait(lambda: (root/'gate').exists(), 10, 'effect gate')
    started = time.monotonic()
    env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR') if k in os.environ}
    env.update(TEMP=str(root), TMP=str(root), OPTIC317_LATE_ROOT=str(root),
               OPTIC317_LATE_MODE=args.case)
    with (root/'test.log').open('wb') as log:
        test = subprocess.Popen([args.test_exe, '--exact', 'classifies_result',
            '--ignored', '--nocapture', '--test-threads=1'], env=env,
            cwd=root, stdout=log, stderr=log)
        code = test.wait(timeout=10)
    result = dict(exit=code, elapsed_ms=round((time.monotonic()-started)*1000))
    (root/'finished.json').write_text(json.dumps(result), encoding='utf-8')
    # Keep the process-group identity alive until its owner observes host-only.
    wait(lambda: (root/'exit-gate').exists(), 10, 'owner acknowledgement')

def outer_api():
    path = Path(__file__).parents[1]/'tests/doctor_pipe_supervisor.py'
    spec = importlib.util.spec_from_file_location('outer', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def case(exe, root, mode):
    root.mkdir()
    job = outer_api().Job() if os.name == 'nt' else None
    host = None
    evidence = dict(mode=mode, passed=False, cleanup_proven=False)
    env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR') if k in os.environ}
    env.update(TEMP=str(root), TMP=str(root), PYTHONUTF8='1')
    try:
        with (root/'worker.log').open('wb') as log:
            host = subprocess.Popen([sys.executable, str(Path(__file__).resolve()),
                '--worker', '--test-exe', str(exe), '--output', str(root),
                '--case', mode], cwd=root, env=env, stdout=log, stderr=log,
                start_new_session=(os.name != 'nt'))
            if job:
                job.assign(host)
                wait(lambda: job.active() == 1, 2, 'outer owns worker')
            else:
                assert os.getpgid(host.pid) == host.pid
                wait(lambda: group_members(host.pid) == 1, 2, 'owned group leader only')
            evidence['outer_owned_before_gate'] = True
            (root/'gate').write_text('owned', encoding='ascii')
            wait(lambda: (root/'finished.json').exists() or host.poll() is not None,
                 15, 'fixed test did not complete')
            assert host.poll() is None, 'group owner unexpectedly exited'
            evidence.update(json.loads((root/'finished.json').read_text(encoding='utf-8')))
            count = (lambda: job.active()) if job else (lambda: group_members(host.pid))
            wait(lambda: count() == 1, 2, 'fixture descendants remain BEFORE outer cleanup')
            evidence['members_before_owner_exit'] = count()
            text = (root/'test.log').read_text(encoding='utf-8', errors='replace')
            assert (root/'test.log').stat().st_size < 64*1024
            evidence['receipt'] = [line for line in text.splitlines()
                                   if 'OPTIC317_CLASSIFICATION' in line]
            evidence['late_success_counterexample'] = (
                'LATE_SUCCESS_MUST_BE_TIMEOUT' in text and 'Ok(HardenedCommandOutput' in text)
            evidence['passed'] = evidence['exit'] == 0 and '1 passed; 0 failed' in text
            (root/'exit-gate').write_text('host-only observed', encoding='ascii')
            host.wait(timeout=3)
            wait(lambda: count() == 0, 2, 'outer not empty after normal owner exit')
            evidence['cleanup_proven'] = True
    except BaseException as error:
        evidence['failure'] = type(error).__name__ + ': ' + str(error)
    finally:
        try:
            if job:
                # Only the owned native Job. An already empty observation above
                # must exist before this can count as successful fixture cleanup.
                job.close()
            elif host is not None and host.poll() is None:
                # Group identity is still anchored by our live Popen leader.
                os.killpg(host.pid, signal.SIGKILL)
                host.wait(timeout=3)
        finally:
            if host is not None:
                if host.poll() is None:
                    host.kill()
                host.wait(timeout=3)
                if os.name == 'nt':
                    host._handle.Close()
            evidence['recovery_attempted'] = True
            (root/'case.json').write_text(json.dumps(evidence, indent=2), encoding='utf-8')
    return evidence

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--build-json')
    p.add_argument('--output', required=True)
    p.add_argument('--worker', action='store_true')
    p.add_argument('--test-exe')
    p.add_argument('--case', choices=CASES)
    args = p.parse_args()
    if args.worker:
        worker(args)
        return
    assert os.environ.get('GITHUB_ACTIONS') == 'true', 'hosted validation only'
    items = [json.loads(line) for line in Path(args.build_json).read_text(encoding='utf-8-sig').splitlines()]
    artifacts = [j for j in items if j.get('reason') == 'compiler-artifact'
                 and j.get('target', {}).get('name') == 'issue317_late_completion'
                 and j.get('executable')]
    assert len(artifacts) == 1
    exe = Path(artifacts[0]['executable']).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = dict(sha=os.environ['GITHUB_SHA'], run=os.environ['GITHUB_RUN_ID'],
                  attempt=os.environ['GITHUB_RUN_ATTEMPT'], platform=sys.platform,
                  complete=False, cases=[])
    try:
        for mode in CASES:
            result = case(exe, output/mode, mode)
            report['cases'].append(result)
            if not result['cleanup_proven']:
                break
        report['complete'] = len(report['cases']) == len(CASES) and all(
            c['passed'] and c['cleanup_proven'] for c in report['cases'])
    finally:
        (output/'manifest.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print('OPTIC317_LATE_CLASSIFICATION ' + json.dumps(report))
    if not report['complete']:
        raise SystemExit(1)

if __name__ == '__main__':
    main()
