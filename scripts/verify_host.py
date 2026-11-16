"""Repeat host/guest integration checks and retain a common build fingerprint."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--browser',action='store_true',help='Also run Playwright/Chromium verification')
    args=parser.parse_args()
    output=ROOT/'artifacts/host-verification';output.mkdir(parents=True,exist_ok=True)
    report={'passed':False,'started':time.time(),'external_model_tested':False,'checks':[]}
    for name in ['manifest.json','versions.txt']:
        shutil.copy2(ROOT/'artifacts/build'/name,output/name)
    sources={str(p.relative_to(ROOT)).replace('\\','/'):hashlib.sha256(p.read_bytes()).hexdigest()
             for folder in ['host','dashboard','tests','scripts'] for p in (ROOT/folder).rglob('*')
             if p.is_file() and '__pycache__' not in p.parts and p.suffix in ('.py','.js','.html','.css')}
    (output/'source-sha256.json').write_text(json.dumps(sources,indent=2),encoding='utf-8')
    commands=[('unit',['-m','unittest','discover','-s','tests','-v']),
              ('bridge',['scripts/test_bridge.py']),('service',['scripts/test_service.py']),
              ('stability',['scripts/test_stability.py'])]
    if args.browser:commands.append(('browser',['scripts/test_dashboard.py']))
    try:
        (output/'results.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        for name,command in commands:
            result=subprocess.run([sys.executable,*command],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
            (output/f'{name}.log').write_bytes(result.stdout)
            report['checks'].append({'name':name,'passed':result.returncode==0,'exit_code':result.returncode})
            if result.returncode:
                print(result.stdout.decode('utf-8',errors='replace'))
                raise RuntimeError(f'{name} failed; see {output/name}.log')
            print(f'PASS {name}',flush=True)
        report['passed']=True
    finally:
        report['finished']=time.time()
        (output/'results.json').write_text(json.dumps(report,indent=2),encoding='utf-8')

if __name__=='__main__':main()
