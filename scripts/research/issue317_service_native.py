"""Own the scoped #317 service test before any child effect; always recover."""
import argparse,importlib.util,json,os,subprocess,tempfile,time
from pathlib import Path

def main():
 p=argparse.ArgumentParser()
 p.add_argument('--test-exe',required=True)
 p.add_argument('--output',required=True)
 mode=p.add_mutually_exclusive_group()
 mode.add_argument('--suspended-revoke',action='store_true')
 mode.add_argument('--kernel',action='store_true')
 mode.add_argument('--matrix',action='store_true')
 a=p.parse_args()
 assert os.name=='nt'
 source=Path(__file__).parents[1]/'tests'/'doctor_pipe_supervisor.py'
 spec=importlib.util.spec_from_file_location('outer',source)
 outer=importlib.util.module_from_spec(spec);spec.loader.exec_module(outer)
 result={'complete':False,'source_sha':os.environ.get('OPTIC_SOURCE_SHA',os.environ.get('GITHUB_SHA')),
         'production_wired':False,'claim':'private read-service kernel or suspended phase; not production GitRead integration'}
 dest=Path(a.output);dest.parent.mkdir(parents=True,exist_ok=True)
 temporary=tempfile.TemporaryDirectory(prefix='optic317-service-')
 root=Path(temporary.name);gate=root/'go';job=outer.Job();child=None
 start=time.monotonic()
 try:
  env=os.environ.copy();env['OPTIC317_SERVICE_GATE']=str(gate)
  env['OPTIC317_PYTHON']=__import__('sys').executable
  env['OPTIC317_FIXTURE']=str(Path(__file__).with_name('issue317_baseline_fixture.py').resolve())
  log=root/'log'
  with log.open('wb') as out:
   child=subprocess.Popen([a.test_exe,'--exact',
     ('process::read_service::tests::service_kernel_outcome_and_pipe_matrix' if a.matrix else
      'job::prepared_read_command::tests::revocation_between_assignment_and_resume' if a.suspended_revoke else
      'process::read_service::tests::service_kernel_retains_owners_through_request_loss_and_shutdown'),
     '--ignored','--nocapture'],env=env,stdout=out,stderr=out)
   job.assign(child);gate.write_text('owned',encoding='ascii')
   end=time.monotonic()+15
   while child.poll() is None:
    if not a.suspended_revoke and gate.with_suffix('.shutdown-lock-ready').exists() and not gate.with_suffix('.shutdown-lock-observed').exists():
     result['active_with_bookkeeping_lock']=job.active()
     assert result['active_with_bookkeeping_lock']==2
     gate.with_suffix('.shutdown-lock-observed').write_text('two members observed',encoding='ascii')
    if not a.suspended_revoke and gate.with_suffix('.shutdown-lock-stop').exists() and not gate.with_suffix('.shutdown-lock-confirmed').exists():
     outer.wait_for(lambda:job.active()==1,2,'native stop while bookkeeping locked')
     result['active_after_shutdown_with_bookkeeping_lock']=job.active()
     gate.with_suffix('.shutdown-lock-confirmed').write_text('host only',encoding='ascii')
    if not a.suspended_revoke and gate.with_suffix('.kernel-stopped').exists() and not gate.with_suffix('.kernel-confirmed').exists():
     outer.wait_for(lambda:job.active()==1,2,'native stop during blocked service observation')
     result['active_during_uncertainty']=job.active()
     gate.with_suffix('.kernel-confirmed').write_text('owned Job accounting',encoding='ascii')
    if log.stat().st_size>128*1024:raise RuntimeError('synthetic output ceiling')
    if time.monotonic()>end:raise TimeoutError('scoped service did not recover')
    time.sleep(.01)
  result['exit_code']=child.returncode
  text=log.read_text(errors='replace')
  result['proof_markers']=[l for l in text.splitlines() if l.startswith('OPTIC317_SERVICE ')]
  result['cases']=[json.loads(l.removeprefix('OPTIC317_SERVICE_CASE ')) for l in text.splitlines() if l.startswith('OPTIC317_SERVICE_CASE ')]
  if a.matrix:assert len(result['cases'])==8
  if child.returncode or len(result['proof_markers'])!=1:
   raise RuntimeError('service proof failed')
  result['complete']=True
 except BaseException as error:
  result['error_type']=type(error).__name__
  if (root/'log').exists():
   detail=(root/'log').read_text(errors='replace')[-6000:]
   for old,new in [(str(root),'<fixture>'),(str(Path.home()),'<home>'),(a.test_exe,'<test-exe>')]:
    detail=detail.replace(old,new)
   result['failure_diagnostic']=detail
  raise
 finally:
  try:
   result['active_before_cleanup']=job.active()
   job.close();result['outer_job_empty_confirmed']=True
  finally:
   if child:child.wait(timeout=2);child._handle.Close()
   temporary.cleanup()
   result['elapsed_ms']=round((time.monotonic()-start)*1000)
   dest.write_text(json.dumps(result,indent=2),encoding='utf-8')
   print(json.dumps(result))

if __name__=='__main__':main()
