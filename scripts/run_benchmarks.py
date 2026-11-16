"""Execute measurements in QEMU, retain raw observations and descriptive stats."""
import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import statistics
import shutil
from qemu_test import Guest

p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
a.output.mkdir(parents=True,exist_ok=True)
for name in ['manifest.json','versions.txt']:
    source=Path(__file__).resolve().parents[1]/'artifacts/build'/name
    if source.exists():shutil.copy2(source,a.output/name)
disk=a.output.resolve()/'fs.img'
shutil.copy2(Path(__file__).resolve().parents[1]/'artifacts/build/fs.img',disk)
guest=Guest(a.root,a.output/'serial.log',disk)
passed=False
try:
    guest.wait('Rust user shell')
    guest.run('agent_file_tests','[PASS] agent_file_tests',timeout=180)
    guest.run('agent_bench','[PASS] agent_bench',timeout=900)
    passed=True
finally:
    guest.close()
    (a.output/'results.json').write_text(json.dumps({'passed':passed}),encoding='utf-8')
rows=[]
for line in (a.output/'serial.log').read_text(encoding='utf-8',errors='replace').splitlines():
    if line.startswith('@BENCH '):rows.append(json.loads(line[7:]))
(a.output/'raw.json').write_text(json.dumps(rows,indent=2),encoding='utf-8')
groups=defaultdict(list)
for row in rows:
    if row['experiment'] in ('environment','index_update'):continue
    key=(row['experiment'],row.get('nodes'),row.get('files'),row.get('case'),row['mode'])
    groups[key].append(row)
summary=[]
for key,values in groups.items():
    item=dict(zip(('experiment','nodes','files','case','mode'),key));item['samples']=len(values)
    for field in ('ticks','runtime_ticks','elapsed_ticks','latency_ticks','receive_calls','wait_calls','dispatches','checked','matched','index_allocated_bytes','index_payload_bytes'):
        if field in values[0]:
            data=sorted(v[field] for v in values)
            item[field]={'median':statistics.median(data),'p95':data[math.ceil(len(data)*.95)-1],'min':min(data),'max':max(data)}
    summary.append(item)
(a.output/'summary.json').write_text(json.dumps({'clock_hz':rows[0]['clock_hz'],'p95_method':'nearest rank; 11 samples per group','groups':summary},indent=2),encoding='utf-8')
assert len(groups)==32,(len(groups),list(groups))
assert all(len(v)==11 for v in groups.values())
print(f'PASS three experiments, {len(rows)} raw records, {len(groups)} comparison groups')
