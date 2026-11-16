from pathlib import Path
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE, MSO_CONNECTOR
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR

ROOT=Path(__file__).resolve().parents[2]; OUT=ROOT/'generated_ppts_v3'; OUT.mkdir(exist_ok=True)
N=RGBColor(13,48,108); B=RGBColor(36,91,177); M=RGBColor(74,125,194); C=RGBColor(47,150,173); O=RGBColor(225,133,54); G=RGBColor(51,135,104); R=RGBColor(191,72,68); I=RGBColor(37,50,67); MUT=RGBColor(91,107,124); BG=RGBColor(244,247,251); PA=RGBColor(231,239,250); L=RGBColor(159,177,202); W=RGBColor(255,255,255)
def clear(p):
 while len(p.slides):
  rid=p.slides._sldIdLst[0].rId;p.part.drop_rel(rid);del p.slides._sldIdLst[0]
def box(s,x,y,w,h,fill=W,line=L,round=False):
 q=s.shapes.add_shape(MSO_SHAPE.ROUNDED_RECTANGLE if round else MSO_SHAPE.RECTANGLE, Inches(x),Inches(y),Inches(w),Inches(h));q.fill.solid();q.fill.fore_color.rgb=fill;q.line.color.rgb=line;q.line.width=Pt(.8);return q
def tx(s,v,x,y,w,h,z=10,col=I,bold=False,align=PP_ALIGN.LEFT,font='Microsoft YaHei',margin=.04):
 q=s.shapes.add_textbox(Inches(x),Inches(y),Inches(w),Inches(h));f=q.text_frame;f.clear();f.word_wrap=True;f.margin_left=Inches(margin);f.margin_right=Inches(margin);f.margin_top=Inches(margin);f.vertical_anchor=MSO_ANCHOR.TOP;p=f.paragraphs[0];p.text=v;p.alignment=align;p.font.name=font;p.font.size=Pt(z);p.font.bold=bold;p.font.color.rgb=col;return q
def line(s,x1,y1,x2,y2,col=L,w=1):
 q=s.shapes.add_connector(MSO_CONNECTOR.STRAIGHT,Inches(x1),Inches(y1),Inches(x2),Inches(y2));q.line.color.rgb=col;q.line.width=Pt(w);return q
def title(p,sec,name,no,take=''):
 s=p.slides.add_slide(p.slide_layouts[6]);s.background.fill.solid();s.background.fill.fore_color.rgb=BG;box(s,0,0,13.333,.12,N,N);box(s,.42,.34,.10,.34,B,B);tx(s,sec.upper(),.6,.35,3,.18,8.5,N,True);tx(s,'AGENTOS / ADVANCED OPERATING SYSTEMS',9.2,.35,3.5,.18,7.5,M,True,PP_ALIGN.RIGHT);tx(s,name,.6,.7,10,.43,23,N,True);tx(s,f'{no:02d}',12.25,.67,.55,.4,24,B,True,PP_ALIGN.RIGHT);line(s,.6,1.27,12.72,1.27,N,1.2)
 if take: box(s,.6,6.72,12.1,.3,PA,PA);tx(s,take,.8,6.76,11.7,.17,9.2,N,True)
 tx(s,'高级操作系统课程设计  |  RISC-V / rCore / QEMU',.6,7.13,5.8,.14,7.2,MUT);return s
def cover(p,kind,sub):
 s=p.slides.add_slide(p.slide_layouts[6]);s.background.fill.solid();s.background.fill.fore_color.rgb=BG;box(s,0,0,13.33,.15,N,N)
 tx(s,'ADVANCED OPERATING SYSTEMS',.8,.7,5,.25,10,B,True);tx(s,'AgentOS',.78,1.28,7,.72,42,N,True);tx(s,'面向 AI 智能体的操作系统内核支持',.82,2.2,9,.4,22,I,True);tx(s,sub,.84,2.8,9,.3,15,MUT)
 box(s,.8,3.52,11.75,1.38,W,L,True);tx(s,'研究对象',1.08,3.8,1.2,.2,9,B,True);tx(s,'Agent 进程  ·  结构化 Tool Call  ·  Context Path',1.08,4.08,8,.26,16,N,True);tx(s,'把“决策—行动—观察—再决策”纳入操作系统可验证的机制边界',1.08,4.47,10,.2,11.5,MUT)
 for i,c in enumerate([N,B,M,C]): box(s,.8+i*3,5.75-i*.34,2.3,.11,c,c);q=s.shapes.add_shape(MSO_SHAPE.OVAL,Inches(.8+i*3-.06),Inches(5.75-i*.34-.06),Inches(.12),Inches(.12));q.fill.solid();q.fill.fore_color.rgb=c;q.line.color.rgb=c
 tx(s,f'{kind}答辩  |  2026',.82,6.57,4,.22,10,N,True);tx(s,'rCore-Tutorial-v3 / Rust / RISC-V virt / QEMU',7.3,6.57,5.3,.2,9.5,MUT,False,PP_ALIGN.RIGHT);return s
def card(s,t,b,x,y,w,h,c=B,n=''):
 box(s,x,y,w,h,W,L,True);box(s,x,y,w,.08,c,c);box(s,x+.15,y+.18,.42,.27,c,c,True)
 tx(s,n,x+.15,y+.21,.42,.14,7.5,W,True,PP_ALIGN.CENTER);tx(s,t,x+.7,y+.18,w-.85,.25,12,N,True);tx(s,b,x+.18,y+.58,w-.36,h-.68,9.1,MUT)
