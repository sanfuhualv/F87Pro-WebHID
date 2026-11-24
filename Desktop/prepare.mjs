import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=path.dirname(fileURLToPath(import.meta.url));
fs.mkdirSync(path.join(root,'ui'),{recursive:true});
const html=fs.readFileSync(path.join(root,'../Web/F87-WebHID.html'),'utf8');
const desktopHtml=html.replace('<html lang="zh-CN">','<html lang="zh-CN" class="f87-desktop">').replace('<title>F87pro · WebHID</title>','<title>F87 Pro Desktop</title>')
 .replaceAll('打开 GitHub 项目仓库（新标签页）','使用系统浏览器打开 GitHub 项目仓库')
 .replaceAll('<span>WebHID</span>','<span>Desktop</span>')
 .replace('<footer><span>F87pro WebHID</span>','<footer><span>F87 Pro Desktop</span>')
 .replace('body:not(.frames-mode-play) #frames-open-desktop{display:none!important}','body #frames-open-desktop{display:none!important}');
fs.writeFileSync(path.join(root,'ui/index.html'),desktopHtml);
for(const name of ['community.js','noto-sans-sc-subset.woff2','space-grotesk-latin.woff2','favicon.png'])fs.copyFileSync(path.join(root,'../Web',name),path.join(root,'ui',name));
fs.cpSync(path.join(root,'../Web/licenses'),path.join(root,'ui/licenses'),{recursive:true});
const data=JSON.parse(html.match(/globalThis.F87_DATA = (.*);/)[1]);
fs.writeFileSync(path.join(root,'ui/keys.json'),JSON.stringify(data.keys,null,2)+'\n');
execFileSync('pwsh',['-NoProfile','-File',path.join(root,'build-icons.ps1')],{stdio:'inherit'});
console.log('Prepared full webpage, keyboard layout and matching desktop icons.');
