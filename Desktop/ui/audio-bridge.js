(() => {
 'use strict';
 const trusted = window.__F87_DESKTOP_PAGE__;
 const local = ['http://tauri.localhost', 'tauri://localhost'].includes(location.origin);
 if (!trusted || window.top !== window || (!local && (location.origin !== trusted.origin || !location.pathname.startsWith(trusted.path)))) return;
 const call = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
 const $ = id => document.getElementById(id);
 const presets = [['#ff0000','红'],['#ff8800','橙'],['#ffff00','黄'],['#00ff00','绿'],['#00ffff','青'],['#0000ff','蓝'],['#ff00ff','紫'],['#ffffff','白'],['#000000','熄灭']];
 const validColor = value => /^#[0-9a-f]{6}$/i.test(value || '');
 const hex = rgb => '#' + rgb.map(v => v.toString(16).padStart(2,'0')).join('');
 let color = '#00aaff', custom = color, fps = 30, gain = 3, keepSending = false;
 let wanted = false, lastPage = null, revision = 0, queue = Promise.resolve(), refreshing = false, editTimer;
 try {
   const saved = JSON.parse(localStorage.getItem('f87-audio-settings') || '{}');
   const previous = localStorage.getItem('f87-audio-color');
   if (validColor(saved.color || previous)) color = (saved.color || previous).toLowerCase();
   custom = validColor(saved.custom) ? saved.custom : color;
   if (Number.isInteger(saved.fps) && saved.fps >= 1 && saved.fps <= 60) fps = saved.fps;
   if (Number.isFinite(saved.gain) && saved.gain >= 0.3 && saved.gain <= 8) gain = saved.gain;
   keepSending = saved.keepSending === true;
 } catch {}
 function args() { return { color: [1,3,5].map(i => parseInt(color.slice(i,i+2),16)), fps, gain, keepSending }; }
 function save() { try { localStorage.setItem('f87-audio-settings', JSON.stringify({color,custom,fps,gain,keepSending})); } catch {} }
 function showError(e) { if (wanted) $('audio-status').textContent = '音频响应失败：' + String(e); }
 function sync() {
   const current = ++revision;
   queue = queue.catch(() => {}).then(async () => {
     if (current !== revision) return;
     try {
       if (wanted) { window.__F87_BACKGROUND_ACTIVE__ = true; await call('audio_start', args()); }
       else { await call('audio_stop'); if (current === revision && !document.body.classList.contains('frames-page')) window.__F87_BACKGROUND_ACTIVE__ = false; }
       if (current === revision && wanted) await refresh();
     } catch (e) { showError(e); }
   });
 }
 function changed() {
   save(); clearTimeout(editTimer);
   if (wanted) editTimer = setTimeout(sync, 60);
 }
 function renderColor() {
   $('audio-color').value = color;
   $('audio-hex').value = color.slice(1).toUpperCase();
   $('audio-hex').style.color = color;
   $('audio-hex').setAttribute('aria-invalid', 'false');
   for (const button of $('audio-palette').children) button.setAttribute('aria-pressed', String(button.dataset.color === color));
   $('audio-custom-color').style.setProperty('--paint', custom);
   $('audio-custom-color').setAttribute('aria-pressed', String(color === custom));
 }
 function setColor(value, isCustom = false) {
   color = value.toLowerCase(); if (isCustom) custom = color;
   renderColor(); changed();
 }
 function pageChanged(page) {
   if (page === lastPage) return;
   lastPage = page; wanted = page === 'audio'; clearTimeout(editTimer);
   if (wanted) { window.__F87_BACKGROUND_ACTIVE__ = true; $('audio-status').textContent = '正在启动音频响应…'; }
   sync();
 }
 async function refresh() {
   if (refreshing || !wanted) return;
   refreshing = true;
   try {
     const state = await call('audio_status');
     if (!wanted) return;
     $('audio-status').textContent = state.error ? '音频响应失败：' + state.error : state.idle ? '静默待机' :
       '单色频谱 · ' + (state.connected ? '已连接' : '预览模式') + ' · ' + Number(state.fps || 0).toFixed(1) + ' FPS · ' + Number(state.packets || 0) + ' 包/帧 · ' + Number(state.latency_ms || 0).toFixed(1) + ' ms';
     document.querySelectorAll('#audio-keyboard .onboard-key').forEach(key => {
       const value = hex(state.colors?.[key.dataset.index] || [0,0,0]);
       key.style.setProperty('--paint', value); key.dataset.color = value;
     });
   } catch (e) { showError(e); } finally { refreshing = false; }
 }
 function bind() {
   document.documentElement.classList.add('f87-desktop');
   for (const [value,label] of presets) {
     const button = document.createElement('button'); button.type = 'button'; button.dataset.color = value;
     button.style.setProperty('--paint', value); button.setAttribute('aria-label', label); button.title = label;
     button.onclick = () => setColor(value); $('audio-palette').append(button);
   }
   $('audio-color').oninput = e => setColor(e.target.value, true);
   $('audio-custom-color').onclick = () => setColor(custom);
   $('audio-hex').oninput = e => {
     const value = e.target.value.trim(), valid = /^#?[0-9a-f]{6}$/i.test(value);
     e.target.setAttribute('aria-invalid', String(!valid));
     if (valid) setColor(value.startsWith('#') ? value : '#' + value, true);
   };
   $('audio-hex').onchange = renderColor;
   $('audio-hex').onkeydown = e => { if (e.key === 'Enter') { e.preventDefault(); renderColor(); e.target.blur(); } };
   $('audio-fps').value = fps; $('audio-fps-value').textContent = fps + ' FPS';
   $('audio-gain').value = gain; $('audio-gain-value').textContent = gain.toFixed(1) + ' ×';
   $('audio-keep-sending').checked = keepSending;
   $('audio-fps').oninput = e => { fps = Number(e.target.value); $('audio-fps-value').textContent = fps + ' FPS'; changed(); };
   $('audio-gain').oninput = e => { gain = Number(e.target.value); $('audio-gain-value').textContent = gain.toFixed(1) + ' ×'; changed(); };
   $('audio-keep-sending').onchange = e => { keepSending = e.target.checked; changed(); };
   renderColor();
   window.addEventListener('f87-page-change', e => pageChanged(e.detail.page));
   pageChanged(document.querySelector('[data-page][aria-current="page"]')?.dataset.page || 'backlight');
   setInterval(refresh, 33);
 }
 if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', bind, {once:true}); else bind();
})();
