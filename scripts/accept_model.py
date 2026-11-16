"""Explicit external-model acceptance. Never run without local configuration."""
import argparse
import json
from pathlib import Path
import sys
import time
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from host.config import Config
from host.server import Service

def main():
    p=argparse.ArgumentParser();p.add_argument('--check-config',action='store_true');a=p.parse_args()
    config=Config.load(ROOT)
    if not config.configured:
        print('Pending: configure AGENTOS_BASE_URL and AGENTOS_MODEL in host/local.env. No API request sent.')
        return 2
    if a.check_config:
        print('Local endpoint/model are configured. No API request sent.');return 0
    output=ROOT/'artifacts/model-acceptance';output.mkdir(exist_ok=True)
    result={'passed':False,'mode':'real','model':config.model,'started':time.time(),'semantic_review_required':True}
    (output/'results.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    service=Service(ROOT)
    try:
        service.action({'action':'boot','mode':'real'})
        service.action({'action':'workload'})
        deadline=time.monotonic()+60
        while not service.state()['workers']:
            if time.monotonic()>deadline:raise TimeoutError('Worker startup')
            time.sleep(.1)
        goal='系统为什么变慢了？请至少两次采样比较客体进程运行量，检索 type=log,tag=diagnostic 的相关诊断文件，最后引用 Context 序号说明观察、推断和限制。暂不执行处置。'
        service.action({'action':'task','goal':goal,'max_rounds':12})
        deadline=time.monotonic()+13*(config.timeout+5)
        while True:
            state=service.state()
            if state['tasks'] and state['tasks'][-1]['status'] in ('task_final','task_error','task_cancelled'):break
            if time.monotonic()>deadline:raise TimeoutError('Real model acceptance deadline')
            time.sleep(.2)
        task=state['tasks'][-1]
        records=[e for e in service.events if e['event']=='tool_result' and e['instance']==task['instance']]
        names=[e['observation']['name'] for e in records]
        assert task['status']=='task_final' and task['final'].get('complete'),task.get('final')
        assert names.count('query_process')>=2,names
        assert 'query_files' in names or 'read_file_summary' in names,names
        assert task['final'].get('evidence'),task['final']
        result.update(passed=True,run_id=state['run_id'],tools=names,final=task['final'],run_directory=str(service.session.output))
        (output/'export.json').write_text(json.dumps(state,ensure_ascii=False,indent=2),encoding='utf-8')
        print('PASS external API mechanics and evidence references; review final claims against recorded observations.')
    finally:
        service.close()
        (output/'results.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    return 0

if __name__=='__main__':raise SystemExit(main())
