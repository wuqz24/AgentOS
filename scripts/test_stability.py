"""One minute real-guest run with repeated worker creation/exit and idle samples."""
import argparse
import json
from pathlib import Path
import sys
import time
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from host.qemu import QemuSession

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--seconds',type=int,default=60);a=parser.parse_args()
    output=ROOT/'artifacts/stability';output.mkdir(exist_ok=True)
    (output/'results.json').write_text(json.dumps({'passed':False}),encoding='utf-8')
    disk=output/'fs.img'
    if disk.exists():disk.unlink()
    q=QemuSession(ROOT,output);samples=[]
    try:
        q.start();start=time.monotonic()
        for cycle in range(20):
            before=q.event_seq;assert q.command('workload')['status']==0
            worker=q.wait_event('worker_ready',after=before,timeout=30)
            assert q.command('deliver',target_pid=worker['pid'],target_instance=worker['instance'],kind=6,payload={})['status']==0
            q.wait_event('worker_stopped',after=worker['seq'])
            # snapshot dispatch reaps terminated children before taking the metric.
            system=q.command('snapshot')['result']['system']
            if system['agents']!=1:
                time.sleep(.05);system=q.command('snapshot')['result']['system']
            samples.append({'cycle':cycle,**system})
            assert system['agents']==1 and system['processes']==3,system
        while time.monotonic()-start<a.seconds:
            system=q.command('snapshot')['result']['system'];samples.append({'cycle':'idle',**system});time.sleep(1)
        pages=[s['physical_pages_free'] for s in samples[2:]]
        assert max(pages)==min(pages),pages
        result={'passed':True,'elapsed_seconds':time.monotonic()-start,'worker_cycles':20,
                'physical_pages_after_warmup':pages[0],'samples':samples,'scope':'physical pages and live process/Agent counts; disk logs intentionally retained'}
        (output/'results.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
        print('PASS continuous guest run, 20 worker cycles and constant post-warmup physical pages')
    finally:q.stop()

if __name__=='__main__':main()
