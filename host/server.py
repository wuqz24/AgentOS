"""Local-only HTTP/SSE service for the real QEMU guest. Python stdlib only."""
import argparse
from collections import deque
from concurrent.futures import ThreadPoolExecutor
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import threading
import time
from urllib.parse import urlsplit, parse_qs

from .config import Config
from .model import ApiModel, FixtureModel, ModelError
from .qemu import QemuSession

ROOT = Path(__file__).resolve().parents[1]
FINISHED = {"task_final", "task_error", "task_cancelled"}


class Service:
    def __init__(self, root=ROOT):
        self.root = Path(root)
        self.condition = threading.Condition(threading.RLock())
        self.action_lock = threading.Lock()
        self.events = deque(maxlen=1500)
        self.seq = 0
        self.session = None
        self.mode = "real"
        self.model = None
        self.specs = []
        self.tasks = {}
        self.workers = {}
        self.snapshot = None
        self.pending_starts = 0
        self.records = None
        self.pool = ThreadPoolExecutor(max_workers=2, thread_name_prefix="agentos-model")
        self.closing = threading.Event()
        self.config = Config.load(self.root)
        self.poller = threading.Thread(target=self._poll, daemon=True)
        self.poller.start()

    def emit(self, event):
        with self.condition:
            self.seq += 1
            event = {**event,"stream_seq":self.seq,"host_time":event.get("host_time", time.time())}
            self.events.append(event)
            if self.records:
                self.records.write(json.dumps(event,ensure_ascii=False) + "\n")
                self.records.flush()
            self.condition.notify_all()
        return event

    def consume(self, event):
        session = self.session
        if not session or event["run_id"] != session.run_id:
            return
        name = event["event"]
        with self.condition:
            if name == "bridge_ready":
                self.specs = event["tools"]
            elif name == "agent_ready":
                if not event.get("scenario"):
                    self.pending_starts = max(0, self.pending_starts - 1)
                self.tasks[event["instance"]] = {"pid":event["pid"],"instance":event["instance"],"status":"running","goal":event["goal"],"context":event["context"]}
                for instance in list(self.tasks):
                    if len(self.tasks) <= 32:
                        break
                    if self.tasks[instance]["status"] in FINISHED:
                        del self.tasks[instance]
            elif name == "model_request":
                task = self.tasks.get(event["instance"])
                if task:
                    task.update(status="model_wait",request_id=event["request_id"],context=event["context"],round=event["round"])
            elif name in ("tool_result", "context_changed", "loop_event"):
                task = self.tasks.get(event["instance"])
                if task:
                    task.update(context=event["context"],observation=event.get("observation"),status="running")
            elif name in FINISHED:
                task = self.tasks.get(event["instance"])
                if task:
                    task.update(status=name,context=event.get("context"),final=event)
                if self.model:
                    self.model.forget(event["run_id"], event["instance"])
            elif name in ("worker_ready", "worker_state"):
                self.workers[event["instance"]] = {"pid":event["pid"],"instance":event["instance"],"state":event["state"]}
            elif name == "worker_stopped":
                self.workers.pop(event["instance"], None)
            elif name == "qemu_disconnected":
                for task in self.tasks.values():
                    if task["status"] not in FINISHED:
                        task.update(status="task_error", final={"event":"task_error","source":"host","error":"QEMU 连接已断开，任务中断。"})
                        if self.model:
                            self.model.forget(event["run_id"],task["instance"])
        self.emit(event)
        if name == "model_request":
            self.pool.submit(self._decide, session, self.model, event)

    def _decide(self, session, model, event):
        if not model:
            return
        try:
            decision, seconds = model.decide(event)
        except ModelError as error:
            decision, seconds = {"request_id":event["request_id"],"action":"error","text":str(error)}, None
        except Exception:
            decision, seconds = {"request_id":event["request_id"],"action":"error","text":"Model adapter failed; inspect local diagnostics"}, None
        with self.condition:
            task = self.tasks.get(event["instance"], {})
            valid = self.session is session and task.get("status") == "model_wait" and task.get("request_id") == event["request_id"]
        if not valid:
            model.forget(event["run_id"], event["instance"])
            self.emit({"event":"late_model_result_discarded","run_id":event["run_id"],"instance":event["instance"]})
            return
        self.emit({"event":"model_decision","run_id":event["run_id"],"instance":event["instance"],"mode":model.mode,"duration_seconds":seconds,"decision":decision})
        try:
            reply = session.command("deliver", target_pid=event["pid"],target_instance=event["instance"],kind=2,payload=decision)
            if reply["status"] != 0:
                self.emit({"event":"delivery_error","run_id":event["run_id"],"status":reply["status"]})
        except (TimeoutError, RuntimeError, OSError) as error:
            self.emit({"event":"delivery_error","run_id":event["run_id"],"error":str(error)})

    def state(self):
        with self.condition:
            return json.loads(json.dumps({"stream_seq":self.seq,"run_id":self.session.run_id if self.session else None,
                "connected":bool(self.session and self.session.ready.is_set() and not self.session.ended.is_set()),
                "mode":self.mode,"model_configured":self.config.configured,"model_name":self.config.model,
                "tools":self.specs,"tasks":list(self.tasks.values()),"workers":list(self.workers.values()),
                "snapshot":self.snapshot,"events":list(self.events)[-100:]}))

    def start(self, mode):
        if mode not in ("real", "fixture"):
            raise ValueError("模式必须是 real 或 fixture")
        self.stop()
        self.config = Config.load(self.root)
        # A guest can start without API configuration; real tasks are gated below.
        self.mode = mode
        with self.condition:
            self.tasks.clear()
            self.workers.clear()
            self.events.clear()
            self.snapshot = None
            self.pending_starts = 0
        output = self.root / "artifacts/runs" / time.strftime("%Y%m%d-%H%M%S")
        output = output.with_name(output.name + f"-{time.time_ns() % 1000000:06d}")
        session = QemuSession(self.root, output, self.consume, self.config.distro)
        self.records = (output / "host-events.jsonl").open("w",encoding="utf-8")
        self.session = session
        try:
            session.start()
            self.model = FixtureModel() if mode == "fixture" else (ApiModel(self.config, self.specs) if self.config.configured else None)
            (output / "run.json").write_text(json.dumps({"run_id":session.run_id,"mode":mode,"model":self.config.model if mode == "real" else "deterministic fixture","created":time.time()},ensure_ascii=False,indent=2),encoding="utf-8")
        except Exception:
            self.stop()
            raise
        self.emit({"event":"session_started","run_id":session.run_id,"mode":mode})
        return self.state()

    def stop(self):
        session, self.session = self.session, None
        self.model = None
        if session:
            session.stop()
            with self.condition:
                for task in self.tasks.values():
                    if task["status"] not in FINISHED:
                        task.update(status="task_error",final={"event":"task_error","source":"host","error":"QEMU 已关闭，本次任务中断。"})
            self.emit({"event":"session_stopped","run_id":session.run_id})
        with self.condition:
            if self.records:
                self.records.close()
                self.records = None

    def action(self, command):
        # Serialize browser mutations; serial responses are handled on another thread.
        with self.action_lock:
            action = command.get("action")
            if action == "boot":
                return self.start(command.get("mode", "real"))
            if action == "shutdown":
                self.stop()
                return self.state()
            session = self.session
            if not session or session.ended.is_set():
                raise ValueError("请先启动 QEMU")
            if action == "task":
                if self.mode == "real" and self.model is None:
                    # Allow filling local.env after the guest has already started.
                    self.config = Config.load(self.root)
                    self.model = ApiModel(self.config, self.specs)
                with self.condition:
                    active = self.pending_starts + sum(t["status"] not in FINISHED for t in self.tasks.values())
                if active >= 2:
                    raise ValueError("同时最多运行两个模型任务")
                goal = command.get("goal", "")
                if not isinstance(goal, str) or not goal.strip() or len(goal.encode()) > 768:
                    raise ValueError("目标不能为空，且不能超过 768 UTF-8 字节")
                rounds = command.get("max_rounds", 8)
                if type(rounds) is not int or not 1 <= rounds <= 16:
                    raise ValueError("工具轮数必须为 1..16")
                with self.condition:
                    self.pending_starts += 1
                try:
                    result = session.command("start",goal=goal,max_rounds=rounds)
                    if result["status"] < 0:
                        with self.condition:
                            self.pending_starts = max(0,self.pending_starts-1)
                    return result
                except Exception:
                    with self.condition:
                        self.pending_starts = max(0,self.pending_starts-1)
                    raise
            if action == "workload":
                return session.command("workload")
            if action == "scenario":
                if command.get("scenario") not in ("context","events"):
                    raise ValueError("未知机制场景")
                return session.command("scenario",scenario=command["scenario"])
            if action in ("cancel", "worker_control"):
                instance = command.get("instance")
                with self.condition:
                    target = (self.tasks if action == "cancel" else self.workers).get(instance)
                    if not target:
                        raise ValueError("目标已退出或不属于当前运行")
                    target = dict(target)
                    if action == "cancel":
                        self.tasks[instance]["status"] = "cancelling"
                kind = 6 if action == "cancel" or command.get("state") == "stop" else 7
                payload = {"busy":command.get("state") == "busy"} if kind == 7 else {}
                return session.command("deliver",target_pid=target["pid"],target_instance=instance,kind=kind,payload=payload)
            if action == "tool":
                # Kernel still validates each parameter. These are explicit UI operations.
                if command.get("name") not in {s["name"] for s in self.specs}:
                    raise ValueError("未知工具")
                return session.command("tool",name=command["name"],params=command.get("params", {}))
            raise ValueError("未知操作")

    def _poll(self):
        while not self.closing.wait(1):
            session = self.session
            if not session or not session.ready.is_set() or session.ended.is_set():
                continue
            try:
                result = session.command("snapshot",timeout=5)
                if self.session is session and result["status"] == 0:
                    with self.condition:
                        self.snapshot = {**result["result"],"time_ms":result["time_ms"],"host_time":result["host_time"]}
                    self.emit({"event":"snapshot","run_id":session.run_id,**self.snapshot})
            except (RuntimeError, TimeoutError, OSError):
                if self.session is session:
                    self.emit({"event":"snapshot_stale","run_id":session.run_id})

    def close(self):
        self.closing.set()
        self.stop()
        self.pool.shutdown(wait=False,cancel_futures=True)


