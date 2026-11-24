import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=path.dirname(fileURLToPath(import.meta.url));
const meta=JSON.parse(execFileSync('cargo',['metadata','--format-version','1','--locked','--manifest-path',path.join(root,'src-tauri/Cargo.toml'),'--filter-platform','x86_64-pc-windows-msvc'],{encoding:'utf8',maxBuffer:32*1024*1024}));
const nodes=new Map(meta.resolve.nodes.map(n=>[n.id,n])),ids=new Set();
function visit(id){if(ids.has(id))return;ids.add(id);for(const d of nodes.get(id)?.deps||[])visit(d.pkg);}
visit(meta.resolve.root);
const packages=meta.packages.filter(p=>ids.has(p.id)&&p.source).sort((a,b)=>a.name.localeCompare(b.name)||a.version.localeCompare(b.version));
const destination=path.resolve(root,'licenses/generated');
if(path.dirname(destination)!==path.resolve(root,'licenses'))throw Error('Unexpected license destination');
fs.rmSync(destination,{recursive:true,force:true});fs.mkdirSync(destination,{recursive:true});
const copy=(from,to)=>{fs.mkdirSync(path.dirname(to),{recursive:true});fs.cpSync(from,to,{recursive:true});};
const inventory=[];
for(const pkg of packages){
 const base=path.dirname(pkg.manifest_path),name=pkg.name+'-'+pkg.version,out=path.join(destination,'rust',name);
 let entries=fs.readdirSync(base).filter(n=>/^(licen[sc]e|copying|notice|copyright|unlicense)/i.test(n));
 for(const entry of entries)copy(path.join(base,entry),path.join(out,entry));
 if(pkg.license_file&&fs.existsSync(path.resolve(base,pkg.license_file))){copy(path.resolve(base,pkg.license_file),path.join(out,path.basename(pkg.license_file)));entries.push(path.basename(pkg.license_file));}
 if(!entries.length){
  const fallback=pkg.name.startsWith('unic-')?'unic':pkg.name==='webview2-com-sys'?'webview2-com':pkg.name;
  const upstream=path.join(root,'licenses/upstream',fallback);
  if(!fs.existsSync(upstream)||!fs.readdirSync(upstream).length)throw Error('Missing full license text: '+name);
  copy(upstream,out);
 }
 if(pkg.license?.includes('MPL-2.0')){
  const registry=path.dirname(path.dirname(path.dirname(base)));
  const archive=path.join(registry,'cache',path.basename(path.dirname(base)),name+'.crate');
  if(!fs.existsSync(archive))throw Error('Missing MPL source archive: '+archive);
  copy(archive,path.join(out,name+'-source.crate'));
 }
 inventory.push({name:pkg.name,version:pkg.version,license:pkg.license,repository:pkg.repository,source:'https://crates.io/api/v1/crates/'+pkg.name+'/'+pkg.version+'/download',path:'rust/'+name});
}
for(const font of ['Noto-Sans-SC','Space-Grotesk'])copy(path.join(root,'licenses/upstream',font),path.join(destination,'fonts',font));
copy(path.join(root,'licenses/upstream/NSIS-3.11'),path.join(destination,'installer/NSIS-3.11'));
copy(path.join(root,'licenses/sources.json'),path.join(destination,'upstream-sources.json'));
fs.writeFileSync(path.join(destination,'inventory.json'),JSON.stringify(inventory,null,2)+'\n');
const header='# Third-party notices / 第三方声明\n\n本项目的许可证见 LICENSE。下列第三方组件保留各自许可证，不由本项目重新授权。\n\n## 字体\n\n- Noto Sans SC（noto-sans-sc-subset.woff2）：SIL Open Font License 1.1，见 third-party-licenses/fonts/Noto-Sans-SC/OFL.txt。上游：https://github.com/google/fonts/tree/main/ofl/notosanssc\n- Space Grotesk（space-grotesk-latin.woff2）：SIL Open Font License 1.1，见 third-party-licenses/fonts/Space-Grotesk/OFL.txt。上游：https://github.com/google/fonts/tree/main/ofl/spacegrotesk\n\n## 安装程序与系统组件\n\n- NSIS 3.11：完整许可和版权声明见 third-party-licenses/installer/NSIS-3.11/COPYING，包含 zlib/libpng、bzip2、CPL 1.0 条款。源码：https://sourceforge.net/projects/nsis/files/NSIS%203/3.11/\n- Microsoft Edge WebView2：安装程序包含微软官方引导程序，WebView2 运行时由微软单独安装，适用微软原始条款。https://developer.microsoft.com/microsoft-edge/webview2/\n- Windows SDK / WebView2 系统接口不因此被重新授权。\n\n## Rust 依赖\n\n根据 Cargo.lock 和 Windows x64 依赖图生成，包含直接、间接及构建依赖；这是保守清单，不表示每个依赖的代码都进入最终二进制。每个组件的完整许可及版权声明位于 third-party-licenses/rust/名称-版本/。\n\nMPL-2.0 组件 selectors 的对应未修改源码随包保存在该目录的 *-source.crate 中（tar.gz 格式），也可通过表中精确版本源码链接取得。\n\n源码仓库内，上述 third-party-licenses 对应 Desktop/licenses/generated。免安装分发请同时附带 LICENSE、本文和 third-party-licenses 目录。\n\n| 组件 | 版本 | 许可证 | 对应源码 |\n|---|---|---|---|\n';
fs.writeFileSync(path.join(root,'../THIRD_PARTY_NOTICES.md'),header+inventory.map(p=>'| '+p.name+' | '+p.version+' | '+p.license+' | [源码]('+p.source+') |').join('\n')+'\n');
for(const font of ['Noto-Sans-SC','Space-Grotesk'])copy(path.join(root,'licenses/upstream',font,'OFL.txt'),path.join(root,'../Web/licenses',font,'OFL.txt'));
const escape=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;');
let texts='';
function collect(dir){for(const entry of fs.readdirSync(dir,{withFileTypes:true})){const f=path.join(dir,entry.name);if(entry.isDirectory())collect(f);else if(!/\.(crate|json)$/i.test(entry.name))texts+='\n\n'+path.relative(destination,f)+'\n'+fs.readFileSync(f,'utf8');}}
collect(destination);
fs.mkdirSync(path.join(root,'ui'),{recursive:true});
fs.writeFileSync(path.join(root,'ui/third-party-notices.html'),'<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>第三方许可证</title><body><pre style="white-space:pre-wrap;word-break:break-word">'+escape(header+texts)+'</pre></body></html>');
console.log('Prepared full third-party license texts for '+inventory.length+' Rust packages, 2 fonts, and NSIS.');
