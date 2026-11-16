# AgentOS 队友接手与运行指南

交接日期：2026-10-20。

这是面向AI智能体的操作系统课程项目，基于rCore教学内核，在RISC-V QEMU里运行。工程包括内核、用户态Agent、宿主机服务、实时网页、测试、实验数据和汇报材料。

**现在不需要模型API就能演示。请选择“确定性回归（无模型）”。** 该模式使用固定决策步骤检查链路，但进程、内存、工具执行、Context和文件结果来自真实QEMU。真实模型适配器已实现，真实模型的动态诊断效果尚未验收，不能把固定模式说成AI实际诊断。

## 1. 解压后先确认目录

建议解压到本地磁盘，例如 `D:\AgentOS`。下面的Windows命令都以该路径为例；实际位置不同就替换。先完整解压，不要在压缩软件的预览窗口里运行。使用普通本地路径，避免逗号、网络共享路径和自动同步中的目录。

如果解压后出现两层AgentOS，请进入包含 `README.md`、`host`、`os` 的那一层。

```text
AgentOS/
  README_队友必读.md         ← 先读本文件
  README.md                 ← 工程概览
  agent-abi/                ← 内核与用户态共享的ABI
  os/                       ← rCore内核与Agent机制
  user/                     ← 客体SDK、桥接、管理员、负载与测试程序
  easy-fs/、easy-fs-fuse/    ← 文件系统与镜像生成
  host/、dashboard/         ← Python本地服务与网页
  scripts/、tests/           ← 启动、构建、验证和打包
  docs/                     ← 总体方案、设计、协议、验收、实验与讲稿
  docs/赛题原文.md           ← 从原工作区保留的任务要求
  artifacts/build/          ← 已编译的正式内核、固件和干净磁盘
  artifacts/delivery/       ← 当前实现对应的结题PPT
  artifacts/dashboard/      ← 页面截图与回放JSON
  materials/                ← 完整交接包额外附带的开题和历史汇报资料
  DELIVERY-MANIFEST.json    ← 压缩包逐文件SHA256清单
```

不需要原作者的用户名、桌面目录或Codex才能运行项目。包内不包含作者的WSL虚拟磁盘、Python/Rust/QEMU安装程序、本地API密钥、Git历史或编译缓存；需要在自己的电脑准备运行环境。第三方Rust依赖在首次重新编译时联网获取。

## 2. Windows上只运行演示

### 2.1 准备Python和WSL2

先在PowerShell检查：

```powershell
python --version
wsl -l -v
```

