"""Actual QEMU serial/IPC/Agent Loop regression; no model API is used."""
import json
from pathlib import Path
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from host.qemu import QemuSession
from host.protocol import encode_command


def main():
    output = ROOT / "artifacts/bridge"
    output.mkdir(parents=True,exist_ok=True)
    (output / "results.json").write_text(json.dumps({"passed":False,"started":time.time()}),encoding="utf-8")
    # Use the newest guest programs on every run.
    disk = output / "fs.img"
    if disk.exists():
        disk.unlink()
    session = QemuSession(ROOT, output)
    checks = []
    try:
        session.start()
        snapshot = session.command("snapshot")
        assert snapshot["status"] == 0, snapshot
        assert snapshot["result"]["system"], snapshot
        checks.append("real_kernel_snapshot")
        started = session.command("start", goal="Inspect real kernel metrics", max_rounds=4)
        assert started["status"] == 0, started
        request = session.wait_event("model_request")
        target = {"target_pid":request["pid"], "target_instance":request["instance"], "kind":2}
        # A stale decision must not execute a tool or advance the current round.
        reply = session.command("deliver", **target, payload={"request_id":0,"action":"tool","name":"get_system_status","params":{}})
        assert reply["status"] == 0, reply
        session.wait_event("decision_rejected", predicate=lambda e:e.get("reason") == "stale_request_id")
        checks.append("stale_model_response_rejected")
        for name, params in [("get_system_status", {}), ("query_process", {"limit":4})]:
            reply = session.command("deliver", **target, payload={"request_id":request["request_id"],"action":"tool","name":name,"params":params})
            assert reply["status"] == 0, reply
            result = session.wait_event("tool_result", after=request["seq"])
            assert result["observation"]["status"] == 0, result
            assert result["observation"]["source"] == "kernel", result
            request = session.wait_event("model_request", after=result["seq"])
        assert request["context"]["length"] == 2
        checks.append("guest_owned_multi_round_loop_context")
        session.command("deliver", **target, payload={"request_id":request["request_id"],"action":"final","text":"Fixture verified two kernel observations.","evidence":[0,1]})
        final = session.wait_event("task_final")
        assert final["complete"] and final["context"]["length"] == 2
        checks.append("evidence_final_and_finish")
        # Repeating an identical command ID must return the cached result.
        worker = session.command("workload")
        assert worker["status"] == 0, worker
        ready = session.wait_event("worker_ready")
        before = session.event_seq
        with session.write_lock:
            session.proc.stdin.write(encode_command({"v":1,"id":worker["id"],"command":"workload"}))
            session.proc.stdin.flush()
        duplicate = session.wait_event("command_result", after=before)
        assert duplicate["result"]["child_pid"] == worker["result"]["child_pid"]
        checks.append("command_deduplication")
        sent=session.command("tool",name="send_message",params={"target_pid":ready["pid"],"target_instance":ready["instance"],"kind":7,"message":'{"busy":false}'})
        assert sent["result"]["status"]==0,sent
        session.wait_event("worker_state", predicate=lambda e:e.get("state") == "waiting")
        session.command("deliver", target_pid=ready["pid"],target_instance=ready["instance"],kind=6,payload={})
        session.wait_event("worker_stopped")
        checks.append("workload_control_and_cancel")
        for data in (b"{bad json}\n", b"["*9+b"]"*9+b"\n", b"x"*8200+b"\n"):
            before = session.event_seq
            with session.write_lock:
                for offset in range(0, len(data), 128):
                    session.proc.stdin.write(data[offset:offset+128])
                    session.proc.stdin.flush()
                    time.sleep(0.002)
            session.wait_event("protocol_error", after=before)
        assert session.command("snapshot")["status"] == 0
        checks.append("malformed_deep_oversize_recovery")
    finally:
        session.stop()
    (output / "results.json").write_text(json.dumps({"passed":True,"checks":checks,"model":"fixture; no external API","time":time.time()}, indent=2), encoding="utf-8")
    for check in checks:
        print("PASS", check)


if __name__ == "__main__":
    main()
