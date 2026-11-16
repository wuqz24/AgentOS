# AgentOS：面向 AI Agent 的操作系统内核支持

**队友首次接手：先看 [README_队友必读.md](README_队友必读.md)。** 其中包含解压目录、Windows/WSL 与 Linux 环境准备、直接使用附带镜像的启动方法、演示顺序和常见错误。完整交接包另附赛题原文与历史汇报资料。

在 rCore / RISC-V 上实现 Agent 进程、结构化工具调用、有界用户态 Context、事件等待与 IPC、文件属性索引，并通过宿主机模型适配器和实时页面运行系统管理员场景。

**当前边界：内核、串口、确定性闭环、实时页面和性能实验已能运行；外部真实模型验收等待 API 地址与模型名称。** 确定性模式只用于回归，不会被标为真实模型。真实模式调用失败会显示错误，不会偷偷切换模式。

## 启动

工作目录是包含本 README 的 AgentOS 目录。已验证环境为 Windows + WSL2 的 `AgentOS-Dev`，Rust 1.99.1，QEMU 8.2.2，Python 3.11。Linux 也可直接运行；日常服务只使用 Python 标准库。

压缩包已包含正式镜像 `artifacts/build/{kernel.bin,fs.img,firmware.bin}`。仅运行演示时可以跳过编译，按队友 README 准备 Python、QEMU 和 WSL 配置即可。仅从源码获取项目、缺失镜像，或修改客体代码后，在 PowerShell 执行（`-Distro` 改成自己电脑上 `wsl -l -v` 显示的名称）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Distro Ubuntu-24.04 -Test
```

启动本地服务：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start.ps1
```

打开 **http://127.0.0.1:8765**。选择“确定性回归（无模型）”，点击“启动系统”，即可操作真实 QEMU、工作负载、Context 和事件场景。不要直接双击 HTML，页面需要 HTTP/SSE 服务。

Linux / WSL 内的对应命令：

```bash
bash scripts/build.sh test
python3 -m host.server
```

构建会把源码同步到 Linux 专用缓存 `~/.cache/agentos/build`，解决 Windows 挂载目录构建效率问题；缓存仅用于此项目。构建输出位于 `artifacts/build`，每次页面启动复制独立磁盘，关闭会终止对应 QEMU。

## 本地模型配置

`scripts/start.ps1` 会在不存在时生成 `host/local.env`；也可以从 `host/local.env.example` 复制。填写：

```dotenv
AGENTOS_BASE_URL=https://你的服务地址/v1
AGENTOS_MODEL=你的模型名称
AGENTOS_API_KEY=仅在本地填写
AGENTOS_MODEL_TIMEOUT=60
AGENTOS_WSL_DISTRO=AgentOS-Dev
```

接口采用 OpenAI 兼容的 Chat Completions 工具调用格式，自动追加 `/chat/completions`。模型需支持 `tools`、`tool_choice=required` 和单个 function call。无密钥的本地兼容服务可留空 API_KEY；其他协议需要对应适配器。

