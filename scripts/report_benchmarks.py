"""Generate figures and conclusions directly from retained guest measurements."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

ROOT=Path(__file__).resolve().parents[1]
data=json.loads((ROOT/'artifacts/benchmarks/summary.json').read_text(encoding='utf-8'))
groups=data['groups'];hz=data['clock_hz']
def group(experiment,mode,**keys):return next(g for g in groups if g['experiment']==experiment and g['mode']==mode and all(g[k]==v for k,v in keys.items()))
def median(g,field='ticks'):return g[field]['median']
def us(ticks):return ticks/hz*1e6

out=ROOT/'docs/figures';out.mkdir(exist_ok=True)
plt.rcParams.update({'font.family':'DejaVu Sans','font.size':11,'axes.spines.top':False,'axes.spines.right':False,'axes.labelcolor':'#253b59','text.color':'#253b59','axes.edgecolor':'#c9d4e5','figure.facecolor':'white','axes.facecolor':'white','savefig.facecolor':'white'})
blue,grey='#175ac9','#9bb0cc'
fig,axes=plt.subplots(1,3,figsize=(16,4.5),layout='constrained')
counts=[4,16,64];x=list(range(3));w=.34
for offset,mode,color,label in [(-w/2,'direct',blue,'Mapped read'),(w/2,'syscall',grey,'Syscall query')]:
    vals=[us(median(group('context',mode,nodes=n)))/1000 for n in counts]
    axes[0].bar([v+offset for v in x],vals,w,label=label,color=color)
axes[0].set_xticks(x,counts);axes[0].set_xlabel('Context nodes (320 B each)');axes[0].set_ylabel('Microseconds per read');axes[0].set_title('Context access');axes[0].legend(frameon=False)
cases=['selective','broad','empty','and'];x=list(range(4))
for offset,mode,color in [(-w/2,'index',blue),(w/2,'scan',grey)]:
    axes[1].bar([v+offset for v in x],[us(median(group('index',mode,files=512,case=c))) for c in cases],w,label=mode,color=color)
axes[1].set_xticks(x,['Selective','Broad','Empty','AND'],rotation=15);axes[1].set_ylabel('Microseconds per query');axes[1].set_title('512 benchmark files + 33 test files');axes[1].legend(frameon=False)
axes[2].bar(['Event wait','Tight poll'],[median(group('wait',mode),'runtime_ticks')/hz*1000 for mode in ['wait','poll']],color=[blue,grey]);axes[2].set_ylabel('Agent runtime (ms)');axes[2].set_title('One message after a 50 ms delay')
for ax in axes:ax.grid(axis='y',alpha=.18);ax.set_axisbelow(True)
fig.savefig(out/'benchmarks.png',dpi=180);fig.savefig(out/'benchmarks.svg');plt.close(fig)
lines=['# 性能实验报告','',f'客体时钟频率为 {hz:,} Hz。每组保留 11 个样本，下面使用中位数；P95 按最近秩计算（11 个样本时取最大值）。所有测量在单核 QEMU 的 RISC-V virt 客体中进行，不能当作实体硬件或宿主机性能。','',
'![实验结果](figures/benchmarks.png)','',
'## 一、复现与原始数据','',
'先运行 `bash scripts/build.sh test`，再执行 `python3 scripts/run_benchmarks.py --root "$(cat artifacts/build/build-root.txt)" --output artifacts/benchmarks`。最后在安装 matplotlib 的宿主机执行 `python scripts/report_benchmarks.py`。',
'','每次实验复制构建时的干净磁盘，先运行文件功能测试建立 33 个属性文件，再创建实验数据。不会把上一次 512 文件实验残留当作新的 32 文件数据集。',
'','原始串口：`artifacts/benchmarks/serial.log`；356 条原始记录：`raw.json`；32 组统计：`summary.json`；构建输入与镜像 SHA256：`manifest.json`；工具版本：`versions.txt`。每组先比较结果，再测量；索引与扫描在所有分页上比较文件 ID 集合。',
'','## 二、Context 直接读取与系统调用','',
'两条路径返回相同数量、顺序和内容的 320 字节节点。每个样本重复 1,000 次，交替两种路径的先后顺序；计时区间不输出串口，每次真实消费读取结果，防止优化器消除访问。','',
'| 节点数 | 直接读取 μs/次 | syscall μs/次 | 时间比 syscall/直接 |','| --- | ---: | ---: | ---: |']
for n in counts:
    d=median(group('context','direct',nodes=n));s=median(group('context','syscall',nodes=n));lines.append(f'| {n} | {us(d)/1000:.3f} | {us(s)/1000:.3f} | {s/d:.2f} |')
lines+=['','这里减少的是访问路径与用户缓冲区检查等开销。节点越多，数据复制所占比例上升，倍数随之下降。不能把该倍数直接解释成模型推理或完整调查的加速。',
'','## 三、属性索引与全量扫描','',
'目录共包含“实验文件数 + 33”个已登记文件；程序镜像没有登记属性。索引从最小候选集合开始做 AND 检查，扫描遍历全部目录。两者返回同样字段、每页最多四条；摘要缓存预热，不存在查询结果缓存，也未声称是冷缓存实验。','',
'| 实验文件数 | 查询 | 索引 μs | 扫描 μs | 检查记录 索引/扫描 | 时间比 扫描/索引 |','| --- | --- | ---: | ---: | ---: | ---: |']
for n in [32,128,512]:
    for c in cases:
        a,b=group('index','index',files=n,case=c),group('index','scan',files=n,case=c)
        lines.append(f'| {n} | {c} | {us(median(a)):.1f} | {us(median(b)):.1f} | {median(a,"checked"):g}/{median(b,"checked"):g} | {median(b)/median(a):.2f} |')
g=group('index','index',files=512,case='selective')
lines+=['',f'512 文件实验时，索引对象实际占用约 {median(g,"index_allocated_bytes")/1024:.1f} KiB；索引字符串有效内容 {median(g,"index_payload_bytes")/1024:.1f} KiB。前者在单核临界区内按 buddy allocator 的实际分配增减统计，包含树节点与分配对齐，后者不包含这些开销。属性目录、摘要缓存和磁盘日志另计。',
'','选择性高或无结果的条件减少了候选检查；宽泛条件仍处理大部分文件，收益很小，部分小样本可能更慢。索引还增加内存与更新维护成本。`index_update` 原始记录包含创建文件、写内容、设置三项属性和持久化的整批耗时，不能称为单纯内存索引更新耗时。',
'','## 四、紧密轮询与事件等待','',
'父进程收到子进程就绪通知后 sleep 50 ms，再发送同一条 IPC 消息。轮询基线持续调用 receive，不主动 yield；等待组使用 agent_wait。两组都通过两次内核进程查询取得运行 ticks 与调度次数，因此运行量包括共同的测量开销。','',
'| 方式 | Agent 运行 ms | receive 次数 | wait 次数 | 调度次数 | 发送到收取 μs |','| --- | ---: | ---: | ---: | ---: | ---: |']
for mode in ['wait','poll']:
    g=group('wait',mode)
    lines.append(f'| {mode} | {median(g,"runtime_ticks")/hz*1000:.3f} | {median(g,"receive_calls"):g} | {median(g,"wait_calls"):g} | {median(g,"dispatches"):g} | {us(median(g,"latency_ticks")):.1f} |')
a,b=group('wait','wait'),group('wait','poll')
lines+=['',f'本次样本中等待组的 Agent 运行量比紧密轮询低 {(1-median(a,"runtime_ticks")/median(b,"runtime_ticks"))*100:.1f}%。唤醒涉及调度，延迟可能比已在运行的轮询更高。等待不保证系统整体 CPU 空闲：原有 initproc 和 shell 仍有轮询/yield 行为，报告只比较被测 Agent。',
'','## 五、限制与真实模型待验收','',
'QEMU 客体计时会受到宿主机负载、模拟器和缓存状态影响。样本数量有限，报告不作统计显著性或硬实时保证。内核、IPC 和页面指标均可独立复现；外部模型调用耗时、动态诊断正确性和完整端到端分解，仍需要用户提供可用 API 配置后另外测量。确定性回归耗时不得冒充真实模型耗时。','']
(ROOT/'docs/性能实验报告.md').write_text('\n'.join(lines),encoding='utf-8')
print('Generated benchmark figures and report from raw measurements')
