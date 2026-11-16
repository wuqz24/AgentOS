import argparse
import hashlib
import json
from pathlib import Path
import time

p=argparse.ArgumentParser();p.add_argument('--source',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
files={}
for directory in ['agent-abi','os','user','easy-fs','easy-fs-fuse']:
    for file in (a.source/directory).rglob('*'):
        if file.is_file() and 'target' not in file.parts and file.suffix in ('.rs','.toml','.lock','.ld','.S'):
            files[str(file.relative_to(a.source)).replace('\\','/')]=hashlib.sha256(file.read_bytes()).hexdigest()
files['rust-toolchain.toml']=hashlib.sha256((a.source/'rust-toolchain.toml').read_bytes()).hexdigest()
combined=hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest()
images={name:hashlib.sha256((a.output/name).read_bytes()).hexdigest() for name in ['kernel.bin','fs.img','firmware.bin']}
(a.output/'manifest.json').write_text(json.dumps({'built_at':time.time(),'source_sha256':combined,'source_files':files,'images':images},indent=2),encoding='utf-8')
