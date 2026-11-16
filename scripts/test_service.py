"""Service -> fixture -> serial -> guest -> SSE-state integration."""
import json
from pathlib import Path
import sys
import time
import threading
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from host.server import Service
from host.model import FixtureModel

def wait(service, predicate, timeout=60):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        value=service.state()
        if predicate(value):return value
        time.sleep(.05)
    raise TimeoutError('Service integration condition: '+str(service.state()['tasks']))

def main():
    output=ROOT/'artifacts/service';output.mkdir(exist_ok=True)
    (output/'results.json').write_text(json.dumps({'passed':False,'started':time.time()}),encoding='utf-8')
    service=Service(ROOT)
    try:
        state=service.action({'action':'boot','mode':'fixture'})
        assert state['connected'] and len(state['tools'])>=4
        service.action({'action':'workload'})
        wait(service,lambda s:len(s['workers'])==1)
        service.action({'action':'task','goal':'Inspect system evidence','max_rounds':6})
        state=wait(service,lambda s:bool(s['tasks']) and s['tasks'][-1]['status']=='task_final')
        task=state['tasks'][-1]
        assert task['final']['complete'] and task['context']['length']==4
        assert task['final']['evidence']==[0,1,2,3]
        assert state['mode']=='fixture'
        wait(service,lambda s:s['snapshot'] is not None)
        worker=state['workers'][0]
        service.action({'action':'worker_control','instance':worker['instance'],'state':'waiting'})
        wait(service,lambda s:s['workers'][0]['state']=='waiting')
        service.action({'action':'worker_control','instance':worker['instance'],'state':'stop'})
        wait(service,lambda s:not s['workers'])
        for scenario in ['context','events']:
            before=service.seq
            service.action({'action':'scenario','scenario':scenario})
            wait(service,lambda s:any(e['event']=='task_final' and e.get('scenario')==scenario and e['stream_seq']>before for e in s['events']))
            if scenario=='context':
                assert any(e['event']=='context_changed' and e['operation']=='clear' and e['context']['length']==0 for e in service.events)
                assert any(e['event']=='tool_result' and e.get('context',{}).get('evictions')==3 for e in service.events)
            else:
                assert any(e['event']=='loop_event' and e.get('kind')=='ipc_wakeup' for e in service.events)
        released=threading.Event()
        class DelayedFixture(FixtureModel):
            def decide(self,event):
                released.wait(5)
                return super().decide(event)
        service.model=DelayedFixture()
        service.action({'action':'task','goal':'Cancel pending fixture','max_rounds':3})
        pending=wait(service,lambda s:s['tasks'][-1]['status']=='model_wait')['tasks'][-1]
        service.action({'action':'cancel','instance':pending['instance']})
        wait(service,lambda s:s['tasks'][-1]['status']=='task_cancelled')
        released.set()
        wait(service,lambda s:any(e['event']=='late_model_result_discarded' for e in s['events']))
        assert not any(e['event']=='tool_result' and e['instance']==pending['instance'] for e in service.events)
        service.model=FixtureModel()
        service.action({'action':'task','goal':'Recover after cancellation','max_rounds':6})
        wait(service,lambda s:bool(s['tasks']) and s['tasks'][-1]['status']=='task_final')
        old_session=service.session
        old_event=next(e for e in reversed(service.events) if e['event']=='model_request')
        old_run=old_session.run_id
        service.action({'action':'boot','mode':'fixture'})
        assert service.state()['run_id']!=old_run and not service.state()['tasks']
        service._decide(old_session,FixtureModel(),old_event)
        assert not service.state()['tasks']
        assert old_session.proc.poll() is not None
        service.action({'action':'task','goal':'New run has no old task state','max_rounds':6})
        wait(service,lambda s:bool(s['tasks']) and s['tasks'][-1]['status']=='task_final')
        released.clear()
        service.model=DelayedFixture()
        service.action({'action':'task','goal':'Unexpected guest disconnect','max_rounds':3})
        wait(service,lambda s:s['tasks'][-1]['status']=='model_wait')
        service.session.stop()
        wait(service,lambda s:not s['connected'] and s['tasks'][-1]['status']=='task_error')
        released.set()
        output=ROOT/'artifacts/service';output.mkdir(exist_ok=True)
        (output/'export.json').write_text(json.dumps(service.state(),ensure_ascii=False,indent=2),encoding='utf-8')
        (output/'results.json').write_text(json.dumps({'passed':True,'model':'explicit deterministic fixture','checks':['multi_round_context','evidence_sequence','workload_state','snapshot','export','context_scenario','heartbeat_ipc_scenario','cancel_pending','discard_late_decision','recover_after_cancel','restart_isolation','old_qemu_terminated','unexpected_disconnect']}),encoding='utf-8')
        print('PASS service, real guest metrics, scenarios, cancel/late response and recovery')
    finally:service.close()

if __name__=='__main__':main()
