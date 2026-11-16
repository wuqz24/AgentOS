// Native editable closing deck using the installed Codex artifact runtime.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const runtime=path.join(os.homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies');
process.env.RUNTIME_NODE_MODULES ||= path.join(runtime,'node/node_modules');
const skill=process.env.AGENTOS_PRESENTATIONS_SKILL || path.join(os.homedir(),'.codex/plugins/cache/openai-primary-runtime/presentations/26.1013.11914/skills/presentations');
const {importRuntimeModule}=await import(pathToFileURL(path.join(skill,'container_tools/runtime_helpers.mjs')));
const {Presentation,PresentationFile}=await importRuntimeModule('@oai/artifact-tool');
const {applyPresentationChartFont,finalizePresentation}=await import(pathToFileURL(path.join(skill,'container_tools/artifact_tool_utils.mjs')));
const build=path.join(root,'.presentation-build');
const output=path.join(root,'artifacts/delivery');
await fs.mkdir(build,{recursive:true});await fs.mkdir(output,{recursive:true});
const data=JSON.parse(await fs.readFile(path.join(root,'artifacts/benchmarks/summary.json'),'utf8'));
const hz=data.clock_hz;
const g=(experiment,mode,keys={})=>data.groups.find(x=>x.experiment===experiment&&x.mode===mode&&Object.entries(keys).every(([k,v])=>x[k]===v));
const us=t=>t/hz*1e6;
const FONT='Microsoft YaHei', INK='#142f53', BLUE='#175ac9', MUTED='#60758f', PALE='#edf3fb';
const pres=Presentation.create({slideSize:{width:1280,height:720}});
function text(slide,value,x,y,w,h,size=25,color=INK,bold=false){
 const box=slide.shapes.add({geometry:'textbox',position:{left:x,top:y,width:w,height:h},fill:'none',line:{fill:'none',width:0}});
 box.text=value;box.text.style={typeface:FONT,fontSize:size,bold,color,verticalAlignment:'top',autoFit:'none',insets:0,wrap:'square'};return box;
}
function slide(title,source){const s=pres.slides.add();s.background.fill='#ffffff';text(s,title,68,44,1144,70,44,INK,true);text(s,`${source}   |   ${String(pres.slides.items.length).padStart(2,'0')}`,68,676,1144,24,15,MUTED);return s;}
function notes(s,body){s.speakerNotes.text=body;}
function table(s,rows,widths,top=153,height=440,font=24){
 const t=s.tables.add({rows:rows.length,columns:rows[0].length,left:68,top,width:1144,height,values:rows,columnWidths:widths});
 t.borders.assign({fill:'#dce5ef',width:1,style:'solid'});
 for(let r=0;r<rows.length;r++)for(let c=0;c<rows[0].length;c++){const cell=t.getCell(r,c);cell.fill=r===0?INK:(r%2? '#ffffff':PALE);cell.text.style={typeface:FONT,fontSize:font,color:r===0?'#ffffff':INK,bold:r===0,verticalAlignment:'middle',insets:12,wrap:'square',autoFit:'none'};}
 return t;
}
function lines(s,items,{x=68,y=164,w=1144,gap=112,size=27}={}){items.forEach((item,i)=>text(s,item,x,y+i*gap,w,gap-18,size));}
function chart(s,categories,series,{title,left=90,top=165,width=1100,height=430,format='0.0'}={}){
 // Chart workbook values use a declared display precision; raw ticks remain in JSON.
 series=series.map(row=>({...row,values:row.values.map(value=>Number(value.toFixed(4)))}));
 const c=s.charts.add('bar',{position:{left,top,width,height},title,titleTextStyle:{fontSize:24,typeface:FONT},categories,series,
  hasLegend:series.length>1,legend:{position:'bottom',textStyle:{fontSize:20,typeface:FONT}},
  barOptions:{direction:'column',grouping:'clustered',gapWidth:100},
  chartFill:'#ffffff',plotAreaFill:'#ffffff',
  xAxis:{textStyle:{fontSize:22,typeface:FONT}},yAxis:{numberFormatCode:format,textStyle:{fontSize:19,typeface:FONT},majorGridlines:{fill:'#dce5ef',width:1}},
  dataLabels:{showValue:true,position:'outEnd',numberFormatCode:format,textStyle:{fontSize:18,typeface:FONT}}});
 applyPresentationChartFont(c,{fontFamily:FONT});return c;
}

