/* A WebHID-compatible adapter for the unmodified full F87 webpage. */
(() => {
 'use strict';
 const trusted=window.__F87_DESKTOP_PAGE__;
 const local=['http://tauri.localhost','tauri://localhost'].includes(location.origin);
 if(!trusted||window.top!==window||(!local&&(location.origin!==trusted.origin||!location.pathname.startsWith(trusted.path))))return;
 const call=(cmd,args)=>window.__TAURI__.core.invoke(cmd,args);
 window.__F87_BACKGROUND_ACTIVE__=false;
 const hid=new EventTarget();
 class NativeDevice extends EventTarget{
  vendorId=0x3554;productId=0xfa09;productName='F87 PRO 2.4G · 本地驱动';opened=false;lease=null;polling=false;
  collections=[{usagePage:0xff02,usage:2,inputReports:[{reportId:0x13,items:[{reportSize:8,reportCount:19}]}],outputReports:[{reportId:0x13,items:[{reportSize:8,reportCount:19}]}]}];
  async open(){const s=await call('snapshot');if(!s.connected)throw Error('接收器未连接，正在自动重试');this.opened=true;}
  async close(){this.opened=false;this.lease=null;await call('hid_detach');}
  async sendReport(id,data){if(!this.opened||id!==0x13||!this.lease)throw Error('设备未连接或事务已结束');const payload=Array.from(new Uint8Array(data.buffer||data,data.byteOffset||0,data.byteLength));await call('hid_send',{token:this.lease,raw:[id,...payload]});}
  async poll(){if(!this.opened||!this.lease||this.polling)return;this.polling=true;const token=this.lease;try{const reports=await call('hid_poll',{token});if(token!==this.lease)return;for(const raw of reports){const e=new Event('inputreport');Object.defineProperties(e,{device:{value:this},reportId:{value:raw[0]},data:{value:new DataView(Uint8Array.from(raw.slice(1)).buffer)}});this.dispatchEvent(e);}}catch{}finally{this.polling=false;}}
 }
 const device=new NativeDevice();
 hid.requestDevice=async()=>{const s=await call('snapshot');return s.connected?[device]:[];};hid.getDevices=hid.requestDevice;
 Object.defineProperty(navigator,'hid',{configurable:true,value:hid});
 function wrap(api){
  if(!api?.Transport||api.__nativeHid)return api;const Base=api.Transport;
  api.Transport=class extends Base{
   exclusive(task){return super.exclusive(async()=>{const token=await call('hid_begin');device.lease=token;try{return await task();}finally{device.lease=null;await call('hid_end',{token}).catch(()=>{});}});}
   sendEncoded(encoded,gap,shouldSend){if(window.__F87_BACKGROUND_ACTIVE__||document.body.classList.contains('frames-mode-play'))return Promise.resolve({packets:0,payloadBytes:encoded.payload.length,sourceColors:encoded.sourceColors});return super.sendEncoded(encoded,gap,shouldSend);}
  };api.__nativeHid=true;return api;
 }
 let api=window.F87Protocol;Object.defineProperty(window,'F87Protocol',{configurable:true,get:()=>api,set:value=>{api=wrap(value);}});if(api)api=wrap(api);
 let manual=false,connecting=false,checking=false;
 document.addEventListener('click',e=>{const b=e.target.closest?.('button');if(b?.id==='disconnect'){manual=true;window.__F87_BACKGROUND_ACTIVE__=false;void call('stop_playback');}if(['connect','connection-screen-connect'].includes(b?.id))manual=false;},true);
 async function connection(){
  if(checking)return;checking=true;try{
   const s=await call('snapshot');
   if(!s.connected&&device.opened){device.opened=false;device.lease=null;const e=new Event('disconnect');Object.defineProperty(e,'device',{value:device});hid.dispatchEvent(e);}
   if(s.connected&&!device.opened&&!manual&&!connecting){const b=document.getElementById('connect');if(b&&!b.disabled){connecting=true;b.click();setTimeout(()=>{connecting=false;},1200);}}
  }catch{}finally{checking=false;}
 }
 setInterval(()=>void device.poll(),5);setInterval(()=>void connection(),500);
 if(local){
  const fetchOriginal=window.fetch.bind(window),apiBase=trusted.apiBase;
  window.fetch=async(input,options={})=>{
   const url=typeof input==='string'?input:input.url;
   if(typeof url==='string'&&url.startsWith(apiBase+'/api/')){
    if(options.signal?.aborted)throw new DOMException('Aborted','AbortError');
    const headers={};new Headers(options.headers).forEach((value,key)=>{headers[key]=value;});
    const r=await call('community_request',{path:url.slice(apiBase.length),method:options.method||'GET',body:options.body||'',headers});
    if(options.signal?.aborted)throw new DOMException('Aborted','AbortError');
    return new Response(r.body,{status:r.status,headers:{'Content-Type':'application/json'}});
   }return fetchOriginal(input,options);
  };
  const open=window.open.bind(window);
  window.open=(...args)=>{const popup=open(...args),target=String(args[0]||'');if(popup&&(target.startsWith(apiBase+'/auth/')||target==='about:blank')){const timer=setInterval(()=>{if(popup.closed){clearInterval(timer);window.dispatchEvent(new MessageEvent('message',{origin:new URL(apiBase).origin,source:popup,data:{type:'f87-account',ok:true}}));}},500);}return popup;};
 }
})();
