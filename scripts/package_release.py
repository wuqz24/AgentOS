"""Create a reviewable local handoff, excluding configuration and run disks."""
import hashlib
import argparse
import json
from pathlib import Path
import time
import zipfile

ROOT=Path(__file__).resolve().parents[1]
OUTPUT=ROOT/'artifacts/release'

def digest(file):return hashlib.sha256(file.read_bytes()).hexdigest()

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--output',type=Path,default=OUTPUT)
    parser.add_argument('--name',default='AgentOS-current-model-pending.zip')
    parser.add_argument('--workspace-materials',type=Path,help='Include the original task and presentation history from this workspace')
    args=parser.parse_args()
    if Path(args.name).name!=args.name or not args.name.endswith('.zip'):
        parser.error('--name must be a plain .zip filename')
    output=args.output.resolve()
    output.mkdir(parents=True,exist_ok=True)
    checks={}
    for name in ['verification','production','persistence','bridge','service','dashboard','benchmarks','stability','host-verification']:
        report=json.loads((ROOT/'artifacts'/name/'results.json').read_text(encoding='utf-8'))
        if report.get('passed') is not True:raise RuntimeError(f'Validation not passed: {name}')
        checks[name]={'passed':True,'report':f'artifacts/{name}/results.json'}
    build=json.loads((ROOT/'artifacts/build/manifest.json').read_text(encoding='utf-8'))
    for name,expected in build['source_files'].items():
        if digest(ROOT/name)!=expected:raise RuntimeError(f'Build source differs: {name}')
    for name,expected in build['images'].items():
        if digest(ROOT/'artifacts/build'/name)!=expected:raise RuntimeError(f'Build image differs: {name}')
    production=json.loads((ROOT/'artifacts/production/manifest.json').read_text(encoding='utf-8'))
    if production['images']!=build['images']:raise RuntimeError('Production regression belongs to different images')
    if 'Kernel features: production' not in (ROOT/'artifacts/build/versions.txt').read_text():
        raise RuntimeError('Refusing to package an injection-enabled test kernel')
    native=(ROOT/'artifacts/native/checks.log').read_text(encoding='utf-8')
    if 'Native ABI and filesystem tests passed.' not in native:raise RuntimeError('Native checks are missing')
    ppt=ROOT/'artifacts/delivery/AgentOS-结题汇报-待模型验收.pptx'
    if not ppt.is_file():raise RuntimeError('Final presentation missing')
    files=[]
    for folder in ['agent-abi','os','user','easy-fs','easy-fs-fuse','host','dashboard','docs','scripts','tests','bootloader','figures','.github','.devcontainer']:
        for file in (ROOT/folder).rglob('*'):
            if not file.is_file() or file.is_symlink():continue
            rel=file.relative_to(ROOT)
            if any(x in ('target','__pycache__','node_modules','.venv') for x in rel.parts):continue
            if file.name in ('local.env','qemu.pid','link_app.S','linker.ld','.gdb_history'):continue
            if file.suffix in ('.pyc','.log') or file.name.startswith('last-'):continue
            files.append(file)
    for name in ['README.md','README_队友必读.md','LICENSE','rust-toolchain.toml','.gitignore','.gitattributes','Makefile','Dockerfile','.dockerignore','dev-env-info.md','setenv.sh','ping.py']:
        if (ROOT/name).is_file():files.append(ROOT/name)
    evidence=['baseline','p1','p2-kernel','verification','production','native','persistence','bridge','service','dashboard','benchmarks','stability','host-verification','delivery','build']
    for folder in evidence:
        for file in (ROOT/'artifacts'/folder).rglob('*'):
            if not file.is_file() or file.is_symlink():continue
            if file.name=='qemu.pid' or file.suffix=='.zip':continue
            if file.name=='fs.img' and folder!='build':continue
            if file.suffix not in ('.json','.jsonl','.log','.txt','.png','.svg','.pptx','.bin','.img'):continue
            files.append(file)
    files=sorted(set(files),key=lambda p:p.relative_to(ROOT).as_posix())
    entries={p.relative_to(ROOT).as_posix():p for p in files}
    if args.workspace_materials:
        workspace=args.workspace_materials.resolve()
        extras=[
            (workspace/'AgentOS-汇报材料','materials/开题汇报历史'),
            (workspace/'高级操作系统/高级操作系统/generated_ppts_v3','materials/早期原型汇报')]
        for directory,prefix in extras:
            if not directory.is_dir():raise FileNotFoundError(directory)
            for file in directory.iterdir():
                if file.is_file() and not file.is_symlink() and file.suffix.lower() in ('.pptx','.docx','.pdf','.md'):
                    entries[f'{prefix}/{file.name}']=file
        task=workspace/'高级操作系统/高级操作系统/docs.md'
        if not task.is_file():raise FileNotFoundError(task)
        entries['docs/赛题原文.md']=task
        reference=workspace/'第一组-9-崔付顺.pptx'
        if reference.is_file():entries[f'materials/原始参考课件/{reference.name}']=reference
    manifest={'created':time.time(),'status':'mechanisms_verified_real_model_pending','external_model_tested':False,
              'pending':['B03 provider-specific failure recovery','B04 real dynamic model decisions','E01 diagnostic correctness'],
              'checks':checks,'files':{name:digest(p) for name,p in sorted(entries.items())}}
    archive=output/args.name
    with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as z:
        for name,file in sorted(entries.items()):z.write(file,'AgentOS/'+name)
        z.writestr('AgentOS/DELIVERY-MANIFEST.json',json.dumps(manifest,ensure_ascii=False,indent=2))
    with zipfile.ZipFile(archive) as z:
        assert z.testzip() is None
        for name,expected in manifest['files'].items():
            assert hashlib.sha256(z.read('AgentOS/'+name)).hexdigest()==expected,name
        assert not any(name.endswith('/local.env') or '/target/' in name or '/runs/' in name for name in z.namelist())
    sha=digest(archive)
    (archive.with_suffix('.sha256')).write_text(f'{sha}  {archive.name}\n',encoding='utf-8')
    result={'archive':archive.name,'sha256':sha,'bytes':archive.stat().st_size,'file_count':len(entries)+1,
            'archive_integrity_verified':True,'status':manifest['status'],'checks':checks}
    report=output/'results.json' if args.name=='AgentOS-current-model-pending.zip' else archive.with_suffix('.verification.json')
    report.write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result,indent=2))

if __name__=='__main__':main()
