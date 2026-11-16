from http.server import ThreadingHTTPServer
import json
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import Request,urlopen
from host.server import Handler, Service

class HttpBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.service=Service();cls.server=ThreadingHTTPServer(('127.0.0.1',0),Handler);cls.server.service=cls.service
        cls.thread=threading.Thread(target=cls.server.serve_forever,daemon=True);cls.thread.start()
        cls.base=f'http://127.0.0.1:{cls.server.server_port}'
    @classmethod
    def tearDownClass(cls):
        cls.service.close();cls.server.shutdown();cls.server.server_close()
    def code(self,path,headers=None,data=None):
        try:
            with urlopen(Request(self.base+path,data=data,headers=headers or {}),timeout=5) as r:return r.status
        except HTTPError as e:return e.code
    def test_host_and_origin(self):
        self.assertEqual(self.code('/api/state',{'Host':'attacker.example'}),403)
        self.assertEqual(self.code('/api/action',{'Origin':'https://attacker.example','Content-Type':'application/json'},b'{}'),403)
    def test_body_limits_and_paths(self):
        self.assertEqual(self.code('/api/action',{'Content-Type':'application/json'},b'x'*9000),413)
        self.assertEqual(self.code('/api/action',{},b'{}'),415)
        self.assertEqual(self.code('/api/action',{'Content-Type':'application/json'},b'null'),400)
        self.assertEqual(self.code('/../host/local.env'),404)
    def test_state_has_no_credentials(self):
        with urlopen(self.base+'/api/state') as r:data=json.loads(r.read())
        self.assertNotIn('api_key',data);self.assertNotIn('base_url',data)
        self.assertFalse(data['connected'])
    def test_event_storage_is_bounded(self):
        for i in range(1600):self.service.emit({'event':'test','value':i})
        self.assertEqual(len(self.service.events),1500)

if __name__=='__main__':unittest.main()
