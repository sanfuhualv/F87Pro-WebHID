/* Keep the original webpage UI. Only hand its playback actions to the native host. */
(() => {
 'use strict';
 const trusted=window.__F87_DESKTOP_PAGE__;
 const local=['http://tauri.localhost','tauri://localhost'].includes(location.origin);
 if(!trusted||window.top!==window||(!local&&(location.origin!==trusted.origin||!location.pathname.startsWith(trusted.path))))return;
 const call=(cmd,args)=>window.__TAURI__.core.invoke(cmd,args),$=id=>document.getElementById(id);
 let page=null,community=null,last=null,busy=false,polling=false;
 const codeValid=code=>/^[A-Za-z0-9_-]{22}$/.test(code);
 function message(text,failed=false){
  const el=$(failed?'error':'message');if(el){el.textContent=String(text?.message||text);if(failed)el.classList.remove('hidden');}
 }
 async function task(action){
  if(busy)return;busy=true;window.__F87_BACKGROUND_ACTIVE__=true;
  try{await action();message('动画已缓存到本机，关闭窗口后由后台继续播放。');}
  catch(e){message(e,true);}finally{busy=false;await refresh();}
 }
 async function playDraft(){
  const sequence=page?.snapshot();if(!sequence?.frames?.length)return;
  await task(()=>call('play_sequence',{name:$('frames-animation-name')?.value.trim()||'编辑内容',sequence}));
 }
 async function playSelected(a,code){
  await task(async()=>{
   if(codeValid(code))return call('play_share',{code});
   const id=a.id?.replace(/^community-/,'');
   if(/^[a-f0-9]{32}$/.test(id)){
    try{return await call('play_public',{id});}catch{
     // Private/owned works may only be readable by the authenticated webpage.
     return call('play_sequence',{name:a.name,sequence:a.sequence});
    }
   }
   return call('play_sequence',{name:a.name,sequence:a.sequence});
  });
 }
 function unlock(){
  if(!document.body)return;
  for(const name of ['receiver-offline','receiver-connecting'])if(document.body.classList.contains(name))document.body.classList.remove(name);
  for(const el of [$('workspace'),document.querySelector('.sidebar')])if(el?.inert)el.inert=false;
  if($('connection-screen')){$('connection-screen').inert=true;$('connection-screen').setAttribute('aria-hidden','true');}
 }
 function wrap(module){
  if(!module?.create||module.__desktopWrapped)return module;
  const create=module.create;module.__desktopWrapped=true;
  module.create=function(options){
   page={snapshot:options.sequenceSnapshot,apply:options.applyAnimation};
   options.applyAnimation=(a,mode,meta)=>{
    const code=$('community-share-code')?.value||'';
    if(mode==='play'){
     window.__F87_BACKGROUND_ACTIVE__=true;
     // Use the webpage's existing animation library for offline access as well.
     options.saveAnimation(a);
    }
    const result=page.apply(a,mode,meta);
    if(mode==='play')void playSelected(a,code);
    return result;
   };
   const controller=create(options),setActive=controller.setActive;
   controller.setActive=()=>setActive.call(controller,document.body.classList.contains('frames-page')&&document.body.classList.contains('frames-mode-community'));
   community=controller;return controller;
  };return module;
 }
 let module=window.F87Community;
 Object.defineProperty(window,'F87Community',{configurable:true,get:()=>module,set:value=>{module=wrap(value);}});
 if(module)module=wrap(module);
 async function refresh(){
  if(polling)return;polling=true;
  try{last=await call('snapshot');if(!busy)window.__F87_BACKGROUND_ACTIVE__=last.running||last.audio;}
  catch(e){message(e,true);}finally{polling=false;}
 }
 function init(){
  if(!$('frames-section'))return;
  document.addEventListener('click',e=>{
   const button=e.target.closest?.('button');if(!button)return;
   // The original handlers still update their preview and play/pause controls.
   if(button.id==='frames-play'){
    if(button.getAttribute('aria-pressed')==='true'){window.__F87_BACKGROUND_ACTIVE__=false;void call('stop_playback').then(refresh).catch(e=>message(e,true));}else void playDraft();
   }
   else if(button.dataset.page&&button.dataset.page!=='frames'){window.__F87_BACKGROUND_ACTIVE__=false;void call('stop_playback').catch(e=>message(e,true));}
   else if(button.dataset.framesMode==='edit'){window.__F87_BACKGROUND_ACTIVE__=false;void call('stop_playback').catch(e=>message(e,true));}
   else if(button.dataset.framesMode==='play'||button.dataset.page==='frames')setTimeout(()=>{if(document.body.classList.contains('frames-mode-play')&&page?.snapshot()?.frames?.length)void playDraft();},0);
   else if(button.closest('#frames-animations')&&button.classList.contains('onboard-template-apply'))setTimeout(()=>{if(document.body.classList.contains('frames-mode-play'))void playDraft();},0);
  },true);
  document.addEventListener('pointerdown',e=>{if(e.target.closest?.('#frames-keyboard')&&document.body.classList.contains('frames-page')&&!document.body.classList.contains('frames-mode-play')){window.__F87_BACKGROUND_ACTIVE__=false;void call('stop_playback').catch(e=>message(e,true));}},true);
  unlock();new MutationObserver(unlock).observe(document.body,{attributes:true,attributeFilter:['class']});
  community?.setActive(true);void refresh();setInterval(()=>{if(!document.hidden)void refresh();},500);
 }
 if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',init,{once:true});else init();
})();