// 1. Minimal cover; the pending model acceptance is a material qualification.
{
 const s=pres.slides.add();s.background.fill='#102e54';
 text(s,'AgentOS',82,160,1116,100,78,'#ffffff',true);
 text(s,'面向 AI 智能体的操作系统内核支持',85,290,1100,70,40,'#ffffff');
 text(s,'高级操作系统课程设计 · 实现与验证',85,395,1100,45,28,'#bed3f2');
 text(s,'内核、文件查询、事件闭环与实时页面已验证\n真实模型效果：等待本地 API 配置后验收',85,535,1100,90,25,'#dbe6f7');
 notes(s,'本报告展示当前实际代码和运行证据。真实模型API尚未配置，不能把确定性回归说成真实模型诊断。来源：docs/实施进展.md、docs/验收矩阵.md。');
}
// 2. Requirement mapping.
{
 const s=slide('赛题对应关系','依据：总体方案与验收矩阵');
 table(s,[['赛题任务','实际实现','验收状态'],['一 · Agent 进程','PCB、资源配额、映射与生命周期','机制已验证'],['二 · Tool Call','名称 + 类型化 KV，10 个内核工具','机制已验证'],['三 · Context Path','用户态读取、FIFO、回溯与清空','机制已验证'],['四 · 文件扩展','AND 索引、摘要、缓存与日志恢复','机制已验证'],['五 · Agent Loop','心跳、阻塞、IPC、取消和实例校验','机制已验证'],['六 · 管理员应用','客体执行 + API 适配 + 实时可视化','真实模型待验收']],[205,695,244],150,476,23);
 notes(s,'前三项是基础机制；四、五落实了多种进阶能力；六的确定性闭环与真实内核链路已通过，但动态模型诊断不能仅由接口存在推断。');
}
// 3. Responsibility boundary.
{
 const s=slide('模型策略与内核机制的分工','实现：host / user / os');
 table(s,[['位置','负责内容','原因'],['宿主机','API、会话、串口、HTTP/SSE、日志','复用网络与模型运行环境'],['客体用户态','等待决策、调用工具、记录观察、继续或终止','Agent Loop 真正在客体执行'],['内核','身份、映射、校验、调度、IPC 与文件索引','约束资源并提供实际系统数据'],['浏览器','显示采样、轨迹、Context 和结论','仅显示事件，不虚构系统状态']],[180,630,334],160,378,24);
 text(s,'每一轮：模型选择 → 客体执行 → 内核返回 → Context 记录 → 下一轮',68,585,1144,48,27,BLUE,true);
 notes(s,'输入由桥接进程独占。完整事件通过统一 emit 函数在一次 write 中输出。等待模型时 agent_wait 阻塞，不用循环轮询网络。');
}
// 4. Memory and lifecycle.
{
 const s=slide('Agent 的地址空间与生命周期','实现：os/src/agent/mod.rs · mm · PCB');
 table(s,[['区域','用户权限','边界'],['控制页 0x40000000','只读','内核维护长度、head、配额、代数'],['节点页 0x40001000 起','可读写','每节点 320 B；按页取整'],['总 Context 映射','最多 64 KiB','含控制页，最多 192 个节点']],[440,220,484],160,292,25);
 lines(s,['fork：独立节点页、新实例，清空异步登记','exec / finish / exit：旧状态与资源回收'],{y:500,gap:68,size:27});
 notes(s,'实际测试包括只读页写故障、分配失败逐点回滚、已有映射冲突、fork/exec 和 100 次创建释放。Context 是进程私有策略数据，并非不可篡改审计日志。');
}
// 5. ABI.
{
 const s=slide('结构化工具协议','唯一 ABI：agent-abi/src/lib.rs');
 text(s,'请求：工具名 + 类型化参数',68,160,1144,46,30,BLUE,true);
 text(s,'query_process\n{ "pid": 4, "limit": 1 }',68,224,680,100,30);
 text(s,'结果：实际进程、运行 ticks、调度次数、状态与实例',68,357,1144,50,28,BLUE,true);
 text(s,'版本、指针、页权限、重复键、参数类型和容量均由内核检查。\n未知工具、过期实例、越权与队列满返回明确错误。',68,443,1144,120,28);
 notes(s,'参数最多六组，结果 JSON 最多2048字节。用户缓冲区全范围检查后才执行工具，避免无效输出地址导致部分副作用。工具列表含4个系统/通信工具和6个文件工具。');
}
// 6. Context evidence.
{
 const s=slide('Context 的有界历史','实测：agent_scenarios · Context 边界场景');
 lines(s,['配额 3，连续调用 6 次\n累计淘汰 3 个节点','回溯保留 1 个节点，再追加\n序号继续递增，不复用旧编号','clear 清空有效窗口\n完整轨迹仍保存在运行日志'],{x:68,y:170,w:570,gap:145,size:28});
 const candidate=path.join(root,'artifacts/dashboard/context-scenario.png');
 try{await fs.access(candidate);s.images.add({blob:new Uint8Array(await fs.readFile(candidate)),contentType:'image/png',fit:'contain',alt:'QEMU Context 场景的实际页面截图',position:{left:700,top:150,width:510,height:490}});}catch{const fallback=path.join(root,'artifacts/dashboard/context.png');s.images.add({blob:new Uint8Array(await fs.readFile(fallback)),contentType:'image/png',fit:'contain',alt:'实际Context页面，固定回归模式',position:{left:700,top:150,width:510,height:490}});}
 notes(s,'图为真实浏览器读取QEMU事件生成的Context视图。节点摘要是有界文本，不保证截断后仍是完整JSON。直接读取与query的一致性由客体测试验证。');
}
// 7. Events.
{
 const s=slide('真正阻塞的 Agent Loop','证据：agent_loop_tests · events 场景');
 table(s,[['机制','实现语义','验证'],['agent_wait','检查事件与进入 Blocked 原子完成','无事件时运行量不增长'],['心跳','截止时间存 PCB，可修改和停止','60→20 ms 场景与内核回归'],['IPC','四条队列；域与实例共同校验','1000轮交错通信、满队列拒绝'],['取消与退出','唤醒等待者，清理消息和映射','迟到模型结果丢弃、任务可恢复']],[220,540,384],156,410,25);
 text(s,'等待节省运行量；唤醒需要调度，并非零延迟。',68,601,1144,42,28,BLUE,true);
 notes(s,'定时检查位于调度器边界，避免中断中分配事件对象；不提供硬实时保证。跨独立Agent通信域发送会返回EPERM，PID复用实例不符返回ESTALE。');
}
// 8. Files.
{
 const s=slide('文件属性查询与持久化','实现：os/src/fs/agent_attrs.rs');
 table(s,[['能力','实现','资源或一致性边界'],['描述式查找','type=log,owner=4 等 AND 条件','从最小候选集合开始筛选'],['摘要与缓存','文件前缀摘取 + 关键词匹配','写入使旧摘要失效，版本递增'],['稳定身份','文件目录 ID 与名称关联','最多1024文件，每文件8项属性'],['持久化','双保留文件：快照 + 校验增量日志','损坏尾被识别，后续切换检查点']],[230,500,414],156,410,24);
 text(s,'四次真实重启验证恢复；不声称 easy-fs 具备全盘掉电事务。',68,601,1144,45,26,BLUE,true);
 notes(s,'普通应用不能直接open保留元数据文件。磁盘耗尽时预留块回滚，不越界分配；单元测试保留原文件内容并验证后续分配仍可用。');
}
// 9. Dashboard screenshot.
{
 const s=slide('实时系统实验台','截图来源：Chromium + QEMU，明确标为确定性回归');
 const imagePath=path.join(root,'artifacts/dashboard/viewport.png');
 s.images.add({blob:new Uint8Array(await fs.readFile(imagePath)),contentType:'image/png',fit:'contain',alt:'AgentOS 实时页面，确定性测试模式',position:{left:68,top:140,width:760,height:490}});
 lines(s,['实际内核采样\n显示进程、阻塞和运行量','工具结果与 Context\n可以展开原始记录','刷新恢复与断线标记\n旧运行不混入新任务'],{x:870,y:176,w:340,gap:145,size:26});
 notes(s,'该截图不证明真实模型能力。浏览器测试覆盖工作负载暂停、文件查询、中文目标、最终结论、刷新恢复和移动端宽度。现场演示可独立运行Context与事件场景。');
}
// 10. Context chart.
{
 const s=slide('Context 访问性能','11个样本/组 · 每样本1000次读取 · 单核QEMU');
 const counts=[4,16,64];
 chart(s,counts.map(n=>`${n} 节点`),[
  {name:'直接映射读取',values:counts.map(n=>us(g('context','direct',{nodes:n}).ticks.median)/1000),fill:BLUE},
  {name:'系统调用查询',values:counts.map(n=>us(g('context','syscall',{nodes:n}).ticks.median)/1000),fill:'#9bb0cc'}],{title:'单次读取中位数（μs）'});
 text(s,'节点越多，复制成本占比越大；该结果不能直接推导模型推理加速。',68,615,1144,42,25,MUTED);
 notes(s,'原始数据：artifacts/benchmarks/raw.json；统计：summary.json。两条路径比较相同有效节点和顺序，真实消费结果，批次内不输出串口。P95使用最近秩，见性能报告。');
}
// 11. Index chart.
{
 const s=slide('索引收益取决于查询选择性','512个实验文件 + 33个功能测试文件 · 摘要缓存预热');
 const cases=['selective','broad','empty','and'];
 chart(s,['高选择性','宽泛条件','无结果','AND条件'],[
  {name:'索引',values:cases.map(c=>us(g('index','index',{files:512,case:c}).ticks.median)),fill:BLUE},
  {name:'扫描',values:cases.map(c=>us(g('index','scan',{files:512,case:c}).ticks.median)),fill:'#9bb0cc'}],{title:'内核查询中位数（μs）',format:'0'});
 const bytes=g('index','index',{files:512,case:'selective'}).index_allocated_bytes.median;
 text(s,`宽泛条件接近扫描。索引实际占用约 ${(bytes/1024).toFixed(1)} KiB，更新和持久化另有开销。`,68,615,1144,42,25,MUTED);
 notes(s,'所有分页返回ID集合一致后才测量。内核elapsed_ticks覆盖查询内部；不包含模型/串口。索引内存按buddy实际分配增减计量，含树节点与对齐，不含属性目录、摘要与文件缓存。');
}
// 12. Wait chart plus evidence table.
{
 const s=slide('事件等待与紧密轮询','同一触发：父进程延迟50 ms后发送消息');
 chart(s,['事件等待','紧密轮询'],[{name:'Agent运行量',values:['wait','poll'].map(m=>g('wait',m).runtime_ticks.median/hz*1000),fill:BLUE}],{title:'Agent 运行时间中位数（ms）',left:70,width:650,top:164,height:402});
 const a=g('wait','wait'),b=g('wait','poll');
 text(s,`receive 次数中位数\n等待 ${a.receive_calls.median} 次 / 轮询 ${b.receive_calls.median} 次`,790,185,420,100,27);
 text(s,`发送至收取延迟\n等待 ${us(a.latency_ticks.median).toFixed(1)} μs\n轮询 ${us(b.latency_ticks.median).toFixed(1)} μs`,790,345,420,150,27);
 text(s,'节省的是被测 Agent 的运行量；唤醒调度仍有成本，系统其他进程可能在运行。',68,610,1144,50,25,MUTED);
 notes(s,'11个样本，模式顺序交替。两组都包括共同的进程查询测量开销。轮询没有主动yield；等待调用agent_wait。系统initproc/shell有原有轮询行为，因此不能据此声称整台QEMU CPU空闲。');
}
// 13. Acceptance.
{
 const s=slide('验收证据与未完成项','原始日志与逐项追踪：docs/验收矩阵.md');
 table(s,[['范围','实际证据','结论'],['内存 / Context','故障注入、100次创建、10000次追加','已验证'],['事件 / IPC','1000轮交错、权限、阻塞、取消','已验证'],['文件机制','索引=扫描、摘要失效、四次重启','已验证'],['宿主机 / 页面','协议/HTTP测试、真实浏览器与QEMU','已验证'],['性能','356条记录，32组比较，保留原始数据','已测量'],['真实模型效果','API地址和模型名尚待用户配置','待真实模型验收']],[245,635,264],150,476,23);
 notes(s,'不能把当前实现称为整个赛题已全部通过：B04真实动态决策和E01真实模型诊断仍待配置。固定流程仅用于机械链路回归。细项见矩阵，代码与镜像SHA256见构建manifest。');
}
// 14. Delivery and remaining acceptance.
{
 const s=slide('交付与后续验收','运行入口与复现依据：README.md');
 lines(s,['启动：scripts/start.ps1 → http://127.0.0.1:8765','复现：verify.sh、run_benchmarks.py 与原始日志','配置：host/local.env 中填写服务地址、模型名和本地密钥','补验：accept_model.py 跑真实多轮调查，再核对结论与观察'],{y:163,gap:105,size:29});
 text(s,'范围：单核教学内核、平面文件系统、单线程 Agent；没有宣称生产安全或硬实时。',68,611,1144,45,23,MUTED);
 notes(s,'交付包括代码、测试、实时页面、设计/协议/验收/性能文档、运行证据与本汇报。真实模型配置到位后需检查至少两次进程采样、文件证据、最终引用和故障恢复，并更新验收结论。');
}

