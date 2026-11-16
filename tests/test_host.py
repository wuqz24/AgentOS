import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from host.protocol import FrameDecoder, encode_command
from host.config import Config
from host.model import ApiModel, FixtureModel, ModelError, bounded_decision, schemas

class ProtocolTests(unittest.TestCase):
    def test_utf8_every_boundary_and_boot_logs(self):
        wire=b'boot\n>> '+b'@AGENTOS '+json.dumps({'v':1,'event':'ready','text':'系统'},ensure_ascii=False).encode()+b'\n'
        for split in range(len(wire)+1):
            d=FrameDecoder(); events=d.feed(wire[:split])+d.feed(wire[split:])
            self.assertEqual(events[0]['text'],'系统')
    def test_bad_oversize_then_recovery(self):
        d=FrameDecoder()
        frames=d.feed(b'@AGENTOS invalid\n'+b'x'*20000+b'\n@AGENTOS {"v":1,"event":"ok"}\n')
        self.assertEqual([f['event'] for f in frames],['ok']);self.assertEqual(d.errors,2)
    def test_multiple_and_command_limit(self):
        d=FrameDecoder(); self.assertEqual(len(d.feed(b'@AGENTOS {"v":1,"event":"x"}\n'*3)),3)
        with self.assertRaises(ValueError):encode_command({'text':'x'*8192})
        with self.assertRaises(ValueError):encode_command({'x':float('nan')})

class ConfigTests(unittest.TestCase):
    def test_config_and_local_secret(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'host';p.mkdir();(p/'local.env').write_text('AGENTOS_BASE_URL=http://localhost:9876/v1\nAGENTOS_MODEL=test\nAGENTOS_API_KEY=local-secret',encoding='utf-8')
            c=Config.load(d);self.assertTrue(c.configured);self.assertEqual(c.api_key,'local-secret')
    def test_missing_and_bad_url(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(Config.load(d).configured)
            p=Path(d)/'host';p.mkdir();(p/'local.env').write_text('AGENTOS_BASE_URL=https://user:secret@host/v1',encoding='utf-8')
            with self.assertRaises(ValueError):Config.load(d)

class FakeResponse:
    def __init__(self,value):self.value=value
    def __enter__(self):return self
    def __exit__(self,*args):pass
    def read(self,*args):return json.dumps(self.value).encode()

class ModelTests(unittest.TestCase):
    def event(self,round=0):
        return {'run_id':'r','instance':2,'request_id':10+round,'round':round,'max_rounds':8,'goal':'inspect',
                'context':{'nodes':[{'seq':0}]},'latest':{'source':'kernel','result':{'processes':3}}}
    def model(self):return ApiModel(Config('http://127.0.0.1:9/v1','fixture-provider','test-key'),[{'name':'get_system_status','params':'','description':'Actual guest metrics'}])
    def response(self,name,args):return {'choices':[{'message':{'role':'assistant','content':None,'tool_calls':[{'id':'call_1','type':'function','function':{'name':name,'arguments':json.dumps(args)}}]}}]}
    def test_tool_result_association_and_finish(self):
        m=self.model();bodies=[]
        def request(req,**kwargs):
            bodies.append(json.loads(req.data));self.assertEqual(req.full_url,'http://127.0.0.1:9/v1/chat/completions')
            return FakeResponse(self.response('get_system_status',{}) if len(bodies)==1 else self.response('finish',{'text':'Observed','evidence':[0]}))
        with patch('urllib.request.urlopen',request):
            first,_=m.decide(self.event());second,_=m.decide(self.event(1))
        self.assertEqual(first['action'],'tool');self.assertEqual(second['evidence'],[0])
        tool=[x for x in bodies[1]['messages'] if x['role']=='tool'][0]
        self.assertEqual(tool['tool_call_id'],'call_1');self.assertIn('processes',tool['content'])
        m.forget('r',2);self.assertFalse(m.histories)
    def test_invalid_final_unknown_tool_malformed(self):
        for response in [self.response('finish',{'text':'invented','evidence':[99]}),self.response('bad',{}),{'choices':[]}]:
            with patch('urllib.request.urlopen',return_value=FakeResponse(response)),self.assertRaises(ModelError):self.model().decide(self.event())
    def test_timeout_has_no_fallback(self):
        with patch('urllib.request.urlopen',side_effect=TimeoutError),self.assertRaises(ModelError):self.model().decide(self.event())
    def test_bounded_typed_args(self):
        for param in ['汉'*32,True,2**64]:
            with self.assertRaises(ModelError):bounded_decision({'action':'tool','name':'x','params':{'p':param}})
        self.assertLessEqual(len(bounded_decision({'action':'final','text':'汉'*1000,'evidence':[0]})['text'].encode()),1000)
    def test_fixture_is_explicit(self):
        f=FixtureModel();self.assertEqual(f.mode,'fixture');self.assertEqual(f.decide(self.event())[0]['action'],'tool')
    def test_schema_required_types(self):
        s=schemas([{'name':'q','description':'q','params':'pid:number,path:text?'}])[0]['function']['parameters']
        self.assertEqual(s['required'],['pid']);self.assertEqual(s['properties']['pid']['type'],'integer')

if __name__=='__main__':unittest.main()
