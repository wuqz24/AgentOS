"""OpenAI-compatible Chat Completions adapter; no silent fixture fallback."""
import json
import time
import urllib.error
import urllib.request


SYSTEM = """你是 AgentOS 客体系统管理员。只依据工具返回的真实客体数据调查用户目标。
每轮选择一个工具。进程 runtime_ticks 是累计客体运行时钟，必须至少两次采样才能分析区间运行量；不要声称它是宿主机 CPU 使用率。
必要时查找文件属性和摘要。所有参数遵循工具约束。观察不足继续取证，错误也是观察，不要编造工具执行或指标。
send_message 仅用于用户要求的本次受控工作负载处置，不能擅自取消其他 Agent。
已有足够证据才调用 finish，使用 evidence 引用 Context seq。说明观察、推断和限制。
note 是给用户看的简短行动说明；不要求提供内部思维过程。达到轮数上限时直接给出有限结论。
"""


class ModelError(RuntimeError):
    pass


def schemas(specs):
    result = []
    for spec in specs:
        properties, required = {}, []
        for item in filter(None, spec["params"].split(",")):
            name, kind = item.split(":")
            properties[name] = {"type":"integer" if kind.rstrip("?") == "number" else "string"}
            if kind.rstrip("?") == "text":
                properties[name]["description"] = "At most 95 UTF-8 bytes"
            if not kind.endswith("?"):
                required.append(name)
        result.append({"type":"function","function":{"name":spec["name"],"description":spec["description"],
            "parameters":{"type":"object","properties":properties,"required":required,"additionalProperties":False}}})
    result.append({"type":"function","function":{"name":"finish","description":"Finish with a concise evidence-based answer after at least one observation.",
        "parameters":{"type":"object","properties":{"text":{"type":"string","description":"Chinese answer, at most 900 UTF-8 bytes"},
            "evidence":{"type":"array","items":{"type":"integer"}}},"required":["text","evidence"],"additionalProperties":False}}})
    return result


def trim_bytes(text, count):
    return text.encode("utf-8")[:count].decode("utf-8", errors="ignore")


def bounded_decision(value):
    action = value.get("action")
    if action == "tool":
        if not isinstance(value.get("name"), str) or len(value["name"].encode()) > 31:
            raise ModelError("Model returned an invalid tool name")
        params = value.get("params", {})
        if not isinstance(params, dict) or len(params) > 6:
            raise ModelError("Model returned invalid tool parameters")
        for key, item in params.items():
            if len(key.encode()) > 23 or not isinstance(item, (str, int)) or isinstance(item, bool):
                raise ModelError("Tool parameters must be bounded strings or integers")
            if isinstance(item, str) and (len(item.encode()) > 95 or "\0" in item):
                raise ModelError("Tool parameter exceeds the guest ABI string limit")
            if isinstance(item, int) and not -(1 << 63) <= item < (1 << 63):
                raise ModelError("Tool parameter exceeds the guest ABI integer limit")
        value["note"] = trim_bytes(str(value.get("note", "")), 160)
    elif action in ("final", "error"):
        value["text"] = trim_bytes(str(value.get("text", "")), 1000)
        if action == "final":
            evidence = value.get("evidence", [])
            if not isinstance(evidence, list) or len(evidence) > 16 or any(type(x) is not int or x < 0 for x in evidence):
                raise ModelError("Final evidence must contain Context sequence numbers")
    else:
        raise ModelError("Model returned an unsupported action")
    if len(json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()) > 1536:
        raise ModelError("Model response exceeds the guest message limit")
    return value


class ApiModel:
    mode = "real"

    def __init__(self, config, specs):
        if not config.configured:
            raise ModelError("请先在 host/local.env 配置模型服务地址和模型名称")
        self.config, self.tools = config, schemas(specs)
        self.histories = {}
        self.pending = {}

    def decide(self, event):
        key = (event["run_id"], event["instance"])
        messages = self.histories.setdefault(key, [{"role":"system","content":SYSTEM}])
        previous = self.pending.pop(key, None)
        if previous:
            messages.append({"role":"tool","tool_call_id":previous,
                "content":json.dumps(event.get("latest"), ensure_ascii=False)})
        messages.append({"role":"user","content":json.dumps({k:event.get(k) for k in ("goal","round","max_rounds","context","latest")}, ensure_ascii=False)})
        body = json.dumps({"model":self.config.model,"messages":messages,"tools":self.tools,
                           "tool_choice":"required","parallel_tool_calls":False}, ensure_ascii=False).encode()
        headers = {"Content-Type":"application/json"}
        if self.config.api_key:
            headers["Authorization"] = "Bearer " + self.config.api_key
        request = urllib.request.Request(self.config.base_url + "/chat/completions", body, headers)
        started = time.monotonic()
        try:
            with urllib.request.urlopen(request, timeout=self.config.timeout) as response:
                data = response.read(1_048_577)
            if len(data) > 1_048_576:
                raise ModelError("Model service response exceeds 1 MiB")
            message = json.loads(data)["choices"][0]["message"]
            calls = message.get("tool_calls", [])
            if len(calls) != 1:
                raise ModelError("Model must return exactly one tool call")
            call = calls[0]
            name = call["function"]["name"]
            params = json.loads(call["function"]["arguments"])
            if name not in {t["function"]["name"] for t in self.tools} or not isinstance(params, dict):
                raise ModelError("Model returned an unknown tool or invalid arguments")
            # Keep only standard protocol fields, never proprietary reasoning metadata.
            messages.append({"role":"assistant","content":message.get("content"),"tool_calls":[call]})
            self.pending[key] = call["id"]
            if name == "finish":
                decision = {"action":"final","text":params.get("text", ""),"evidence":params.get("evidence", [])}
                valid = {node["seq"] for node in event["context"]["nodes"]}
                if not decision["evidence"] or not set(decision["evidence"]).issubset(valid):
                    raise ModelError("Final answer must reference available Context evidence")
            else:
                decision = {"action":"tool","name":name,"params":params,"note":f"调用 {name} 获取或更新客体证据"}
            decision["request_id"] = event["request_id"]
            return bounded_decision(decision), time.monotonic() - started
        except urllib.error.HTTPError as error:
            # The provider's body may include request data or credentials. Do not expose it.
            raise ModelError(f"Model service HTTP {error.code}; inspect local configuration") from None
        except (urllib.error.URLError, TimeoutError, OSError):
            raise ModelError("Model service connection failed or timed out") from None
        except (KeyError, IndexError, TypeError, ValueError):
            raise ModelError("Model service returned a malformed tool-call response") from None

    def forget(self, run_id, instance):
        self.histories.pop((run_id, instance), None)
        self.pending.pop((run_id, instance), None)


class FixtureModel:
    """Explicit deterministic integration fixture, never described as AI."""
    mode = "fixture"

    def decide(self, event):
        steps = [("get_system_status", {}), ("query_process", {"type":"agent","limit":6}),
                 ("query_process", {"type":"agent","limit":6}), ("read_context", {})]
        number = event["round"]
        if number < len(steps):
            name, params = steps[number]
            result = {"action":"tool","name":name,"params":params,"note":"确定性回归步骤，不是模型决策"}
        else:
            result = {"action":"final","text":"确定性链路验证完成。已取得客体系统、进程和 Context 数据；此结果用于接口回归，不代表真实模型诊断。",
                      "evidence":[n["seq"] for n in event["context"]["nodes"]]}
        return bounded_decision({**result,"request_id":event["request_id"]}), 0.0

    def forget(self, *args):
        pass
