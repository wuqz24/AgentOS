"""Real Chromium UI -> HTTP/SSE -> QEMU integration and screenshot capture."""
import json
from pathlib import Path
import sys
import threading
from http.server import ThreadingHTTPServer
from playwright.sync_api import sync_playwright, expect
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
from host.server import Service, Handler

def main():
    output=ROOT/'artifacts/dashboard';output.mkdir(parents=True,exist_ok=True)
    (output/'results.json').write_text(json.dumps({'passed':False}),encoding='utf-8')
    service=Service(ROOT);server=ThreadingHTTPServer(('127.0.0.1',0),Handler);server.service=service
    thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    errors=[]
    try:
        with sync_playwright() as p:
            browser=p.chromium.launch(headless=True)
            page=browser.new_page(viewport={'width':1440,'height':1080},device_scale_factor=1)
            page.on('pageerror',lambda error:errors.append(str(error)))
            page.goto(f'http://127.0.0.1:{server.server_port}')
            page.select_option('#mode','fixture');page.click('#boot')
            expect(page.locator('#connection')).to_have_text('客体已连接',timeout=40000)
            page.click('#workload')
            page.wait_for_selector('[data-worker]',timeout=40000)
            page.locator('#taskForm button').click()
            expect(page.locator('#conclusion')).to_contain_text('task_final',timeout=90000)
            assert '确定性' in page.locator('#conclusion').inner_text()
            assert '系统为什么变慢了' in page.locator('#taskList').inner_text()
            assert page.locator('.node').count()==4
            assert page.locator('#processTable tr').count()>=3
            page.locator('#fileForm button').click()
            expect(page.locator('#fileResult')).to_contain_text('worker-',timeout=15000)
            page.screenshot(path=str(output/'desktop.png'),full_page=True)
            page.evaluate('window.scrollTo(0, 0)')
            page.screenshot(path=str(output/'viewport.png'),full_page=False)
            page.locator('#contextMeta').locator('..').screenshot(path=str(output/'context.png'))
            page.locator('[data-worker][data-state="waiting"]').click()
            page.wait_for_selector('[data-worker][data-state="busy"]',timeout=15000)
            page.reload();page.wait_for_selector('.node',timeout=15000)
            assert 'task_final' in page.locator('#conclusion').inner_text()
            page.set_viewport_size({'width':390,'height':844})
            page.screenshot(path=str(output/'mobile.png'),full_page=True)
            assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
            page.set_viewport_size({'width':1440,'height':1080})
            page.click('#contextDemo')
            expect(page.locator('#contextMeta')).to_contain_text('淘汰 3',timeout=30000)
            page.locator('#contextMeta').locator('..').screenshot(path=str(output/'context-scenario.png'))
            expect(page.locator('#contextMeta')).to_contain_text('0 / 3',timeout=20000)
            page.click('#eventsDemo')
            expect(page.locator('#trace')).to_contain_text('loop_event',timeout=20000)
            page.click('#shutdown')
            expect(page.locator('#connection')).to_contain_text('未连接',timeout=20000)
            exported=page.request.get(f'http://127.0.0.1:{server.server_port}/api/export').body()
            (output/'export.json').write_bytes(exported)
            page.locator('#replay').set_input_files({'name':'run.json','mimeType':'application/json','buffer':exported})
            expect(page.locator('#connection')).to_have_text('历史回放')
            assert page.locator('#taskList .item').count()>=3
            page.locator('#taskForm button').click()
            expect(page.locator('#notice')).to_contain_text('请刷新页面后操作实时系统')
            assert not errors,errors
            browser.close()
        (output/'results.json').write_text(json.dumps({'passed':True,'checks':['real_qemu','fixture_loop','chinese_final','file_query','worker_pause','refresh_restore','disconnect','mobile_width','context_scenario','heartbeat_ipc_scenario','export_replay','no_js_errors'],'errors':errors},indent=2),encoding='utf-8')
        print('PASS browser UI, real guest data, reload, disconnect and responsive layout')
    finally:
        service.close();server.shutdown();server.server_close()

if __name__=='__main__':main()