宿主服务只用Python标准库，没有必须执行的 `pip install -r requirements.txt`。已验证Windows Python 3.11、Ubuntu内Python 3.12。安装Python后应能在终端执行 `python`；安装入口见[Python官方Windows下载页](https://www.python.org/downloads/windows/)。

WSL列表中要有能正常启动、VERSION为2的Ubuntu。已有可用发行版就使用现有名称，不需要新建作者电脑上的 `AgentOS-Dev`。

如果还没安装WSL，在**管理员PowerShell**查看可用发行版并安装Ubuntu 24.04；按系统要求重启，首次进入Ubuntu时创建Linux用户名和密码：

```powershell
wsl --list --online
wsl --install -d Ubuntu-24.04
```

如果列表中的名称不同，使用列表给出的准确名称。WSL安装、重启与初次账户设置依据[微软官方安装说明](https://learn.microsoft.com/en-us/windows/wsl/install)。

### 2.2 在WSL里安装QEMU

下面以发行版名 `Ubuntu-24.04` 为例。进入Ubuntu：

```powershell
wsl -d Ubuntu-24.04
```

在打开的**Linux终端**里执行：

```bash
sudo apt-get update
sudo apt-get install -y qemu-system-misc python3
qemu-system-riscv64 --version
python3 --version
exit
```

本项目实际验证的QEMU版本为8.2.2。此时只运行附带镜像，不需要安装Rust。安装依赖后，无模型演示可以离线运行。

### 2.3 配置自己的WSL名称

回到**Windows PowerShell**，进入工程目录：

```powershell
cd D:\AgentOS
if (-not (Test-Path host/local.env)) {
    Copy-Item host/local.env.example host/local.env
}
notepad host/local.env
```

仅把这一行改成 `wsl -l -v` 中自己的发行版名称：

```dotenv
AGENTOS_WSL_DISTRO=Ubuntu-24.04
```

默认模板中写的是作者电脑的 `AgentOS-Dev`，队友电脑通常没有这个名称，必须核对。若自己的名称是 `Ubuntu`，就填 `Ubuntu`。模型的Base URL、Model和API Key三项暂时留空。保存后关闭记事本。

配置会被当前终端中的同名 `AGENTOS_*` 环境变量覆盖；如之前手动设置过，检查是否仍指向旧发行版。

### 2.4 启动页面

仍在**Windows PowerShell的AgentOS目录**：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start.ps1
```

看到 `AgentOS: http://127.0.0.1:8765` 后，保持这个终端打开，在**这台电脑的浏览器**访问：

```text
http://127.0.0.1:8765/
```

这是本机地址，不是作者电脑的远程服务，也不是公网网址。不要加 `www.` 或 `.com`；不要双击 `dashboard/index.html`；网页需要后端HTTP/SSE连接。可以按Ctrl+L选中地址栏，粘贴完整地址，再只按Enter。

日常重开项目只需运行启动命令，首次安装步骤不必重复。更改WSL配置后关闭并重开服务，再在页面启动系统。

## 3. 页面上按这个顺序演示

1. 左上模式选择 **“确定性回归（无模型）”**，点击 **“启动系统”**，等待“客体已连接”。模式只在启动系统时生效。
2. 点击工作负载区的 **“启动负载”**。进程表出现 `agent_worker`，可以看到它的状态和区间运行量。
3. 在管理员区保留默认目标或填写“系统为什么变慢了”，点击 **“开始调查”**。当前固定回归会执行四次实际工具调用，显示调用轨迹、4条Context以及明确标注为测试结论的文本。
4. 在工作负载区点击 **“暂停”**，观察负载转为等待状态；点击 **“恢复”** 再观察运行量变化。运行量是客体调度统计，不是Windows任务管理器的整机CPU百分比。
5. 在文件属性查询中使用 `type=log,tag=diagnostic`，选择索引查询后点击查询，应看到 `worker-...log`。可以切换扫描模式比较结果。
6. 点击 **“Context 边界”**，查看配额为3时的追加、淘汰、回溯和清空。该场景结束时窗口为空是预期结果，过程可在轨迹中查看。
7. 点击 **“心跳与 IPC”**，查看定时唤醒、周期修改和消息唤醒记录。
8. 导出运行记录留作演示证据。查看离线记录时，在页面导入 `artifacts/dashboard/export.json`，页面会明确标识历史回放；刷新页面返回实时模式。

看完先点 **“关闭系统”**，让服务结束本次QEMU，再回终端按 **Ctrl+C** 退出网页服务。重新启动QEMU会使用新的演示磁盘；需要保留结果时先导出记录。

## 4. 原生Linux运行

以下命令用于Ubuntu/Debian。在解压后的AgentOS目录执行：

```bash
sudo apt-get update
sudo apt-get install -y python3 qemu-system-misc
python3 -m host.server
```

然后在同一台机器打开 `http://127.0.0.1:8765/`，按上面的页面流程操作。直接使用附带镜像即可，不需要先编译；Linux直接执行QEMU，不使用WSL发行版配置。

macOS没有在本项目中验证，不能把这些Windows步骤直接当作macOS步骤。

## 5. 修改代码后怎么构建

修改 `dashboard/` 后刷新浏览器；修改 `host/` 后重启Python服务。修改内核、共享ABI、easy-fs或客体用户程序后，必须重新构建内核/文件系统镜像。

首次准备完整开发环境，在**Windows PowerShell的AgentOS目录**执行（发行版名按实际修改）：

```powershell
wsl -d Ubuntu-24.04 --cd "$((Get-Location).Path)" --exec bash scripts/bootstrap-wsl.sh
```

脚本安装Rust 1.99.1、RISC-V目标、cargo-binutils 0.4.0和QEMU等依赖，需要联网，并可能要求输入Linux的sudo密码。它不会替你创建或删除WSL发行版。

构建并跑客体测试：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Distro Ubuntu-24.04 -Test
```

生成正式镜像：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Distro Ubuntu-24.04
```

`-Test`会启用仅用于故障注入的4093–4095测试入口；正式构建关闭这些入口。镜像统一放到 `artifacts/build`，后续网页运行使用这里的文件。构建在Linux的 `~/.cache/agentos/build` 专用缓存中进行，避免Windows挂载盘的编译开销。

原生Linux的对应命令是 `bash scripts/bootstrap-wsl.sh`、`bash scripts/build.sh test` 和 `bash scripts/build.sh`。

## 6. 验证、实验和资料阅读顺序

在**WSL/Linux中进入AgentOS目录**后：

```bash
bash scripts/verify.sh
bash scripts/verify-production.sh
```

前者验证测试内核、ABI、磁盘分配、串口、持久化和服务；后者构建正式内核、跑回归并从干净磁盘生成三组性能数据。会更新 `artifacts/` 下的本地结果。`artifacts/build/build-root.txt` 是生成机器的缓存路径；换电脑后必须先运行构建脚本更新它，不要把作者的 `/root/.cache/...` 当作自己机器已有的目录。

需要浏览器自动化验证时，再在Windows Python安装可选依赖：

```powershell
python -m pip install playwright
python -m playwright install chromium
python scripts/test_dashboard.py
```

这个浏览器检查使用实际QEMU和固定决策，不调用外部模型。测试脚本默认的WSL名称为 `AgentOS-Dev`；在其他发行版上进行完整开发验证，优先使用上述WSL/Linux里的 `verify.sh`。页面和浏览器测试通过 `host/local.env` 读取你配置的发行版名称。

需要从JSON重新生成性能图和报告时：

```powershell
python -m pip install matplotlib
python scripts/report_benchmarks.py
```

| 阅读顺序 | 文档 | 解决的问题 |
| --- | --- | --- |
| 1 | docs/赛题原文.md | 课程要求做什么 |
| 2 | docs/项目总体方案.md | 为什么按这个架构推进，阶段与验收如何对应 |
| 3 | docs/实施进展.md、docs/验收矩阵.md | 当前完成与未完成的部分、每项证据在哪里 |
| 4 | docs/设计说明.md、docs/协议与接口.md | 修改代码前理解机制和边界 |
| 5 | docs/性能实验报告.md | 三组实验的真实结果与限制 |
| 6 | docs/演示流程.md、docs/结题讲稿.md | 怎么演示和汇报 |

当前结题汇报使用 `artifacts/delivery/AgentOS-结题汇报-待模型验收.pptx`。`materials/`中的开题版本、早期结题材料和参考课件保留作历史资料，其内容不一定反映当前实现，不作为当前验收证据。

PPT可以直接打开使用。重新生成PPT的脚本依赖作者机器已有的Codex文档运行时；运行AgentOS、修改内核或浏览器验证不需要该生成环境。

## 7. 模型API后续接入

只在自己的 `host/local.env` 配置：

```dotenv
AGENTOS_BASE_URL=https://你的服务地址/v1
AGENTOS_MODEL=实际模型名称
AGENTOS_API_KEY=在本地填写
AGENTOS_MODEL_TIMEOUT=60
AGENTOS_WSL_DISTRO=自己的WSL发行版名
```

当前适配器使用OpenAI兼容的Chat Completions工具调用格式；其他协议可能需要修改 `host/model.py`。保存后重启服务和系统，选择真实模型模式。

```powershell
python scripts/accept_model.py --check-config
python scripts/accept_model.py
```

第一条只检查本地配置；第二条会实际调用配置的模型服务，可能产生API费用。真实诊断仍需核对多轮工具结果、Context引用和最终结论，不能仅因为接口返回成功就标为全部验收通过。

## 8. 常见问题

| 现象 | 处理 |
| --- | --- |
| `python`找不到或弹出应用商店 | 安装Python并让终端能找到解释器；重新打开终端后检查 `python --version` |
| `ModuleNotFoundError: host` | 当前目录不对；进入包含 `host/` 和README的AgentOS目录 |
| WSL提示没有 `AgentOS-Dev` | 把 `host/local.env` 的发行版名改成自己 `wsl -l -v` 显示的名称，再重启服务 |
| 页面能开，但QEMU握手超时 | 在配置的发行版里检查 `qemu-system-riscv64 --version` 和 `python3 --version`；查看最新 `artifacts/runs/.../serial.log` |
| 缺少kernel.bin/fs.img/firmware.bin | 完整解压交接包，或按第5节重新构建 |
| 浏览器访问 `www.127.0.0.1.com` 失败 | 完整输入 `http://127.0.0.1:8765/`，不要加www或.com |
| 地址正确仍无法连接 | 确认启动终端还开着、有服务启动提示；使用HTTP和正确端口，本机代理应允许直连127.0.0.1 |
| 8765被占用 | `powershell -ExecutionPolicy Bypass -File scripts/start.ps1 -Port 8766`，浏览器改访问8766 |
| 提交调查提示模型未配置 | 关闭系统，选择“确定性回归（无模型）”，重新启动系统再提交 |
| 浏览器显示历史回放，按钮不执行 | 刷新页面回到实时模式 |
| Context场景结束后变成0/3 | 场景最后一步是clear；在调用轨迹查看之前的淘汰与回溯 |
| 重启后诊断文件不见了 | 页面每次启动使用新的演示磁盘；先导出记录。持久化专门测试保留同一磁盘运行四次 |

## 9. 包的完整性与协作约定

压缩包旁的 `.sha256` 是整体校验值；PowerShell可运行 `Get-FileHash .\压缩包文件名.zip -Algorithm SHA256` 核对。包内 `DELIVERY-MANIFEST.json` 给出每个文件的SHA256。

当前已完成工程机制、确定性闭环和相应验证，未完成项是选定真实模型服务后的B03/B04/E01验收。改动时同步更新接口、测试与文档；保留原LICENSE和rCore来源。不要把本地API密钥、运行磁盘或大量编译缓存重新发给别人。
