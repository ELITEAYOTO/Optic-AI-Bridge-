"""Six fixed existing tests, pre-armed outer Job; no arbitrary process/PID API."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time

TESTS = ["process::tests::timeout_terminates_owned_process",
         "process::tests::explicit_stop_terminates_owned_process",
         "process::tests::output_overflow_is_bounded_and_terminates_process"]

def run_fixed(exe, root, index, test):
    started = time.monotonic()
    log = root / (str(index)+".log")
    with log.open('wb') as output:
        child = subprocess.Popen([exe, '--exact', test, '--nocapture', '--test-threads=1'],
                                 stdout=output, stderr=output, cwd=root)
        code = child.wait(timeout=8)
    assert log.stat().st_size < 64*1024
    text = log.read_text(encoding='utf-8', errors='replace')
    assert code == 0 and '1 passed; 0 failed' in text, text[-4000:]
    return dict(index=index, test=test, exit=code, elapsed_ms=round((time.monotonic()-started)*1000),
                diagnostics=[line for line in text.splitlines() if 'OPTIC_317_' in line])

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--test-exe',required=True)
    p.add_argument('--output',required=True)
    p.add_argument('--worker', action='store_true')
    args=p.parse_args()
    assert os.name=='nt'
    root=Path(args.output).resolve()
    if args.worker:
        deadline=time.monotonic()+10
        while not (root/'gate').exists():
            assert time.monotonic()<deadline
            time.sleep(.005)
        results=[]
        try:
            for i in range(3): results.append(run_fixed(args.test_exe,root,i,TESTS[0]))
            with ThreadPoolExecutor(max_workers=3) as pool:
                futures=[pool.submit(run_fixed,args.test_exe,root,3+i,t) for i,t in enumerate(TESTS)]
                for future in futures: results.append(future.result())
        finally:
            (root/'worker-results.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
        return
    root.mkdir(parents=True,exist_ok=False)
    spec=importlib.util.spec_from_file_location('outer',Path(__file__).parents[1]/'tests/doctor_pipe_supervisor.py')
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    job=helper.Job();worker=None
    report=dict(sha=os.environ.get('OPTIC_SOURCE_SHA') or os.environ.get('GITHUB_SHA'),
                windows=platform.platform(),complete=False,outer_cleanup=False,
                plan='3 sequential timeout then 3 concurrent fixed native tests',cases=[])
    env={k:os.environ[k] for k in ('SystemRoot','WINDIR') if k in os.environ}
    env.update(TEMP=str(root),TMP=str(root))
    try:
        with (root/'supervisor.log').open('wb') as log:
            worker=subprocess.Popen([sys.executable,str(Path(__file__).resolve()),'--worker',
                '--test-exe',str(Path(args.test_exe).resolve()),'--output',str(root)],
                stdout=log,stderr=log,env=env,cwd=root)
            job.assign(worker)
            helper.wait_for(lambda:job.active()==1,2,'owns fixed worker before gate')
            report['ownership_before_gate']=True
            (root/'gate').write_text('owned',encoding='ascii')
            worker.wait(timeout=30)
            report['worker_exit']=worker.returncode
            results=root/'worker-results.json'
            if results.exists(): report['cases']=json.loads(results.read_text(encoding='utf-8'))
            report['active_before_outer_cleanup']=job.active()
            assert worker.returncode==0 and len(report['cases'])==6
            helper.wait_for(lambda:job.active()==0,2,'all fixture processes gone BEFORE outer cleanup')
            report['active_before_outer_cleanup']=job.active()
            report['complete']=True
    finally:
        try:
            job.close();report['outer_cleanup']=True
        finally:
            if worker is not None:
                if worker.poll() is None:worker.kill()
                worker.wait(timeout=3);worker._handle.Close()
            (root/'manifest.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
            print('OPTIC317_PHYSICAL_CONFIRMATION '+json.dumps(report))

if __name__=='__main__':main()