def grid(s,items,cols=3,y=1.62,h=4.8):
 gap=.18;w=12.08;cw=(w-gap*(cols-1))/cols;rows=(len(items)+cols-1)//cols;ch=(h-gap*(rows-1))/rows;cs=[B,C,O,G,N,R]
 for i,(t,b) in enumerate(items): card(s,t,b,.62+(i%cols)*(cw+gap),y+(i//cols)*(ch+gap),cw,ch,cs[i%6],f'{i+1:02d}')
def flow(s,items,y=1.75,h=1.2):
 x=.6;w=12.15/len(items);cs=[N,B,M,C,O,G]
 for i,(t,b) in enumerate(items):
  q=s.shapes.add_shape(MSO_SHAPE.CHEVRON,Inches(x+i*w),Inches(y),Inches(w+.08),Inches(h));q.fill.solid();q.fill.fore_color.rgb=cs[i%6];q.line.color.rgb=W;tx(s,t,x+i*w+.08,y+.16,w-.08,.22,10.5,W,True,PP_ALIGN.CENTER);tx(s,b,x+i*w+.08,y+.5,w-.1,.35,8,W,False,PP_ALIGN.CENTER)
def arch(s):
 vals=[('Agent App','决策循环 / 消费 Context',B),('user_lib','固定 ABI / syscall 封装',C),('Syscall Layer','指针翻译 / 权限检查',O),('Agent Kernel','PCB / 工具 / 配额 / 映射',N)]
 x=.62
 for i,(a,b,c) in enumerate(vals):
  box(s,x,1.7,2.7 if i<3 else 3.4,1.45,W,L,True);box(s,x,1.7,2.7 if i<3 else 3.4,.09,c,c);tx(s,a,x+.15,2.0,2.4 if i<3 else 3.1,.22,12,N,True,PP_ALIGN.CENTER);tx(s,b,x+.17,2.42,2.35 if i<3 else 3.0,.42,9.2,MUT,False,PP_ALIGN.CENTER)
  if i<3: x+=2.86; tx(s,'→',x-.05,2.2,.25,.25,18,c,True,PP_ALIGN.CENTER)
def addr(s):
 x=.8
 for a,w,c in [('代码段',1.9,PA),('数据/堆',2.1,RGBColor(220,238,232)),('Agent Context\n0x40000000 / 16 pages',3.7,RGBColor(255,238,213)),('用户栈',2.5,RGBColor(233,229,249))]: box(s,x,1.72,w,1.3,c,L,True);tx(s,a,x+.08,2.12,w-.16,.4,10.5,N,True,PP_ALIGN.CENTER);x+=w+.1
 tx(s,'低地址',.8,3.2,1,.15,8,MUT);tx(s,'高地址',11.6,3.2,1,.15,8,MUT,False,PP_ALIGN.RIGHT);box(s,.8,3.65,11.55,.75,W,L,True);tx(s,'PCB：agent_type | heartbeat_ms | quota | loop_state | context_base / len / head',1.05,3.9,11,.2,10,N,True,PP_ALIGN.CENTER)
def terminal(s,x,y,w,h,txt): box(s,x,y,w,h,RGBColor(24,34,48),RGBColor(24,34,48),True);tx(s,'QEMU SERIAL / ACCEPTANCE LOG',x+.2,y+.14,w-.4,.18,8.3,RGBColor(137,190,214),True,font='Consolas');tx(s,txt,x+.2,y+.48,w-.4,h-.58,8.6,RGBColor(211,232,226),False,font='Consolas')

def opening():
 p=Presentation();p.slide_width=Inches(13.333);p.slide_height=Inches(7.5);clear(p);cover(p,'开题','题目拆解、设计策略与实施路线')
 s=title(p,'01 / Problem','为什么 Agent 需要内核级支持？',2,'核心判断：Agent 的关键状态不是一次性输入，而是持续迭代的行动上下文。');grid(s,[('Agent Loop','决策 → Tool Call → 内核执行 → 观察结果 → 再决策；每一轮依赖上一轮上下文。'),('传统系统调用','内核只看到离散参数，无法识别 Agent 身份、调用历史与资源边界。'),('性能矛盾','上下文放用户态访问快，放内核态可控；需要机制与策略分离。'),('安全矛盾','Agent 需要直接读写高频数据，但配额、映射和生命周期必须由内核约束。'),('课题落点','把 Agent 建模为带 PCB 元数据、Context 区和结构化 ABI 的特殊进程。'),('研究问题','内核提供哪些机制，才能同时满足可运行、可验证、可扩展与资源受控？')])
 s=title(p,'02 / Requirement','题目拆解：三项必做任务对应三类内核能力',3,'从“进程”到“交互”再到“历史”，形成一条可验收的内核闭环。');flow(s,[('任务一','Agent 进程 / 地址空间'),('任务二','结构化 Tool Call ABI'),('任务三','Context Path / 配额'),('验收','QEMU 可运行演示')]);grid(s,[('扩展 PCB','agent_type、heartbeat、quota、loop_state、Context 元信息。'),('建立 Context 区','固定地址 0x40000000；创建成功后映射 16 页 R/W/U。'),('统一请求格式','tool_id + 两个整数参数；响应含 status、三个结果字段与时间戳。'),('工具集合','system_status、query_process、query_context。'),('路径策略','quota 限制长度；满时 FIFO 覆盖最早节点；支持 query / rollback / clear。'),('证据链','agent_admin 主路径 + agent_probe 异常路径 + QEMU 串口日志。')],3,3.35,2.75)
 s=title(p,'03 / Strategy','总体策略：机制下沉，策略留在用户态',4,'内核负责边界、权限和资源；Agent 负责决策、组合工具与解释结果。');arch(s);grid(s,[('为什么不用 JSON','教学内核采用定长二进制结构，避免解析器与动态内存，把重点放在 ABI 边界。'),('必须进内核','身份、地址映射、配额、工具分发、错误码和生命周期清理。'),('留在用户态','策略性上下文节点、工具组合顺序、对结果的下一步推理。'),('可验证性','每次调用均能在 syscall、PCB、Context 节点和串口日志中找到证据。')],2,3.55,2.45)
 s=title(p,'04 / Memory','Agent Context 区：高频数据与内核元信息分层',5,'地址空间设计同时保留高频访问效率与内核资源治理能力。');addr(s);grid(s,[('用户态直接访问','ContextNode 存放 seq、tool_id、request、result、timestamp；无需每次 syscall 读取。'),('内核态保存元信息','PCB 保存 base、len、head、quota、evictions，避免用户伪造容量。'),('注册时机','普通进程创建后仍是普通进程；sys_agent_create 成功后才建立映射。'),('生命周期','fork 复制独立 Context；exec 替换地址空间并清空 Agent 状态。')],2,4.72,1.75)
 s=title(p,'05 / ABI','Tool Call ABI：一次 Agent 行动的可验证边界',6,'结构化 ABI 让请求、执行、响应、记账四个阶段具有稳定契约。');flow(s,[('请求','tool_id / arg0 / arg1'),('翻译','copy_from_user / 地址检查'),('分发','按工具 ID 执行'),('响应','status / value0..2'),('记账','追加 Context / FIFO')],1.62);grid(s,[('system_status','返回任务数、进程数、Context 长度，验证内核状态可观察。'),('query_process','输入 PID，返回目标存在性；错误目标返回 -3。'),('query_context','读取 len、evictions、quota，形成“状态—路径”闭环。'),('错误处理','未知工具统一返回 -38；非 Agent 进程拒绝专属调用。'),('版本化余地','固定 ID 便于后续增加注册表或 ABI 版本字段。'),('数据路径','结果可写入 Context 区，减少大结果重复穿越 syscall 边界。')],3,3.2,2.9)
 s=title(p,'06 / Context','Context Path：配额、窗口和 FIFO 淘汰',7,'quota=4 时，8 次写入最终保留最近 4 个节点。');flow(s,[('push','追加节点'),('满载','len == quota'),('evict','覆盖最早'),('query','按时间导出'),('rollback','截断历史'),('clear','清空归零')],1.55,1.1);grid(s,[('节点语义','seq 保证顺序；tool_id 说明来源；request/result 摘要支撑下一轮决策。'),('FIFO 选择','简单、稳定、确定性强，适合课程项目验收。'),('回溯语义','rollback 保留指定前缀，帮助 Agent 重放或收敛。'),('清理语义','clear 同时清除长度与淘汰计数，新任务从干净上下文开始。')],2,3.0,3.0)
 s=title(p,'07 / Plan','实施流程：从内核边界到运行证据',8,'每个设计决策都对应源码位置、用户程序和可观察输出。');flow(s,[('源码分析','题目映射 / 确定边界'),('数据结构','PCB / Node / 地址布局'),('ABI 实现','syscall 4000—4008 / 错误码'),('用户验证','admin / probe'),('QEMU 证据','串口日志 / 复现验收')],1.62);grid(s,[('先固定契约','先定义字段、状态转换和系统调用编号，降低跨模块返工。'),('内核集中实现','agent 模块处理状态、工具和 Context；process 只接生命周期。'),('用户态最小实验','admin 展示全路径；probe 只验证异常路径，便于复现。'),('验收脚本化','verify.sh 负责构建；run-qemu-serial.sh 负责运行；日志作为证据。')],2,3.3,2.7)
 s=title(p,'08 / Risk','风险控制与验收标准',9,'把风险转成可观察的失败信号，而不是停留在设计口号。');grid(s,[('地址安全','映射只在注册成功后发生；fork 独立拷贝；exec 清空旧地址。'),('资源安全','quota 由 PCB 控制，FIFO 替换最早节点，避免无限增长。'),('接口稳定','固定结构体与工具 ID；未知 ID=-38；坏 PID=-3。'),('运行稳定','完整构建、QEMU 启动、admin、probe 均通过；无 panic。'),('证据完整','日志展示 Context 基址、quota、节点窗口、rollback、clear 和错误工具。'),('边界声明','heartbeat 字段已落 PCB；timer 唤醒、文件索引和 IPC 属后续扩展。')])
 s=title(p,'09 / Expected','开题结论：可运行、可解释、可扩展',10,'AgentOS 以最小内核改动实现 Agent 一等对象的三项基础能力。');arch(s);grid(s,[('课程价值','覆盖 PCB、虚拟内存、系统调用、资源配额和用户态测试。'),('工程价值','不依赖外部 LLM；所有演示在 RISC-V QEMU 内闭环完成。'),('研究价值','呈现 AI Agent 对传统 OS 抽象的冲击，用机制/策略分离回应。'),('交付形态','内核源码、用户态程序、设计文档、构建脚本、串口日志和 dashboard。')],2,3.55,2.45);return p

def closing():
 p=Presentation();p.slide_width=Inches(13.333);p.slide_height=Inches(7.5);clear(p);cover(p,'结题','实现过程、运行实例与验收结论')
 s=title(p,'01 / Scope','最终实现范围：三个基础任务形成闭环',2,'Agent 身份、结构化工具调用和 Context Path 均已在 QEMU 上完成验收。');flow(s,[('进程','PCB / 注册'),('内存','Context / 0x40000000'),('交互','Tool Call / 3 工具'),('历史','FIFO / rollback / clear'),('验证','admin + probe / QEMU')]);grid(s,[('任务一完成','Agent 状态与 Context 映射接入 PCB，普通进程和 Agent 进程可共存。'),('任务二完成','syscall 4000—4008 支持结构化请求、工具分发、统一响应和错误处理。'),('任务三完成','ContextNode 环形窗口支持 push、query、rollback、clear 和淘汰计数。'),('验证结果','agent_admin 主路径通过；agent_probe 未知工具返回 -38；无 kernel panic。')],2,3.35,2.65)
 s=title(p,'02 / Code','工程结构：功能按内核边界落位',3,'状态集中在 agent 模块，生命周期在 process 接入，用户态只依赖 ABI。');grid(s,[('os/src/agent/mod.rs','AgentKernelState、工具执行、Context 写入与查询；syscall ID 4000—4008 的主要实现位置。'),('os/src/task/process.rs','PCB 增加 Agent 字段；fork 复制元数据和 Context；exec 清理状态。'),('user/src/syscall.rs','ToolRequest、ToolResponse、ContextNode 以及 syscall 封装。'),('user/src/bin/agent_admin.rs','创建 Agent、调用 3 个工具、制造 8 个节点并验证 FIFO。'),('user/src/bin/agent_probe.rs','构造未知 tool_id，验证内核返回 -38 的错误路径。'),('dashboard/','静态教学模拟器，展示 PCB、地址空间、请求响应和 Context Ring。')])
 s=title(p,'03 / PCB','PCB 状态：注册前、注册时、注册后',4,'只有 sys_agent_create 成功，进程才获得 Agent 语义。');flow(s,[('普通进程','type=0 / 无 Context'),('create','校验参数 / 初始化'),('映射','16 页 / R-W-U'),('Agent','保存 base / len / head')]);grid(s,[('核心字段','agent_type、heartbeat_ms、quota、loop_state 属于内核控制面。'),('Context 元数据','base=0x40000000、len、head、evictions 用于边界检查。'),('生命周期验证','fork 复制独立 Context；exec 替换地址空间后清空状态。'),('异常边界','非 Agent 进程被拒绝；未知工具不进入执行分支。')],2,3.45,2.55)
 s=title(p,'04 / VM','地址空间：Agent Context 的真实映射',5,'运行日志确认 Context 基址为 0x40000000；用户态可直接读取首个节点。');addr(s);grid(s,[('映射范围','固定 16 页作为 Context 区；quota 控制逻辑节点数。'),('权限设计','R/W/U 使 Agent 可高速访问；内核掌握地址、长度和创建时机。'),('读写路径','Tool Call 结果写入 ContextNode；admin 直接读取 mapped first node。'),('边界','exec 后旧 Context 不再被 PCB 引用，避免悬空状态。')],2,4.72,1.75)
 s=title(p,'05 / ABI','系统调用链：从用户态请求到内核响应',6,'一次调用同时留下返回值与 Context 节点，保证动作和历史一致。');flow(s,[('user_lib','构造请求'),('syscall','4000—4008'),('guard','身份 / 指针检查'),('tool','按 ID 执行'),('reply','响应结构体'),('record','Context / 淘汰')],1.55,1.18);grid(s,[('system_status','返回 [1,3,n] 类状态字段，观察 Agent 标志、进程数和 Context 长度。'),('query_process','round=2 tool=2 status=0 证明按 PID 查询路径有效。'),('query_context','返回窗口相关字段，用于验证 quota 与 evictions。'),('错误返回','agent_probe 调用不存在工具，输出 code=-38。')],2,3.15,2.9)
 s=title(p,'06 / Context','Context Path 实例：8 次写入如何变成 4 节点窗口',7,'真实数据：6 次主调用 + 2 次附加查询 = 8 个节点，quota=4，发生 4 次淘汰。');flow(s,[('round 0—2','tool=1 / 3 节点'),('附加查询','tool=2 / 第 4 个'),('round 3—5','窗口持续滚动'),('附加查询','tool=3 / 总写入 8'),('结果','seq=4—7 / 4 节点')],1.52,1.18);grid(s,[('最终窗口','seq=4 tool=1；seq=5 tool=1；seq=6 tool=3；seq=7 tool=1。'),('FIFO 计数','quota=4，写入第 5—8 个节点时覆盖旧节点，evictions=4。'),('rollback','日志显示 retained 2 nodes，路径回到更早窗口。'),('clear','日志显示 len=0、evictions=0，清理语义完整。')],2,3.15,2.9)
 s=title(p,'07 / Runtime','QEMU 串口运行实例：从启动到通过',8,'运行环境与关键输出来自 artifacts/qemu-agent-demo.log，而非静态模拟。');terminal(s,.62,1.58,7.25,4.72,'QEMU serial acceptance run\nEnvironment: WSL Ubuntu 22.04, QEMU 6.2.0\nMachine: RISC-V virt\n\nAgent Context mapped at 0x40000000\nPCB: type=1 quota=4 heartbeat=1000ms\ntool 1: system_status\ntool 2: query_process\ntool 3: query_context\n\ncontext visible nodes=4\ncontext rollback retained 2 nodes\ncontext cleared, len=0, evictions=0\nAgentOS demo passed.');grid(s,[('环境复现','bootstrap-wsl.sh → verify.sh → run-qemu-serial.sh。'),('主路径','admin 创建 Agent、列工具、运行 6 轮调用并制造 FIFO。'),('错误路径','probe 构造未知 tool_id，返回 -38，未触发崩溃。'),('可视化辅助','dashboard 为 ABI 对齐教学模拟，不冒充 QEMU 实时桥接。')],2,1.58,4.72)
 s=title(p,'08 / Demo','用户态验收：agent_admin 与 agent_probe',9,'演示程序覆盖主流程、资源约束、生命周期语义和异常分支。');grid(s,[('create','打印基址、PCB type、quota、heartbeat，并列出 3 个工具。'),('loop','6 轮 system_status；第 3 轮 query_process；第 5 轮 query_context。'),('window','读取当前 4 节点，验证 FIFO；直接映射读取首节点 seq=4。'),('cleanup','rollback 到 2 节点，再 clear，检查 len 与 evictions 归零。'),('error','调用未注册工具，内核返回 -38；主路径和错误路径均通过。'),('判定','串口出现 AgentOS demo passed，基础任务闭环完成。')])
 s=title(p,'09 / Test','测试矩阵：功能、资源、生命周期与异常',10,'测试同时验证内核状态、用户观察值与错误码的一致性。');grid(s,[('功能','create + 3 tools；所有 round status=0。'),('资源','8 次 push；窗口 len=4；保留 seq=4—7。'),('生命周期','fork 独立复制；exec 清空状态，源码边界明确。'),('回溯','rollback(2)；日志 retained 2 nodes。'),('异常','unknown tool；agent_probe code=-38。'),('稳定性','QEMU 运行无 kernel panic，日志可重复。')])
 s=title(p,'10 / Boundary','成果边界：完成什么，尚未声称什么',11,'结题报告明确区分已验收机制与后续扩展。');grid(s,[('已完成 / 基础任务','PCB、Context 映射、Tool Call、3 工具、FIFO、rollback、clear、QEMU 验收。'),('已完成 / 交付','源码、用户态测试、构建脚本、设计说明、演示流程、日志和 dashboard。'),('heartbeat 接口','heartbeat_ms 已保存并可查询；timer 唤醒和阻塞队列尚未实现。'),('文件索引','inode 属性查询、内容摘要索引没有写入本次基础任务结论。'),('多 Agent IPC','当前重点是单 Agent 可验证闭环；跨 Agent 消息属于扩展。'),('下一步','接入 timer、属性索引、管理员巡检和多 Agent 协同。')])
 s=title(p,'11 / Conclusion','结题结论：把 Agent 工作模式落到 OS 机制上',12,'AgentOS 已完成可运行的基础闭环，并为扩展留下清晰接口。');arch(s);grid(s,[('操作系统视角','Agent 成为具有身份、地址空间扩展、资源额度和可观测状态的系统对象。'),('实现视角','用最小 PCB、syscall 和用户库改动完成请求到历史的闭环。'),('验证视角','QEMU 日志证明映射、工具调用、FIFO、回溯清理和错误工具。'),('课程结论','项目把 Agent Loop 与进程、虚拟内存、系统调用和资源管理逐一对应。')],2,3.55,2.45);return p
def project_tree(s):
    # A native-shape project map: every label remains editable in PowerPoint.
    box(s,.62,1.55,2.28,4.65,RGBColor(236,242,251),L,True);tx(s,'AgentOS/',.86,1.82,1.7,.28,15,N,True)
    entries=[('os/','内核主体',B),('  src/agent/','Agent 状态、工具、Context',C),('  src/task/','进程与生命周期',O),('  src/mm/','地址空间与页表',G),('user/','用户态程序',B),('  src/syscall.rs','统一请求与响应格式',C),('  bin/agent_admin.rs','主流程演示',O),('  bin/agent_probe.rs','错误流程演示',R),('dashboard/','可视化教学模拟',G),('scripts/','构建与运行脚本',M),('artifacts/','QEMU 串口证据',N),('docs/','设计与演示说明',M)]
    y=2.25
    for name,desc,c in entries:
        tx(s,name,.87,y,1.15,.18,9.4,c,True,font='Consolas');tx(s,desc,2.05,y,5.0,.18,8.8,MUT);y+=.29
    # Three functional lanes to the right.
    lanes=[('内核层','PCB 扩展\nAgent 状态\n工具执行\nContext 配额',B),('用户态层','请求封装\n演示程序\n直接读取 Context',C),('证据层','QEMU 串口\n运行日志\ndashboard 图示',O)]
    for i,(a,b,c) in enumerate(lanes):
        x=3.35+i*3.05;box(s,x,4.95,2.63,1.25,W,L,True);box(s,x,4.95,2.63,.09,c,c);tx(s,a,x+.15,5.18,2.33,.23,12,N,True,PP_ALIGN.CENTER);tx(s,b,x+.18,5.55,2.27,.45,9,MUT,False,PP_ALIGN.CENTER)
        if i<2: tx(s,'→',x+2.68,5.42,.28,.25,18,c,True,PP_ALIGN.CENTER)
    tx(s,'代码文件说明“在哪里实现”，三条泳道说明“如何协作完成一次 Agent 行动”。',3.35,1.8,9.2,.3,12,N,True)
    flow(s,[('用户态','程序发起请求'),('系统调用','进入内核检查'),('内核模块','执行并记账'),('日志 / 界面','展示可观察结果')],y=2.55,h=.82)

def runtime_sequence(s):
    # A detailed, readable sequence diagram for the final defense.
    xs=[.75,3.25,5.75,8.25,10.75]; labels=['agent_admin','user_lib','syscall','agent module','Context 区']
    for x,lbl in zip(xs,labels):
        box(s,x,1.55,1.65,.55,PA,L,True);tx(s,lbl,x+.08,1.74,1.49,.18,10,N,True,PP_ALIGN.CENTER)
        line(s,x+.82,2.15,x+.82,5.85,L,1)
    steps=[(2.45,0,1,'构造 ToolRequest'),(3.08,1,2,'传入 tool_id / 参数'),(3.71,2,3,'检查身份与指针'),(4.34,3,4,'执行并生成结果'),(4.97,3,4,'写入 ContextNode'),(5.60,2,0,'返回 ToolResponse')]
    for y,a,b,lab in steps:
        x1=xs[a]+.82;x2=xs[b]+.82;line(s,x1,y,x2,y,[B,C,O,G,N,R][a%6],1.7);tx(s,lab,min(x1,x2)+.06,y-.22,abs(x2-x1)-.12,.18,8.8,N,True,PP_ALIGN.CENTER)
    tx(s,'一轮调用完成后，返回值给程序，历史节点留在 Context 区供下一轮直接读取。',.82,6.18,11.6,.22,10.5,N,True,PP_ALIGN.CENTER)

def opening_v4():
    p=Presentation();p.slide_width=Inches(13.333);p.slide_height=Inches(7.5);clear(p);cover(p,'开题','题目解释、需求分析与系统设计计划')
    s=title(p,'01 / Brief','题目到底要求我们做什么？',2,'题目不是要求“做一个聊天机器人”，而是要求把 Agent 的运行需求转化为操作系统中的可运行功能。')
    grid(s,[('题目背景','AI 工具通常会反复执行“提出想法—调用工具—得到结果—继续判断”。题目希望我们观察这种工作方式，并思考操作系统能否直接支持它。'),('总目标','在教学操作系统中增加 Agent-OS 功能，让一个 Agent 进程能够创建、调用内核工具、保存自己的探索过程，并在 QEMU 中运行。'),('交付物','内核代码、用户态测试程序、运行脚本、设计文档和演示场景，最终需要是一套可以启动和操作的系统。'),('课程关联','这个题目把进程、地址空间、系统调用、内存限制、错误处理等高级操作系统知识放到一个综合项目里。'),('本项目定位','AgentOS 选择题目的三个基础任务，先保证完整、清楚、可验收，再说明未做的进阶任务。'),('答辩主线','先解释题目为什么需要新机制，再说明设计，最后用 QEMU 输出证明每项机制确实工作。')],3,1.56,4.8)
    s=title(p,'02 / Background','题目背景：Agent 是怎样一步步完成任务的？',3,'用一个“查找代码”的例子理解题目：每次工具返回的结果，都会影响下一次操作。')
    flow(s,[('想法','我需要找到调度代码'),('工具','调用 grep / read'),('系统','返回文件或匹配结果'),('判断','根据结果决定下一步'),('循环','直到任务完成')],1.56,1.05)
    grid(s,[('与普通程序的不同','普通程序通常按预定顺序执行；Agent 会根据刚刚得到的结果改变下一步，因此需要保存连续的操作过程。'),('题目指出的缺口','传统操作系统把这些操作看作普通系统调用，不知道它们属于同一个 Agent，也不帮它保存查询路径。'),('为什么和操作系统有关','Agent 需要进程身份、内存空间、系统调用和资源限制，这些本来就是操作系统负责的内容。'),('我们要回答的问题','如果把 Agent 看成一种特殊进程，哪些信息应该由内核保存，哪些内容可以让用户态直接访问？')],2,3.0,3.15)
    s=title(p,'03 / Requirement','题目要求拆解：基础任务与进阶任务',4,'本项目围绕基础任务 1—3 完成；进阶任务作为后续方向，不在结题中冒充已实现功能。')
    flow(s,[('任务一','创建 Agent\n设计地址空间'),('任务二','结构化工具\n调用内核'),('任务三','管理上下文\n保存查询路径')],1.56,1.12)
    grid(s,[('基础任务一','扩展进程控制块，创建 Agent 进程，并在用户地址空间中增加 Context 区；普通进程仍然可以正常运行。'),('基础任务二','设计请求和响应格式，至少提供三个内核工具，并能处理工具不存在、参数不正确等错误。'),('基础任务三','记录连续调用的历史；当历史太长时自动淘汰旧内容，并支持查询、回退和清空。'),('进阶任务四','按文件属性和内容查找文件。本项目暂未实现 inode 属性索引。'),('进阶任务五','让 Agent 由心跳或事件唤醒。本项目保存了心跳字段，但未实现真正的定时唤醒队列。'),('进阶任务六','多 Agent 协作或管理员场景。本项目完成单 Agent 管理员演示，跨 Agent 通信留作后续。')],3,3.05,3.15)
    s=title(p,'04 / Analysis','题目分析：三个任务之间为什么要连起来？',5,'三个任务不是三个孤立功能，而是“Agent 身份—内核交互—历史记忆”的连续关系。')
    flow(s,[('先有身份','知道哪个进程是 Agent'),('再能行动','通过工具访问系统状态'),('还要记忆','把每轮结果保存下来'),('受到限制','配额防止无限增长')],1.55,1.05)
    grid(s,[('第一层：进程','没有 Agent 身份，内核就无法判断谁可以调用专用工具，也无法给它分配专用 Context 空间。'),('第二层：行动','有了身份后，Agent 才能用统一格式请求 system_status、query_process、query_context 等工具。'),('第三层：历史','每次请求和结果都进入 Context Path，下一轮可以直接查看之前发生了什么。'),('第四层：边界','历史不是无限的，内核用 quota 和 FIFO 控制长度，保证系统不会被一个 Agent 占满。')],2,2.95,3.25)
    s=title(p,'05 / Plan','系统结构计划图：准备怎样实现？',6,'计划采用“用户态负责策略、内核负责边界”的分层结构，保持代码修改范围清楚。');arch(s)
    grid(s,[('用户态程序','agent_admin 负责完整演示；agent_probe 专门测试未知工具。程序决定调用顺序，但不能绕过内核的身份和配额检查。'),('用户库','把普通函数参数整理为固定布局的请求结构，再通过系统调用进入内核，避免每个程序重复写底层代码。'),('系统调用层','检查请求来自哪个进程，翻译用户地址，判断参数是否可读写，然后把请求交给 Agent 模块。'),('Agent 内核模块','保存 PCB 中的 Agent 信息，执行三个工具，更新 Context 长度、队列位置和淘汰次数。'),('Context 区','放在用户地址空间，保存详细节点；这样 Agent 下一轮能直接读取，不必每次向内核索取。'),('运行与展示','QEMU 提供真实内核运行环境；dashboard 作为辅助教学界面，展示同一套数据关系。')],3,3.52,2.5)
    s=title(p,'06 / Data','数据设计计划：内核保存什么，用户态保存什么？',7,'设计的关键是把“必须保护的信息”和“经常读取的内容”分开。')
    addr(s)
    grid(s,[('内核 PCB 保存','Agent 类型、心跳字段、上下文配额、当前长度、队列位置和淘汰次数。这些字段决定系统是否允许继续写入。'),('用户 Context 保存','每个节点的序号、工具编号、请求摘要、结果摘要和时间戳。这些内容主要供 Agent 下一轮阅读。'),('为什么这样分','内核字段少而重要，便于保护；节点内容放用户态，读取快，也更容易展示和调试。'),('发生冲突怎么办','用户态不能自己修改配额；内核每次写入前检查长度，满了就按 FIFO 覆盖最早节点。')],2,4.55,1.95)
    s=title(p,'07 / Interface','交互设计：一次工具调用应该经过哪些步骤？',8,'把一次调用拆成六步，每一步都可以在代码或日志里找到对应位置。');runtime_sequence(s)
    s=title(p,'08 / Workflow','实施流程与阶段计划',9,'先做最小闭环，再用程序和 QEMU 逐项验证，避免只完成代码却无法演示。')
    flow(s,[('第 1 阶段','读题目\n列验收点'),('第 2 阶段','扩展 PCB\n加入 Context'),('第 3 阶段','实现 syscall\n和 3 个工具'),('第 4 阶段','完成 admin\n制造 FIFO'),('第 5 阶段','QEMU 运行\n保存日志')],1.6,1.18)
    grid(s,[('阶段一产出','把 docs.md 中的必做任务、系统调用、验收标准整理成清单，确定本项目只承诺基础任务。'),('阶段二产出','普通进程和 Agent 进程共存；成功创建后出现 0x40000000 的 Context 映射。'),('阶段三产出','三个工具可以返回结构化结果，未知工具能返回明确错误码。'),('阶段四产出','连续调用超过 quota，能够看到旧节点被替换；rollback 和 clear 有明确结果。'),('阶段五产出','在 WSL 和 QEMU 环境完成构建、启动、输入命令、记录串口输出。'),('最终材料','设计说明、演示流程、代码目录、运行日志和 dashboard 共同支撑答辩。')],3,3.25,2.75)
    s=title(p,'09 / Acceptance','开题阶段的验收标准',10,'每一项设计都提前对应一个能观察到的结果，结题时按同一张清单复核。')
    grid(s,[('进程标准','能够创建 Agent；PCB 中的类型、配额和心跳字段正确；普通进程不受影响。'),('内存标准','Agent 能够直接读写 Context 区；地址和长度在内核侧可查询。'),('工具标准','至少三个工具调用成功，响应包含状态和结果字段。'),('历史标准','连续五轮以上调用仍能维护路径；超过配额时自动淘汰，不发生内存无限增长。'),('错误标准','未知工具、错误进程等情况返回负错误码，不导致内核崩溃。'),('运行标准','完整系统能在 QEMU 中启动，用户程序能现场重复演示。')])
    s=title(p,'10 / Conclusion','开题结论：本项目要解决的核心问题',11,'一句话概括：让 Agent 在操作系统中“有身份、能行动、记得住、受约束”。')
    arch(s);grid(s,[('有身份','Agent 不再只是普通程序，而是由 PCB 标记、可以查询和管理的进程。'),('能行动','通过结构化请求调用内核工具，不再依赖一串难以检查的 Shell 命令。'),('记得住','把每一轮操作写入 Context Path，让下一步能够参考之前的结果。'),('受约束','配额、生命周期和错误处理由内核负责，保证系统稳定。')],2,3.55,2.45)
    return p

def closing_v4():
    p=Presentation();p.slide_width=Inches(13.333);p.slide_height=Inches(7.5);clear(p);cover(p,'结题','实现过程、项目结构、运行实例与结论')
    s=title(p,'01 / Result','最终完成了题目中的哪些内容？',2,'结题只汇报已经写入代码并在 QEMU 中验证的内容，进阶任务明确标注为后续方向。');grid(s,[('已完成一','Agent 进程创建与 PCB 扩展；创建成功后映射用户态 Context 区。'),('已完成二','结构化工具调用；实现 system_status、query_process、query_context 三个工具。'),('已完成三','Context Path 记录、quota 限制、FIFO 淘汰、rollback 和 clear。'),('已完成四','agent_admin 演示主流程；agent_probe 验证未知工具错误。'),('已完成五','WSL 编译和 QEMU 串口运行；保存了完整的 qemu-agent-demo.log。'),('没有声称的内容','真正的心跳唤醒、文件属性索引、多 Agent IPC 尚未实现，结题中不把接口字段当作完整功能。')],3,1.55,4.8)
    s=title(p,'02 / Project','完整项目结构图：代码如何组成一个可运行系统？',3,'项目不是只有内核文件，还包括用户程序、脚本、文档、日志和可视化辅助。');project_tree(s)
    s=title(p,'03 / Build','从源码到 QEMU：一次完整构建流程',4,'构建流程把 Rust 内核、用户程序、文件系统和 QEMU 启动脚本串起来。');flow(s,[('源码','os + user'),('编译','Rust target\nriscv64'),('打包','文件系统\n内核镜像'),('启动','QEMU virt\n串口输出'),('输入','agent_admin\nagent_probe')],1.6,1.15);grid(s,[('环境准备','在 WSL Ubuntu 22.04 中准备 Rust 1.98.1 和 QEMU 6.2.0，脚本统一执行环境检查。'),('构建阶段','内核和用户态程序分别编译，用户程序被放入文件系统镜像，启动后由 shell 运行。'),('运行阶段','QEMU 使用 RISC-V virt 机器启动，串口输出不依赖图形窗口，便于答辩现场复现。'),('验证阶段','先运行 agent_admin 看主流程，再运行 agent_probe 看错误路径，最后保存日志作为证据。')],2,3.15,2.9)
    s=title(p,'04 / Runtime','一次 Tool Call 的完整运行过程',5,'下面的图说明“程序发起请求”之后，数据怎样经过系统调用进入内核并回到用户态。');runtime_sequence(s)
    s=title(p,'05 / PCB','实现一：Agent 进程与 Context 映射',6,'运行日志中的地址和 PCB 数据，证明设计不只是图纸，而是在 QEMU 中真实出现。');addr(s);grid(s,[('创建前','普通程序没有 Agent 标记，也没有 Context 区，不能调用 Agent 专用工具。'),('创建时','sys_agent_create 初始化类型、quota、heartbeat，并建立固定大小的用户映射。'),('运行中','PCB 记录地址、长度和队列位置；用户程序可以直接读出 Context 中的首个节点。'),('替换时','fork 复制独立数据；exec 清理旧 Agent 状态，避免新的地址空间继续使用旧信息。')],2,4.55,1.95)
    s=title(p,'06 / ABI','实现二：三个工具如何返回结果？',7,'每个工具都用同一套请求和响应结构，用户程序不需要为每个工具设计一套调用方式。');grid(s,[('system_status','不需要额外参数，返回当前系统和 Agent 可观察的状态；日志中多轮调用均显示 status=0。'),('query_process','传入 PID，查询目标进程是否存在及其 Agent 信息；演示中第 3 轮调用成功。'),('query_context','查看当前 Context 长度、淘汰次数和配额；演示中返回窗口长度 4。'),('统一响应','响应包含 status 和三个结果字段，成功为 0；错误用负数表达，便于程序判断。'),('工具表','启动时列出工具编号和参数说明，让答辩者能看到内核当前提供了哪些能力。'),('错误分支','tool_id 不存在时不执行任何工具，直接返回 -38，agent_probe 专门验证这一点。')],3,1.55,4.8)
    s=title(p,'07 / FIFO','实现三：Context Path 的真实数据变化',8,'演示程序用 quota=4 制造 8 次写入，观察窗口如何只保留最近 4 个节点。');flow(s,[('前 4 次','seq 0—3\n先填满窗口'),('第 5—8 次','覆盖旧节点\n淘汰 4 次'),('查询','保留 seq 4—7\n按时间展示'),('回退','保留 2 个\n验证历史'),('清空','len=0\nevictions=0')],1.55,1.18);grid(s,[('真实写入次数','6 次 system_status，加上第 3 轮的 query_process 和第 5 轮的 query_context，一共 8 个节点。'),('最终窗口内容','日志列出 seq=4、5、6、7；其中 seq=6 来自 query_context，说明不同工具都能进入同一条路径。'),('为什么是 FIFO','本项目优先选择规则简单、结果容易复现的先进先出策略，适合课程大作业的现场验收。'),('清理结果','rollback 后只剩 2 个节点；clear 后长度和淘汰次数同时归零，证明状态没有残留。')],2,3.12,2.9)
    s=title(p,'08 / Serial','运行实例：逐段解释 QEMU 串口输出',9,'串口日志不仅是结果截图，还能按顺序解释每个功能是否完成。');terminal(s,.62,1.5,7.2,4.85,'QEMU serial acceptance run\nEnvironment: WSL Ubuntu 22.04, QEMU 6.2.0\n\nAgent Context mapped at 0x40000000\nPCB: type=1 quota=4 heartbeat=1000ms\ntool 1: system_status\ntool 2: query_process\ntool 3: query_context\n\nround=0..5 tool=1 status=0\ncontext visible nodes=4\n  node seq=4 tool=1\n  node seq=5 tool=1\n  node seq=6 tool=3\n  node seq=7 tool=1\ncontext rollback retained 2 nodes\ncontext cleared, len=0, evictions=0\nAgentOS demo passed.');grid(s,[('第一段','Context mapped at 0x40000000：说明用户态扩展空间创建成功。'),('第二段','PCB type=1、quota=4、heartbeat=1000ms：说明 Agent 元信息已初始化。'),('第三段','tool 1—3：说明工具注册表可被用户程序读取。'),('第四段','nodes=4 和 seq=4—7：说明 FIFO 淘汰后窗口只保留最近节点。'),('第五段','rollback / clear：说明路径可以回退，也能恢复为空。'),('最后一段','AgentOS demo passed：主流程完成；随后 agent_probe 另行验证 -38。')],2,1.5,4.85)
    s=title(p,'09 / Test','测试结果：题目验收标准逐项对应',10,'把题目中的“应该能够”换成项目里的“实际输出”。');grid(s,[('Agent 创建','要求：PCB 字段正确、Context 可访问；结果：type=1、基址 0x40000000、用户态读到首节点。'),('三个工具','要求：至少 3 个结构化工具；结果：启动时列出 tool 1—3，多轮 status=0。'),('连续调用','要求：5 轮以上；结果：agent_admin 执行 6 轮主调用，并额外加入 2 次查询。'),('超限处理','要求：超过配额自动淘汰；结果：8 次写入、quota=4、保留 4 个、FIFO 淘汰 4 次。'),('回退清空','要求：支持历史操作；结果：rollback retained 2 nodes，clear 后 len=0。'),('错误处理','要求：未知工具有明确错误；结果：agent_probe 返回 code=-38，无内核崩溃。')],3,1.55,4.8)
    s=title(p,'10 / Visual','可视化界面与现场演示方式',11,'图形界面用于讲解数据关系，QEMU 串口用于证明真实内核运行，两者职责不同。');grid(s,[('dashboard 展示什么','左侧模拟 PCB 和进程状态，中间展示 Context 地址和环形队列，右侧展示工具请求、响应与告警。'),('如何操作','点击重置、推进一轮、触发心跳、发送告警，可以观察路径长度、淘汰和状态变化。'),('与 QEMU 的关系','dashboard 是静态教学模拟器，不宣称与 QEMU 实时连接；最终功能判断仍以串口日志为准。'),('答辩演示顺序','先打开 dashboard 解释概念，再运行 QEMU 展示真实输出，最后用 agent_probe 结束异常测试。')],2,1.55,4.8)
    s=title(p,'11 / Boundary','成果边界与后续工作',12,'完成基础任务并不等于完成题目全部进阶方向，项目把边界写清楚，便于继续扩展。');grid(s,[('已完成','Agent PCB、Context 映射、结构化 Tool Call、三个工具、FIFO、回退、清空、QEMU 验收。'),('已保留接口','heartbeat_ms 已写入 PCB，未来可以接 timer、wait 和 wakeup，形成真正的心跳机制。'),('后续方向一','在 easy-fs inode 上加入 type、owner、tag 等属性，支持 Agent 按条件查询文件。'),('后续方向二','加入事件队列和多 Agent 消息，让 Agent 在无事可做时休眠、收到事件时唤醒。'),('后续方向三','把管理员 Agent 扩展为文件巡检、资源告警和自动记录任务。'),('工程原则','后续扩展继续沿用现有请求格式、Context 区和内核配额，避免推翻当前设计。')],3,1.55,4.8)
    s=title(p,'12 / Conclusion','结题结论：一个本科课程项目的完整闭环',13,'项目把题目中的抽象要求，落成了可以编译、启动、操作、观察和解释的系统。');arch(s);grid(s,[('从题目到代码','题目的三项基础任务分别落到 PCB、系统调用、Context 管理三个实现区域。'),('从代码到运行','用户程序发起请求，内核检查并执行，结果写入 Context，QEMU 串口留下证据。'),('从运行到答辩','地址、工具列表、节点序号、淘汰次数、回退和错误码都能逐项解释。'),('最终认识','AI Agent 的新需求最终仍然要落在操作系统熟悉的主题上：进程、地址空间、调用边界和资源管理。')],2,3.55,2.45);return p

def main():
 a=OUT/'AgentOS-开题答辩-详细题目分析-可编辑.pptx';b=OUT/'AgentOS-结题答辩-详细实现说明-可编辑.pptx';opening_v4().save(a);closing_v4().save(b);print(a);print(b)
if __name__=='__main__':main()