const candidate=path.join(build,'candidate.pptx');
await (await PresentationFile.exportPptx(pres)).save(candidate);
for(const [i,s] of pres.slides.items.entries()){
 const blob=await pres.export({slide:s,format:'png',scale:1});
 await fs.writeFile(path.join(build,`slide-${String(i+1).padStart(2,'0')}.png`),new Uint8Array(await blob.arrayBuffer()));
}
await fs.writeFile(path.join(build,'notes.json'),JSON.stringify(pres.slides.items.map((s,i)=>({slide:i+1,notes:s.speakerNotes.text})),null,2),'utf8');
if(process.argv.includes('--final')){
 const filename=process.env.AGENTOS_DECK_NAME || 'AgentOS-结题汇报-待模型验收.pptx';
 const result=await finalizePresentation({workspaceDir:root,candidatePath:candidate,finalPath:path.join(output,filename),
  pythonExecutable:path.join(runtime,'python/python.exe'),
  integrityValidatorPath:path.join(skill,'container_tools/inspect_presentation_package_integrity.py'),
  layoutValidatorPath:path.join(skill,'container_tools/inspect_presentation_layout_geometry.py'),
  layoutArgs:['--expected-slide-size-emu','12192000,6858000','--validate-bullet-geometry','--validate-heading-fit',
    ...[2,3,4,7,8,13].flatMap(n=>['--require-native-table-slide',String(n)])],
  explicitTotalSlideCount:14,requiredNativeTableOwnerSlides:[2,3,4,7,8,13],requiredNativeChartOwnerSlides:[10,11,12],
  materializeLiteralChartWorkbooks:true,fontPolicy:{basis:'design',families:[FONT]},
  verifyArtifactToolImport:true,receiptPath:path.join(build,'validation.json')});
 console.log(JSON.stringify({finalPath:result.finalPath||path.join(output,filename),slides:14}));
}else console.log('Draft and 14 slide previews written to .presentation-build');
