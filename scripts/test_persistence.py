"""Run after build.sh test, reusing its test filesystem across four QEMU boots."""
import argparse
import json
from pathlib import Path
from qemu_test import Guest

p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
a.output.mkdir(parents=True,exist_ok=True)
(a.output/'results.json').write_text(json.dumps({'passed':False}),encoding='utf-8')
checks=[]
for mode in ['verify','corrupt','recover','verify']:
    guest=Guest(a.root,a.output/f'{len(checks)}-{mode}.log')
    try:
        guest.wait('Rust user shell')
        guest.run('agent_file_tests '+mode,'[PASS] file_persistence '+mode)
        checks.append(mode)
    finally:guest.close()
(a.output/'results.json').write_text(json.dumps({'passed':True,'boots':checks,'scope':'normal restart, checksum tail detection, checkpoint recovery'}),encoding='utf-8')
print('PASS persistence across four boots, corrupt tail and checkpoint recovery')
