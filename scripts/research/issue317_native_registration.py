"""Fixed research host; outer Job is configured and verified BEFORE effect gate."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

MODES = ['auto-stop', 'legacy-no-latch', 'create-error', 'resume-error',
         'closed-before-admission', 'late-register-after-scan', 'close-after-registration',
         'close-before-resume', 'before-commit-error', 'uncommitted-start', 'client-drop',
         'direct-revoke', 'observation-uncertain', 'stop-error', 'controller-expired']

def outer_api():
    path = Path(__file__).parents[1] / 'tests' / 'doctor_pipe_supervisor.py'
    spec = importlib.util.spec_from_file_location('native_registration_outer', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def run_case(api, executable, mode, root):
    job = api.Job()
    host = None
    proof = {'mode': mode, 'complete': False, 'outer_job_cleaned': False}
    started = time.monotonic()
    env = {key: os.environ[key] for key in ('SystemRoot', 'WINDIR') if key in os.environ}
    env.update(TEMP=str(root), TMP=str(root), OPTIC317_OWNER_GATE=str(root/'gate'), OPTIC317_OWNER_MODE=mode)
    trace = root/'trace'
    try:
        with trace.open('wb') as log:
            host = subprocess.Popen([str(executable), '--exact',
                'process::authority_native_owner_tests::authority_native_owner_host',
                '--ignored', '--nocapture', '--test-threads=1'], cwd=root, env=env, stdout=log, stderr=log)
            job.assign(host)
            api.wait_for(lambda: job.active() == 1, 1, 'outer owns exact host before gate')
            proof['outer_ownership_verified_before_gate'] = True
            (root/'gate').write_text('outer-owned', encoding='ascii')
            until = time.monotonic()+12
            while not (root/'ready').exists():
                if host.poll() is not None:
                    raise RuntimeError('fixed host failed before final proof')
                if trace.stat().st_size > 128*1024:
                    raise RuntimeError('fixed trace cap')
                if time.monotonic() >= until:
                    raise TimeoutError('independent native registration deadline')
                for phase, expected in [('live', 2), ('uncertain', 1), ('suspended', 1)]:
                    if (root/(phase+'-ready')).exists() and not (root/(phase+'-confirmed')).exists():
                        api.wait_for(lambda: job.active() == expected, 2, phase+' outer witness')
                        proof['outer_active_during_'+phase] = job.active()
                        (root/(phase+'-confirmed')).write_text('verified', encoding='ascii')
                time.sleep(.002)
            state = json.loads((root/'ready').read_text(encoding='utf-8'))
            assert state['mode'] == mode and state['production_wired'] is False
            fields = ['real_registry', 'real_admission', 'active_ownership_released_after_proof']
            if mode == 'closed-before-admission':
                assert state['completion_kind'] == 'no-admission-no-native'
                assert state['admission_created'] is False and state['real_win32'] is False
                assert state['native_terminal_and_both_eof'] is False and state['stop_worker_joined'] is False
            else:
                fields += ['real_win32', 'native_terminal_and_both_eof', 'stop_worker_joined']
            for field in fields:
                assert state[field] is True, field
            assert state['combined_output_bytes'] <= 16*1024
            no_child = mode in ('create-error', 'closed-before-admission', 'late-register-after-scan',
                               'close-after-registration', 'before-commit-error', 'uncommitted-start', 'controller-expired')
            assert state['atomic_job_association'] == state['native_created'] == (not no_child)
            assert state['legacy_manual_cancel_rescue'] == (mode == 'legacy-no-latch')
            if mode in ('create-error', 'resume-error', 'before-commit-error', 'uncommitted-start', 'observation-uncertain', 'stop-error'):
                assert state['status'] == 'Failed'
            if mode == 'late-register-after-scan':
                assert state['early_scan_count'] == 0 and state['late_registration_inherited_stop']
                assert state['independent_native_completed']
            if mode == 'close-before-resume':
                assert state['resume_denied'] and state['close_published_during_native_pause']
            if mode in ('late-register-after-scan', 'close-after-registration', 'close-before-resume',
                        'before-commit-error', 'uncommitted-start'):
                assert not (root/'executed').exists()
            api.wait_for(lambda: job.active() == 1, 2, 'only fixed host after internal proof')
            proof['outer_before_host_exit'] = job.active()
            host.kill()  # exact known synthetic handle; no machine PID lookup
            host.wait(timeout=3)
            api.wait_for(lambda: job.active() == 0, 2, 'outer tree empty')
            proof.update(native=state, outer_after_host_exit=job.active(), complete=True)
    except BaseException as error:
        proof['failure_class'] = type(error).__name__
        if trace.exists():
            proof['failure_diagnostic'] = trace.read_text(encoding='utf-8', errors='replace')[-8000:].replace(str(root), '<fixture>').replace(str(Path.home()), '<operator-home>')
        raise
    finally:
        try:
            proof['outer_before_cleanup'] = job.active()
            job.close()
            proof['outer_job_cleaned'] = True
        finally:
            if host is not None:
                if host.poll() is None: host.kill()
                host.wait(timeout=3)
                host._handle.Close()
            proof['elapsed_ms'] = round((time.monotonic()-started)*1000)
            (root.parent/(mode+'.json')).write_text(json.dumps(proof, indent=2), encoding='utf-8')
    return proof

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--test-exe', required=True)
    p.add_argument('--output', required=True)
    p.add_argument('--mode', choices=MODES)
    args = p.parse_args()
    assert os.name == 'nt'
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    result = {'source_sha':os.environ.get('OPTIC_SOURCE_SHA',os.environ.get('GITHUB_SHA')),
              'run_id':os.environ.get('GITHUB_RUN_ID'), 'attempt':os.environ.get('GITHUB_RUN_ATTEMPT'),
              'production_wired':False, 'complete':False, 'cases':[]}
    try:
        with tempfile.TemporaryDirectory(prefix='optic317-native-registration-', dir=output.parent) as temporary:
            base = Path(temporary)
            for mode in ([args.mode] if args.mode else MODES):
                root = base/mode
                root.mkdir()
                try:
                    result['cases'].append(run_case(outer_api(), Path(args.test_exe).resolve(), mode, root))
                finally:
                    partial = base/(mode+'.json')
                    if partial.exists() and (not result['cases'] or result['cases'][-1]['mode'] != mode):
                        result['cases'].append(json.loads(partial.read_text(encoding='utf-8')))
        result['complete'] = True
    except BaseException as error:
        result['failure_class'] = type(error).__name__
        raise
    finally:
        output.write_text(json.dumps(result,indent=2),encoding='utf-8')
        print('OPTIC317_NATIVE_REGISTRATION '+json.dumps(result,separators=(',',':')))

if __name__ == '__main__': main()
