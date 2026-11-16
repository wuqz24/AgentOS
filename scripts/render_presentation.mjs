// Render the finalized package, including its embedded editable chart data.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {fileURLToPath,pathToFileURL} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const skill=process.env.AGENTOS_PRESENTATIONS_SKILL || path.join(os.homedir(),'.codex/plugins/cache/openai-primary-runtime/presentations/26.1013.11914/skills/presentations');
process.env.RUNTIME_NODE_MODULES ||= path.join(os.homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules');
const {importRuntimeModule}=await import(pathToFileURL(path.join(skill,'container_tools/runtime_helpers.mjs')));
const {FileBlob,PresentationFile}=await importRuntimeModule('@oai/artifact-tool');
const file=process.argv[2] || path.join(root,'artifacts/delivery/AgentOS-结题汇报-待模型验收.pptx');
const pres=await PresentationFile.importPptx(await FileBlob.load(file));
for(const [i,slide] of pres.slides.items.entries()){
 const blob=await pres.export({slide,format:'png',scale:1});
 await fs.writeFile(path.join(root,'.presentation-build',`final-${String(i+1).padStart(2,'0')}.png`),new Uint8Array(await blob.arrayBuffer()));
}
console.log(`Rendered ${pres.slides.items.length} final slides`);
