"""Fixed synthetic libtest processes; outer Job is emergency containment only.

Run only on hosted Windows. A live outer Job count is checked BEFORE cleanup,
so its termination cannot manufacture successful inner Job cleanup.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import subprocess
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--test-exe', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    assert os.name == 'nt' and os.environ.get('GITHUB_ACTIONS') == 'true'
    helper_path = Path(__file__).parents[1] / 'tests/doctor_pipe_supervisor.py'
    spec = importlib.util.spec_from_file_location('outer', helper_path)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = dict(sha=os.environ.get('GITHUB_SHA'), windows=platform.platform(),
                  research_only=True, complete=False, cases=[])
    try:
        for red, mode in [(True, 'after-create')] + [(False, m) for m in (
                'before-create', 'after-create', 'offered', 'job-only', 'accepted-before-ack')]:
            root = output / (('red-' if red else '') + mode)
            root.mkdir()
            (root / 'mode').write_text(mode, encoding='ascii')
            env = os.environ.copy()
            env['OPTIC317_TRANSFER_ROOT'] = str(root)
            if red:
                env['OPTIC317_RED'] = '1'
            else:
                env.pop('OPTIC317_RED', None)
            outer = helper.Job()
            host = None
            evidence = dict(mode=mode, negative_control=red, outer_cleanup_success=False)
            report['cases'].append(evidence)
            try:
                with (root / 'host.log').open('wb') as log:
                    host = subprocess.Popen([str(Path(args.test_exe).resolve()), '--exact',
                        'session_registry::owned_native_research_tests::launcher_crash::service_host',
                        '--ignored', '--nocapture', '--test-threads=1'], env=env,
                        stdout=log, stderr=log)
                    outer.assign(host)
                    assert outer.active() == 1
                    (root / 'outer-gate').write_text('outer-owned-before-launch', encoding='ascii')
                    helper.wait_for(lambda: (root / 'crash-ready').exists() or host.poll() is not None,
                                    15, 'native transfer phase')
                    assert host.poll() is None, 'host exited before crash boundary'
                    phase = json.loads((root / 'crash-ready').read_text())
                    assert outer.active() == phase['expected_outer']
                    evidence['before_crash'] = phase
                    (root / 'crash-confirmed').write_text('count-observed', encoding='ascii')
                    helper.wait_for(lambda: (root / 'source-exit').exists() or host.poll() is not None,
                                    6, 'owned launcher handle signalled')
                    assert (root / 'source-exit').exists()
                    # No call to TerminateJobObject on outer until finally.
                    retained = mode in ('job-only', 'accepted-before-ack')
                    helper.wait_for(lambda: outer.active() == (2 if retained else 1),
                                    4, 'survivor count before any outer cleanup')
                    evidence['survivors_before_outer_cleanup'] = outer.active()
                    (root / 'survivor-confirmed').write_text('independent-count', encoding='ascii')
                    if red:
                        host.wait(timeout=8)
                        assert host.returncode != 0
                        assert 'guardian vanished before retaining uncertain owner' in (
                            root / 'host.log').read_text(errors='replace')
                        evidence['outcome'] = 'FirstRedGuardianLost'
                    else:
                        name = 'complete-ready' if retained else 'pending-ready'
                        helper.wait_for(lambda: (root / name).exists() or host.poll() is not None,
                                        8, 'retained ownership evidence')
                        assert host.poll() is None
                        proof = json.loads((root / name).read_text())
                        assert proof['original_admission_retained']
                        assert proof['quota_retained_before_proof'] == 1
                        assert proof['revoke_pending_before_proof']
                        assert proof['termination_proven_by_guardian'] == retained
                        assert proof['quota_released'] == retained
                        helper.wait_for(lambda: outer.active() == 1, 4, 'inner cleanup before outer')
                        evidence['proof'] = proof
                        evidence['outcome'] = proof['outcome']
                        evidence['inner_cleanup_attributed_to_outer'] = False
            finally:
                # Only campaign-created Job and its assigned descendants.
                # This is recovery, never credited to the tested guardian.
                try:
                    outer.close()  # includes positive zero-members query
                    evidence['outer_cleanup_success'] = True
                finally:
                    if host is not None:
                        if host.poll() is None:
                            host.kill()
                        host.wait(timeout=3)
                        host._handle.Close()
        report['complete'] = True
    finally:
        (output / 'manifest.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