密钥不进入内核镜像、浏览器或日志。配置文件已被 Git 忽略。服务启动后补充配置也可直接提交真实模式任务；切换模型后重新启动系统加载新配置。接口依据见 [OpenAI Function calling 官方文档](https://developers.openai.com/api/docs/guides/function-calling)。

## 已实现能力

| 范围 | 实现 |
| --- | --- |
| 任务一 | PCB 中的身份、配额、状态；只读控制页与用户可读写 Context；fork 隔离、exec 重置、退出回收 |
| 任务二 | 共享版本化 ABI；工具名 + 类型化 KV；10 个工具；结构化 JSON 结果与负错误码 |
| 任务三 | FIFO、直接读取、query、rollback、clear；单 Agent 最多 64 KiB Context 映射 |
| 任务四 | 有界属性目录、AND 索引、摘要关键词、写入失效、稳定 ID、校验日志与双文件检查点 |
| 任务五 | 可修改/停止的心跳、watch/unwatch、真正阻塞的 wait、4 条有界 IPC、取消与实例校验 |
| 任务六 | 客体拥有 Agent Loop；宿主机 API 适配；实时页面与受控 CPU 诊断文件；真实模型实际效果待配置验收 |

## 验证与实验

在 WSL / Linux 运行：

```bash
bash scripts/verify.sh
python3 scripts/run_benchmarks.py --root "$(cat artifacts/build/build-root.txt)" --output artifacts/benchmarks
```

正式交付前运行 `bash scripts/verify-production.sh`，会重新构建关闭故障注入的版本、跑客体回归和三组实验。当前随交付包附带的镜像采用正式构建。

浏览器验证在 Windows Python 运行（仅验证工具需要额外安装）：

```powershell
python -m pip install playwright
python -m playwright install chromium
python scripts/test_dashboard.py
```

也可执行 `python scripts/verify_host.py --browser` 依次验证宿主机、桥接、服务、60秒稳定性和浏览器，并保存统一结果。真实模型配置就绪后执行 `python scripts/accept_model.py`；脚本通过后还需核对结论与实际观察。

重新生成报告图表需要 matplotlib：`python scripts/report_benchmarks.py`。图表来自原始 JSON，不使用预填加速比。

测试覆盖非法指针、只读页、分配失败回滚、映射冲突、100 次创建释放、10000 次 Context 追加、1000 轮交错 IPC、跨域拒绝、文件索引一致性、四次重启恢复、协议故障、取消、迟到结果、新运行隔离和页面刷新/断线。测试专用系统调用 4093–4095 仅存在于 `agent-test` 构建。正式构建使用 `bash scripts/build.sh`，不含这些入口。

## 目录与交付入口

| 路径 | 用途 |
| --- | --- |
| `agent-abi/` | 唯一共享 ABI、边界和错误码 |
| `os/src/agent/` | Context、工具、事件与 IPC |
| `os/src/fs/agent_attrs.rs` | 属性目录、索引、摘要缓存与持久化 |
| `user/src/agent_runtime.rs` | 客体模型请求、工具执行、观察与终止 |
| `user/src/bin/agent_*.rs` | 管理员、桥接、负载、机制场景、测试与实验 |
| `host/`、`dashboard/` | Python 本地服务与真实数据页面 |
| `docs/项目总体方案.md` | 已批准的整体方案 |
| `docs/设计说明.md`、`docs/协议与接口.md` | 当前实现与精确接口 |
| `docs/验收矩阵.md`、`docs/实施进展.md` | 逐项证据与待验收范围 |
| `docs/性能实验报告.md` | 三组实验、原始数据和限制 |
| `docs/演示流程.md` | 现场操作、故障处理与回放 |
| `docs/交付说明.md`、`docs/结题讲稿.md` | 当前交付边界、压缩包与逐页讲稿 |
| `artifacts/` | 原始串口、JSON、页面截图、构建指纹和交付产物 |

旧原型有源码快照和哈希清单，见 `artifacts/baseline`。旧开题材料与历史日志用于溯源，不能作为当前版本验证证据。

## 实现范围

这是单核教学内核上的课程项目。一个 Agent 只支持一个用户线程；文件系统使用平面命名空间；属性目录最多 1024 个文件、每文件 8 项属性。已增强新增路径的容量检查和磁盘分配回滚，但没有把整个 rCore 改造成生产级安全系统，也不提供 easy-fs 全盘掉电事务或硬实时保证。Context 内核元数据受保护，节点内容属于该进程的用户态策略数据。

新的 Linux 环境可以运行 `bash scripts/bootstrap-wsl.sh` 安装工具链；Windows 需要自行具备一个可用 WSL 发行版，可用 `AGENTOS_WSL_DISTRO` 指定。项目不会修复或覆盖其他发行版磁盘。