class Handler(BaseHTTPRequestHandler):
    server_version = "AgentOS/1"

    def log_message(self, *args):
        pass

    def handle(self):
        try:
            super().handle()
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            pass

    def local_request(self):
        port = self.server.server_address[1]
        hosts = {f"127.0.0.1:{port}", f"localhost:{port}"}
        origin = self.headers.get("Origin")
        return self.headers.get("Host") in hosts and (not origin or origin in {"http://" + h for h in hosts})

    def send_data(self, status, data, content_type="application/json; charset=utf-8"):
        if not isinstance(data, bytes):
            data = json.dumps(data,ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header("Content-Type",content_type)
        self.send_header("Content-Length",str(len(data)))
        self.send_header("Cache-Control","no-store")
        self.send_header("X-Content-Type-Options","nosniff")
        self.send_header("Content-Security-Policy","default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; object-src 'none'; frame-ancestors 'none'")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if not self.local_request():
            return self.send_data(403,{"error":"Local origin required"})
        path = urlsplit(self.path)
        service = self.server.service
        if path.path == "/api/state":
            return self.send_data(200,service.state())
        if path.path == "/api/export":
            return self.send_data(200,service.state())
        if path.path == "/api/events":
            return self.sse(service,path.query)
        files = {"/":"index.html","/index.html":"index.html","/app.js":"app.js","/styles.css":"styles.css"}
        if path.path in files:
            file = service.root / "dashboard" / files[path.path]
            mime = {".html":"text/html", ".js":"text/javascript", ".css":"text/css"}[file.suffix]
            return self.send_data(200,file.read_bytes(),mime + "; charset=utf-8")
        self.send_data(404,{"error":"Not found"})

    def do_POST(self):
        if not self.local_request():
            return self.send_data(403,{"error":"Local origin required"})
        if self.path != "/api/action":
            return self.send_data(404,{"error":"Not found"})
        if self.headers.get("Content-Type", "").split(";")[0] != "application/json":
            return self.send_data(415,{"error":"JSON required"})
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= 8192:
                return self.send_data(413,{"error":"Body must be 1..8192 bytes"})
            body = json.loads(self.rfile.read(length))
            if not isinstance(body,dict):
                raise ValueError("JSON object required")
            result = self.server.service.action(body)
            self.send_data(200,result)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            pass
        except (ValueError, ModelError, TimeoutError, RuntimeError, OSError) as error:
            self.send_data(400,{"error":str(error)})

    def sse(self, service, query):
        try:
            cursor = int(self.headers.get("Last-Event-ID") or parse_qs(query).get("after", ["0"])[0])
        except ValueError:
            return self.send_data(400,{"error":"Invalid event cursor"})
        self.send_response(200)
        self.send_header("Content-Type","text/event-stream")
        self.send_header("Cache-Control","no-store")
        self.send_header("Connection","close")
        self.end_headers()
        try:
            while not service.closing.is_set():
                with service.condition:
                    if service.events and cursor < service.events[0]["stream_seq"] - 1:
                        events = [{"event":"resync","stream_seq":service.seq}]
                    else:
                        events = [e for e in service.events if e["stream_seq"] > cursor]
                    if not events:
                        service.condition.wait(10)
                if events:
                    for event in events:
                        cursor = event["stream_seq"]
                        self.wfile.write(f"id: {cursor}\ndata: {json.dumps(event,ensure_ascii=False)}\n\n".encode())
                else:
                    self.wfile.write(b": heartbeat\n\n")
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port",type=int,default=8765)
    args = parser.parse_args()
    service = Service()
    server = ThreadingHTTPServer(("127.0.0.1",args.port),Handler)
    server.service = service
    print(f"AgentOS: http://127.0.0.1:{args.port} (local only)",flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        service.close()
        server.server_close()


if __name__ == "__main__":
    main()
