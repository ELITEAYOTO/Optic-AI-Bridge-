"""GitHub-hosted Windows only: independent Job-owned adverse-pipe supervision."""
import argparse
import ctypes
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

class Job:
    def __init__(self):
        self.k=ctypes.WinDLL("kernel32",use_last_error=True)
        self.k.CreateJobObjectW.argtypes=[ctypes.c_void_p,w.LPCWSTR]
        self.k.CreateJobObjectW.restype=w.HANDLE
        self.k.SetInformationJobObject.argtypes=[w.HANDLE,ctypes.c_int,ctypes.c_void_p,w.DWORD]
        self.k.AssignProcessToJobObject.argtypes=[w.HANDLE,w.HANDLE]
        self.k.TerminateJobObject.argtypes=[w.HANDLE,w.UINT]
        self.k.QueryInformationJobObject.argtypes=[w.HANDLE,ctypes.c_int,ctypes.c_void_p,w.DWORD,ctypes.c_void_p]
        self.k.CloseHandle.argtypes=[w.HANDLE]
        class Basic(ctypes.Structure):
            _fields_=[("p",ctypes.c_longlong),("j",ctypes.c_longlong),("flags",w.DWORD),
                      ("min",ctypes.c_size_t),("max",ctypes.c_size_t),("count",w.DWORD),
                      ("affinity",ctypes.c_size_t),("priority",w.DWORD),("scheduling",w.DWORD)]
        class IO(ctypes.Structure):
            _fields_=[(n,ctypes.c_ulonglong) for n in ["r","w","o","rb","wb","ob"]]
        class Extended(ctypes.Structure):
            _fields_=[("basic",Basic),("io",IO),("pm",ctypes.c_size_t),("jm",ctypes.c_size_t),
                      ("pp",ctypes.c_size_t),("pj",ctypes.c_size_t)]
        class Accounting(ctypes.Structure):
            _fields_=[(n,ctypes.c_longlong) for n in ["u","k","pu","pk"]]+[(n,w.DWORD) for n in ["faults","total","active","terminated"]]
        self.accounting=Accounting
        self.handle=self.k.CreateJobObjectW(None,None)
        if not self.handle: raise ctypes.WinError(ctypes.get_last_error())
        limits=Extended();limits.basic.flags=0x2000
        if not self.k.SetInformationJobObject(self.handle,9,ctypes.byref(limits),ctypes.sizeof(limits)):
            self.k.CloseHandle(self.handle)
            raise ctypes.WinError(ctypes.get_last_error())
    def assign(self,p):
        if not self.k.AssignProcessToJobObject(self.handle,int(p._handle)): raise ctypes.WinError(ctypes.get_last_error())
    def active(self):
        value=self.accounting()
        if not self.k.QueryInformationJobObject(self.handle,1,ctypes.byref(value),ctypes.sizeof(value),None):
            raise ctypes.WinError(ctypes.get_last_error())
        return value.active
    def close(self):
        try:
            if not self.k.TerminateJobObject(self.handle,87): raise ctypes.WinError(ctypes.get_last_error())
            wait_for(lambda:self.active()==0,2,"job recovery")
        finally: self.k.CloseHandle(self.handle)

def wait_for(predicate,seconds,label):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        if predicate():return
        time.sleep(0.005)
    raise RuntimeError("supervisor deadline: "+label)

def run_case(pwsh,doctor,mode,baseline,root):
    job=Job(); p=None
    started=time.monotonic()
    try:
        with (root/"worker.log").open("wb") as log:
            p=subprocess.Popen([pwsh,"-NoProfile","-NonInteractive","-File",
                str(Path(__file__).with_name("DoctorPipeWorker.ps1")),
                "-Doctor",str(doctor),"-Python",sys.executable,
                "-Fixture",str(Path(__file__).with_name("doctor_pipe_fixture.py")),
                "-Root",str(root),"-Mode",mode],stdout=log,stderr=log)
            job.assign(p)
            wait_for(lambda:(root/"worker.ready").exists() or p.poll() is not None,8,"worker startup")
            assert p.poll() is None,"worker failed before ownership gate"
            (root/"go").write_text("synthetic",encoding="ascii")
            try:
                p.wait(timeout=3 if baseline else 5)
                blocked=False
            except subprocess.TimeoutExpired:
                blocked=True
            if baseline:
                assert blocked,"baseline unbounded diagnostic was not reproduced"
                assert not (root/"result.json").exists()
                return dict(mode=mode,source="baseline",observed="blocked-under-external-supervisor",
                            supervisor_ms=round((time.monotonic()-started)*1000),cleanup="job-empty-confirmed")
            if blocked or p.returncode:
                detail=(root/"worker.log").read_text(errors="replace")[-5000:]
                detail=detail.replace(str(root),"<fixture>").replace(str(doctor),"<doctor>")
                raise RuntimeError("candidate failed: "+mode+"\n"+detail)
            result=json.loads((root/"result.json").read_text(encoding="utf-8-sig"))
            result.update(source="candidate",cleanup="job-empty-confirmed")
            return result
    finally:
        try:job.close()
        finally:
            if p:
                if p.poll() is None:p.kill()
                p.wait(timeout=2)
                p._handle.Close()

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--baseline",required=True)
    parser.add_argument("--candidate",required=True)
    parser.add_argument("--pwsh",required=True)
    parser.add_argument("--output",required=True)
    args=parser.parse_args()
    assert os.name=="nt","native Windows proof required"
    result=dict(sha=os.environ.get("OPTIC_SOURCE_SHA") or os.environ.get("GITHUB_SHA"),checkout_sha=os.environ.get("GITHUB_SHA"),run=os.environ.get("GITHUB_RUN_ID"),
                attempt=os.environ.get("GITHUB_RUN_ATTEMPT"),complete=False,cases=[],
                claim="doctor error-path pipe lifetime; not natural issue319 root cause")
    output=Path(args.output);output.parent.mkdir(parents=True,exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix="optic-doctor-pipes-") as temporary:
            fixtures=[("eof-live",True),("timeout-descendant",True)]
            fixtures += [(m,False) for m in ["normal","eof-live","eof-descendant",
                        "timeout-descendant","partial-utf8","oversized","capture-live",
                        "eof-init-failure","timeout-init-failure","timeout-phase-flood"]]
            for i,(mode,baseline) in enumerate(fixtures):
                root=Path(temporary)/str(i);root.mkdir()
                result["cases"].append(run_case(args.pwsh,Path(args.baseline if baseline else args.candidate).resolve(),
                                               mode,baseline,root))
                output.write_text(json.dumps(result,indent=2),encoding="utf-8")
        result["complete"]=True
    finally:
        output.write_text(json.dumps(result,indent=2),encoding="utf-8")
        print("OPTIC_DOCTOR_PIPE_PROOF "+json.dumps(result,separators=(",",":")))
if __name__=="__main__":main()
