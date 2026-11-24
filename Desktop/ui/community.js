(function () {
  'use strict';

  const apiBase = (window.F87_COMMUNITY_API_BASE || document.currentScript.dataset.apiBase || 'https://api.sanfuhualv.com/f87pro-webhid').replace(/\/$/, '');
  const apiOrigin = new URL(apiBase, location.href).origin;
  const localFile = location.protocol === 'file:';
  const byId = id => document.getElementById(id);
  const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value);
  const isInteger = (value, min, max) => Number.isInteger(value) && value >= min && value <= max;
  const validText = (value, max) => typeof value === 'string' && [...value].length <= max && !/[\p{Cc}\p{Cf}\uFFFD]/u.test(value);

  const communityStyles = `
.community-panel{display:none;width:100%;padding:38px 0 48px}
.frames-mode-community .community-panel{display:block}
.frames-mode-community #frames-section{justify-content:flex-start}
.community-toolbar,.community-pagination,.community-detail-actions{display:flex;align-items:center;gap:18px;flex-wrap:wrap}
.community-toolbar{margin-bottom:28px}
.community-toolbar>span{margin-right:auto;font-size:13px;color:#777}
.community-account-actions{display:flex;align-items:center;gap:10px}
.community-account-trigger{display:inline-flex;align-items:center;gap:10px;max-width:230px}
.community-account-name{max-width:160px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.community-account-name[hidden]{display:none!important}
@media(max-width:680px){.community-account-name{max-width:100px}}
.community-panel button,.community-dialog button{font:inherit}
.community-button{border:1px solid #d8d8d8;background:#fff;border-radius:8px;padding:10px 16px;font-size:12px!important;line-height:1.5;cursor:pointer}
.community-button:hover:enabled{border-color:#333;background:#f5f5f5}
.community-button.primary{background:#161616;border-color:#161616;color:#fff;box-shadow:none}
.community-button.primary:hover:enabled{background:#383838}
.community-button:disabled{opacity:.45;cursor:default}
.community-panel button:focus-visible,.community-dialog :is(button,input,select):focus-visible{outline:2px solid #555!important;outline-offset:4px!important}
.community-grid{list-style:none;margin:0;padding:0;display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:26px 22px}
.community-card{min-width:0}
.community-card>button{display:block;width:100%;padding:0;text-align:left;border:1px solid #e3e3e3;border-radius:12px;background:#fff;overflow:hidden;transition:border-color .2s,transform .2s}
.community-card>button:hover{border-color:#999;transform:translateY(-2px)}
.community-card canvas{display:block;width:100%;height:auto;padding:16px 13px;background:#f5f5f5;aspect-ratio:620/280}
.community-card-title{display:block;font-size:14px;font-weight:600;margin:16px 16px 8px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.community-card-meta{display:block;color:#777;font-size:11px;line-height:1.8;margin:0 16px 16px;overflow-wrap:anywhere}
.community-message{font-size:13px;line-height:1.9;color:#777;margin:20px 0;white-space:pre-wrap;overflow-wrap:anywhere}
.community-message[data-error=true]{color:#a33232}
#community-owned-status:empty{display:none}
.community-pagination{justify-content:center;margin-top:30px}
.community-dialog{width:min(680px,calc(100vw - 40px));max-height:calc(100svh - 48px);padding:28px;border:1px solid #ddd;border-radius:16px;background:#fff;color:#222;overflow:auto;font-family:"Noto Sans SC","Microsoft YaHei",sans-serif}
.community-dialog::backdrop{background:#0005;backdrop-filter:blur(3px)}
.community-dialog-header{display:flex;align-items:flex-start;gap:20px;margin-bottom:20px}
.community-dialog-header h2{font-size:21px;line-height:1.5;margin:0;overflow-wrap:anywhere}
.community-dialog-header>button{margin-left:auto;font-size:26px;line-height:1;padding:0 4px;color:#777}
.community-detail-canvas{width:100%;height:auto;display:block;padding:18px 12px;background:#f5f5f5;border-radius:9px}
.community-detail-meta{color:#777;font-size:12px;line-height:1.9;overflow-wrap:anywhere}
.community-detail-actions{gap:10px;margin-top:22px}
.community-publish-form{display:grid;gap:20px}
.community-publish-form label{display:grid;gap:9px;font-size:12px;color:#555}
.community-publish-form input,.community-publish-form select,.community-share-result input{width:100%;min-width:0;padding:11px 12px;border:1px solid #d8d8d8;border-radius:7px;background:#fff;color:#222;font:inherit}
.community-share-result{margin:20px 0;font-size:12px;color:#555}.community-share-result>div{display:flex;gap:10px;margin-top:10px}.community-share-result input{font-family:monospace}.community-share-result p{line-height:1.8;color:#777}.community-share-result button{white-space:nowrap}
.community-publish-form input:read-only{background:#f6f6f6;color:#777}
.community-publish-form .community-message{margin:0}
.community-publish-form>button{justify-self:start}
.community-owned[hidden]{display:none!important}
.community-owned{margin-top:26px;padding-top:22px;border-top:1px solid #e3e3e3}
.community-owned h3{margin:0;font-size:16px}
.community-owned>.community-message{margin:12px 0}
.community-owned-list{list-style:none;margin:0;padding:0}
.community-owned-item{padding:16px 0;border-bottom:1px solid #eee}
.community-owned-name{margin:0;font-size:14px;font-weight:600;overflow-wrap:anywhere}
.community-owned-meta{margin:7px 0;color:#777;font-size:12px;line-height:1.8;overflow-wrap:anywhere}
.community-owned-item .community-detail-actions{margin-top:10px}
.community-button.danger{color:#a33232}
.community-owned .community-pagination{margin-top:14px;gap:12px}
@media(max-width:1120px){.community-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}
@media(max-width:680px){.community-panel{padding-top:24px}.community-toolbar{gap:10px;margin-bottom:22px}.community-grid{gap:16px 12px}.community-card canvas{padding:10px 5px}.community-card-title{font-size:12px;margin:12px 10px 6px}.community-card-meta{font-size:10px;margin:0 10px 12px}.community-dialog{padding:20px;width:calc(100vw - 28px)}.community-dialog-header h2{font-size:18px}.community-detail-actions{align-items:stretch}.community-detail-actions button{flex:1;white-space:nowrap}.community-toolbar>span{flex-basis:100%}}
@media(prefers-reduced-motion:reduce){.community-card>button{transition:none}.community-card>button:hover{transform:none}}

.frames-mode-community #frames-section > .onboard-editor{display:none}

`;
  const communityPanel = `
 <div id="frames-community" class="community-panel">
  <div class="community-toolbar"><span>最新发布</span><button id="community-share-open" class="community-button" type="button">输入分享码</button><button id="community-refresh" class="community-button" type="button">刷新</button><div class="community-account-actions"><button id="community-publish-open" class="community-button primary" type="button">发布当前动画</button><button id="community-account-open" class="community-button community-account-trigger" type="button" aria-label="账号管理" aria-haspopup="dialog" aria-controls="community-account"><span id="community-account-label">未登录</span><span id="community-account-name" class="community-account-name" hidden></span></button></div></div>
  <p id="community-status" class="community-message" role="status" aria-live="polite"></p>
  <ul id="community-grid" class="community-grid" aria-label="社区灯效作品" aria-busy="false"></ul>
  <div class="community-pagination"><button id="community-previous" class="community-button" type="button" hidden>上一页</button><span id="community-page-label" class="subtle">第 1 页</span><button id="community-next" class="community-button" type="button" hidden>下一页</button></div>
 </div>
`;
  const communityDialogs = `
<dialog id="community-detail" class="community-dialog" aria-labelledby="community-detail-title">
 <div class="community-dialog-header"><h2 id="community-detail-title">社区灯效</h2><button id="community-detail-close" type="button" aria-label="关闭作品详情">×</button></div>
 <canvas id="community-detail-canvas" class="community-detail-canvas" width="620" height="238" aria-label="灯效动画预览"></canvas>
 <p id="community-detail-meta" class="community-detail-meta"></p>
 <div id="community-share-result" class="community-share-result" hidden><label for="community-share-code">分享码</label><div><input id="community-share-code" readonly aria-label="作品分享码"><button id="community-share-copy" class="community-button" type="button">复制分享码</button></div><p>持有分享码的人可以查看此作品。</p></div>
 <div class="community-detail-actions"><button id="community-save" class="community-button" type="button" disabled>保存到全部动画</button><button id="community-edit" class="community-button" type="button" disabled>载入编辑器</button><button id="community-play" class="community-button primary" type="button" disabled>在键盘播放</button></div>
 <p id="community-detail-status" class="community-message" role="status" aria-live="polite"></p>
</dialog>
<dialog id="community-publish" class="community-dialog" aria-labelledby="community-publish-title">
 <div class="community-dialog-header"><h2 id="community-publish-title">发布动画</h2><button id="community-publish-close" type="button" aria-label="关闭发布窗口">×</button></div>
 <form id="community-publish-form" class="community-publish-form">
  <p id="community-publish-meta" class="community-message"></p>
  <p id="community-publish-policy" class="community-message">未登录可通过分享码分享；登录后可发布到公开社区。</p>
  <label>分享方式<select id="community-publish-visibility" name="visibility"><option value="unlisted">分享码分享</option><option value="public" disabled>公开到社区（需登录）</option></select></label>
  <label>动画名称（最多 40 字）<input id="community-publish-name" name="name" required maxlength="80" autocomplete="off"></label>
  <label>署名（选填，最多 32 字）<input id="community-publish-author" name="author" maxlength="64" autocomplete="nickname" placeholder="匿名"></label>
  <div class="community-detail-actions"><button id="community-publish-submit" class="community-button primary" type="submit">生成分享码</button><button id="community-publish-login" class="community-button" type="button">登录账号</button></div>
  <p id="community-publish-status" class="community-message" role="status" aria-live="polite"></p>
 </form>
</dialog>
<dialog id="community-account" class="community-dialog" aria-labelledby="community-account-title">
 <div class="community-dialog-header"><h2 id="community-account-title">账号管理</h2><button id="community-account-close" type="button" aria-label="关闭账号管理">×</button></div>
 <p id="community-account-state" class="community-message">正在检查登录状态…</p>
 <div class="community-detail-actions"><button id="community-account-login" class="community-button primary" type="button" disabled>通过统一账号登录</button><button id="community-account-switch" class="community-button" type="button" hidden>切换账号</button><button id="community-account-logout" class="community-button" type="button" hidden>退出登录</button><button id="community-account-refresh" class="community-button" type="button">刷新状态</button></div>
 <p id="community-account-status" class="community-message" role="status" aria-live="polite"></p>
 <section id="community-owned" class="community-owned" aria-labelledby="community-owned-title" hidden>
  <h3 id="community-owned-title">我的发布</h3>
  <p class="community-message">下架后作品仅自己可见，可重新上架。删除会永久移除作品。</p>
  <p id="community-owned-status" class="community-message" role="status" aria-live="polite"></p>
  <ul id="community-owned-list" class="community-owned-list" aria-label="我发布的作品" aria-busy="false"></ul>
  <div class="community-pagination"><button id="community-owned-prev" class="community-button" type="button" hidden>上一页</button><span id="community-owned-page" class="community-message"></span><button id="community-owned-next" class="community-button" type="button" hidden>下一页</button></div>
 </section>
</dialog>
<dialog id="community-share-dialog" class="community-dialog" aria-labelledby="community-share-title">
 <div class="community-dialog-header"><h2 id="community-share-title">通过分享码打开</h2><button id="community-share-close" type="button" aria-label="关闭分享码窗口">×</button></div>
 <form id="community-share-form" class="community-publish-form"><label>分享码<input id="community-share-input" name="shareCode" required maxlength="64" autocomplete="off" autocapitalize="off" spellcheck="false"></label><button id="community-share-submit" class="community-button primary" type="submit">打开作品</button><p id="community-share-status" class="community-message" role="status" aria-live="polite"></p></form>
</dialog>
`;

  function create(options) {
    const loadAuthor = () => options.loadSetting('f87-community-author-v1');
    const saveAuthor = author => options.saveSetting('f87-community-author-v1', author);
    const localAnimation = animation => ({ id: 'community-' + animation.id, name: animation.name, sequence: animation.sequence });
    const style = document.createElement('style');
    style.textContent = communityStyles;
    document.head.append(style);
    byId('frames-section').insertAdjacentHTML('beforeend', communityPanel);
    document.body.insertAdjacentHTML('beforeend', communityDialogs);
    const entry = document.querySelector('[data-frames-mode="community"]');
    entry.disabled = false;
    entry.removeAttribute('title');
    const grid = byId('community-grid');
    const detail = byId('community-detail');
    const publishDialog = byId('community-publish');
    const accountDialog = byId('community-account');
    const shareDialog = byId('community-share-dialog');
    const form = byId('community-publish-form');
    const status = byId('community-status');
    const keyNames = options.keys.map(key => String(key.index));
    let active = false;
    let loaded = false;
    let listController = null;
    let detailController = null;
    let cursors = [0];
    let nextCursor = 0;
    let selected = null;
    let snapshot = null;
    let requestKey = '';
    let posting = false;
    let previewRaf = null;
    const galleryPreviews = new Map();
    let galleryLoading = 0;
    let detailAnimationID = null;
    const detailPreview = { canvas: byId('community-detail-canvas'), started: 0, lastFrame: -1 };
    let detailTrigger = null;
    let submittedBody = null;
    let submittedIdentity = null;
    let account = { authenticated: false, loginAvailable: false };
    let accountKnown = false;
    let accountPending = null;
    let loginWindow = null;
    let loggingOut = false;
    let switchingAccount = false;
    let ownedOwner = '';
    let ownedController = null;
    let ownedCursors = [0];
    let ownedNextCursor = 0;
    let ownedMutating = false;
    let detailOwner = '';
    const galleryObserver = new IntersectionObserver(entries => {
      for (const entry of entries) {
        const state = galleryPreviews.get(entry.target.dataset.animationId);
        if (!state || state.canvas !== entry.target) continue;
        state.visible = entry.isIntersecting;
        if (!state.visible && detailAnimationID !== state.id) state.controller?.abort();
      }
      startPreview();
    });

    function message(element, text, failed = false) {
      element.textContent = text;
      element.dataset.error = String(failed);
    }

    function validFrame(frame) {
      return isObject(frame) && typeof frame.id === 'string' && /^[a-zA-Z0-9_-]{1,64}$/.test(frame.id)
        && isInteger(frame.brightness, 0, 100) && isObject(frame.colors)
        && Object.keys(frame.colors).length === keyNames.length
        && keyNames.every(key => Array.isArray(frame.colors[key]) && frame.colors[key].length === 3
          && frame.colors[key].every(channel => isInteger(channel, 0, 255)));
    }

    function validSequence(sequence) {
      return isObject(sequence) && sequence.version === 2 && isInteger(sequence.fps, 1, 30)
        && isInteger(sequence.gap, 0, 10) && /^#[\da-f]{6}$/i.test(sequence.customColor)
        && Array.isArray(sequence.frames) && sequence.frames.length >= 1 && sequence.frames.length <= 120
        && sequence.frames.every(validFrame) && new Set(sequence.frames.map(frame => frame.id)).size === sequence.frames.length
        && (sequence.loopAt === null || isInteger(sequence.loopAt, 0, sequence.frames.length - 1));
    }

    function validMetadata(item) {
      return isObject(item) && typeof item.id === 'string' && /^[\da-f]{32}$/.test(item.id)
        && validText(item.name, 40) && item.name.trim().length > 0 && validText(item.author, 32)
        && Number.isSafeInteger(item.createdAt) && item.createdAt > 0 && item.createdAt < 8640000000000;
    }

    async function apiRequest(path, settings = {}) {
      if (!/^https?:$/.test(new URL(apiBase, location.href).protocol)) throw Error('社区 API 地址须使用 HTTP 或 HTTPS。');
      const controller = new AbortController();
      const abort = () => controller.abort();
      settings.signal?.addEventListener('abort', abort, { once: true });
      if (settings.signal?.aborted) controller.abort();
      const timer = setTimeout(abort, 15000);
      try {
        const response = await fetch(apiBase + '/api' + path, { ...settings, signal: controller.signal, cache: 'no-store', credentials: localFile ? 'omit' : 'include' });
        if (Number(response.headers.get('content-length')) > 512 * 1024) throw Error('社区响应过大。');
        const text = await response.text();
        if (text.length > 512 * 1024) throw Error('社区响应过大。');
        let result;
        try { result = JSON.parse(text); } catch { throw Error('社区服务响应异常，请检查后端地址。'); }
        if (!response.ok) {
          const error = Error(typeof result?.error === 'string' ? result.error : '社区请求失败，请稍后重试。');
          error.status = response.status;
          throw error;
        }
        return result;
      } catch (error) {
        if (error.name === 'AbortError' && !settings.signal?.aborted) throw Error('社区请求超时，可稍后重试。');
        if (error instanceof TypeError) throw Error('无法连接社区，请检查网络和后端服务。');
        throw error;
      } finally {
        clearTimeout(timer);
        settings.signal?.removeEventListener('abort', abort);
      }
    }

    const request = (path, settings) => apiRequest('/animations' + path, settings);

    function isDialogOpen() {
      return detail.open || publishDialog.open || accountDialog.open || shareDialog.open;
    }

    function renderAccount() {
      const loggedIn = accountKnown && account.authenticated;
      const identity = loggedIn ? account.user.id : '';
      byId('community-owned').hidden = !loggedIn;
      if (identity !== ownedOwner) {
        ownedController?.abort();
        ownedOwner = identity;
        ownedCursors = [0];
        ownedNextCursor = 0;
        byId('community-owned-list').replaceChildren();
        message(byId('community-owned-status'), '');
        byId('community-owned-prev').hidden = true;
        byId('community-owned-next').hidden = true;
        byId('community-owned-page').textContent = '';
        if (detailOwner && detailOwner !== identity) closeDetail();
      }
      const accountButton = byId('community-account-open');
      accountButton.setAttribute('aria-label', loggedIn ? '账号管理 · ' + account.user.name : '账号管理 · 未登录');
      accountButton.title = loggedIn ? '账号管理 · ' + account.user.name : '账号管理';
      byId('community-account-label').hidden = loggedIn;
      byId('community-account-label').textContent = accountKnown ? '未登录' : '状态未知';
      byId('community-account-name').hidden = !loggedIn;
      byId('community-account-name').textContent = loggedIn ? account.user.name : '';
      byId('community-account-state').textContent = !accountKnown ? '登录状态暂时无法确认，请刷新状态。'
        : loggedIn ? '已登录 · ' + account.user.name : '当前未登录，可通过分享码分享动画。';
      byId('community-account-login').disabled = !accountKnown || !account.loginAvailable || localFile || loggingOut || switchingAccount;
      byId('community-account-login').hidden = loggedIn;
      if (localFile) byId('community-account-state').textContent = '本地文件可浏览社区和通过分享码分享。账号登录请打开线上静态网页。';
      byId('community-account-logout').hidden = !loggedIn;
      byId('community-account-logout').disabled = loggingOut || switchingAccount;
      byId('community-account-refresh').disabled = loggingOut || switchingAccount;
      byId('community-account-switch').hidden = !accountKnown || !account.switchAvailable || localFile;
      byId('community-account-switch').disabled = loggingOut || switchingAccount;
      byId('community-publish-login').hidden = loggedIn;
      byId('community-publish-login').disabled = !accountKnown || !account.loginAvailable || localFile || posting || loggingOut || switchingAccount;
      const visibility = byId('community-publish-visibility');
      visibility.querySelector('[value="public"]').disabled = !loggedIn;
      if (!loggedIn) visibility.value = 'unlisted';
      const author = byId('community-publish-author');
      const publicPost = visibility.value === 'public';
      if (publicPost && loggedIn) author.value = account.user.name;
      else if (author.readOnly) author.value = loadAuthor() || '';
      author.readOnly = publicPost;
      byId('community-publish-submit').textContent = publicPost ? '发布到社区' : '生成分享码';
      byId('community-publish-submit').disabled = posting || !accountKnown;
      byId('community-publish-policy').textContent = loggedIn
        ? '公开作品会显示在社区列表，发布者使用已登录账号。分享码作品可由持码者查看。'
        : '未登录可通过分享码分享；登录后可发布到公开社区。';
      if (accountKnown && !account.loginAvailable && !loggedIn) {
        byId('community-publish-policy').textContent += ' 当前服务尚未配置登录。';
      }
    }

    function refreshAccount() {
      if (accountPending) return accountPending;
      accountPending = apiRequest('/account').then(result => {
        if (!isObject(result) || typeof result.authenticated !== 'boolean' || typeof result.loginAvailable !== 'boolean'
          || (result.authenticated && (!isObject(result.user) || !validText(result.user.name, 32)
            || typeof result.user.id !== 'string' || !/^[\da-f]{64}$/.test(result.user.id)
            || typeof result.csrfToken !== 'string' || !/^[A-Za-z0-9_-]{43}$/.test(result.csrfToken)))) throw Error('登录状态数据无效。');
        account = result;
        accountKnown = true;
        message(byId('community-account-status'), '');
        return true;
      }).catch(error => {
        accountKnown = false;
        message(byId('community-account-status'), error.message, true);
        return false;
      }).finally(() => {
        accountPending = null;
        renderAccount();
      });
      return accountPending;
    }

    async function login(switchAccount = false) {
      if (loggingOut || switchingAccount || localFile) return;
      const target = apiBase + (switchAccount ? '/auth/login?switch=1' : '/auth/login');
      loginWindow = window.open(switchAccount ? 'about:blank' : target, 'f87-community-login', 'popup,width=520,height=720');
      if (!switchAccount) {
        if (!loginWindow) location.assign(target);
        return;
      }
      switchingAccount = true;
      renderAccount();
      try {
        if (!(await refreshAccount())) throw Error('无法确认登录状态，请稍后重试。');
        if (account.authenticated) {
          await apiRequest('/account/logout', { method: 'POST', headers: { 'X-CSRF-Token': account.csrfToken } });
          account = { authenticated: false, loginAvailable: account.loginAvailable, switchAvailable: account.switchAvailable };
          renderAccount();
        }
        if (loginWindow && !loginWindow.closed) loginWindow.location.replace(target);
        else location.assign(target);
      } catch (error) {
        if (loginWindow && !loginWindow.closed) loginWindow.close();
        message(byId('community-account-status'), error.message, true);
      } finally {
        switchingAccount = false;
        renderAccount();
      }
    }

    async function refreshAccountAndPosts() {
      if (await refreshAccount()) await loadOwned([0]);
    }

    function ownedButton(label, action, dangerous = false) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'community-button' + (dangerous ? ' danger' : '');
      button.textContent = label;
      button.onclick = action;
      return button;
    }

    async function loadOwned(history) {
      if (!accountKnown || !account.authenticated || !accountDialog.open) return;
      ownedController?.abort();
      const controller = new AbortController();
      ownedController = controller;
      const owner = account.user.id;
      const list = byId('community-owned-list');
      list.setAttribute('aria-busy', 'true');
      byId('community-owned-prev').disabled = true;
      byId('community-owned-next').disabled = true;
      message(byId('community-owned-status'), '正在加载作品…');
      try {
        const cursor = history.at(-1);
        const result = await apiRequest('/account/animations' + (cursor ? '?cursor=' + cursor : ''), { signal: controller.signal });
        if (!isObject(result) || !Array.isArray(result.items) || result.items.length > 20
          || !Number.isSafeInteger(result.nextCursor) || result.nextCursor < 0
          || (result.nextCursor && (result.items.length !== 20 || (cursor && result.nextCursor >= cursor)))
          || new Set(result.items.map(item => item.id)).size !== result.items.length
          || !result.items.every(item => validMetadata(item) && isInteger(item.frameCount, 1, 120) && isInteger(item.fps, 1, 30)
            && ['public', 'unlisted', 'withdrawn'].includes(item.visibility)
            && (item.visibility !== 'unlisted' || /^[A-Za-z0-9_-]{22}$/.test(item.shareCode)))) throw Error('作品列表数据无效。');
        if (controller.signal.aborted || owner !== ownedOwner || !accountDialog.open) return;
        ownedCursors = history;
        ownedNextCursor = result.nextCursor;
        const rows = result.items.map(item => {
          const row = document.createElement('li');
          row.className = 'community-owned-item';
          row.dataset.animationId = item.id;
          const title = document.createElement('p');
          title.className = 'community-owned-name';
          title.textContent = item.name;
          const meta = document.createElement('p');
          meta.className = 'community-owned-meta';
          const visibility = { public: '公开中', unlisted: '分享码分享', withdrawn: '已下架' }[item.visibility];
          meta.textContent = visibility + ' · ' + item.frameCount + ' 帧 · ' + item.fps + ' FPS · '
            + new Date(item.createdAt * 1000).toLocaleDateString('zh-CN');
          const actions = document.createElement('div');
          actions.className = 'community-detail-actions';
          const view = ownedButton('查看', () => openDetail(item.id, view, true));
          actions.append(view);
          if (item.visibility !== 'unlisted') {
            const action = item.visibility === 'public' ? 'withdraw' : 'restore';
            actions.append(ownedButton(action === 'withdraw' ? '下架' : '重新上架', () => manageOwned(item, action, owner)));
          }
          actions.append(ownedButton('删除', () => manageOwned(item, 'delete', owner), true));
          row.append(title, meta, actions);
          return row;
        });
        list.replaceChildren(...rows);
        byId('community-owned-prev').hidden = history.length === 1;
        byId('community-owned-next').hidden = !ownedNextCursor;
        byId('community-owned-page').textContent = result.items.length ? '第 ' + history.length + ' 页' : '';
        message(byId('community-owned-status'), result.items.length ? '' : '还没有作品。登录期间公开发布和通过分享码分享的作品会显示在这里。');
        return true;
      } catch (error) {
        if (!controller.signal.aborted && owner === ownedOwner) {
          message(byId('community-owned-status'), error.message + ' 可点击“刷新状态”重试。', true);
          if (error.status === 401) await refreshAccount();
        }
      } finally {
        if (ownedController === controller) {
          list.setAttribute('aria-busy', 'false');
          byId('community-owned-prev').disabled = false;
          byId('community-owned-next').disabled = false;
        }
      }
    }

    async function manageOwned(item, action, owner) {
      if (ownedMutating || loggingOut || !accountKnown || !account.authenticated || account.user.id !== owner) return;
      if (action === 'delete' && !window.confirm('永久删除“' + item.name + '”？删除后作品及分享码将失效。')) return;
      ownedMutating = true;
      for (const button of byId('community-owned-list').querySelectorAll('button')) button.disabled = true;
      try {
        await apiRequest('/account/animations/' + item.id + (action === 'delete' ? '' : '/' + action), {
          method: action === 'delete' ? 'DELETE' : 'POST', headers: { 'X-CSRF-Token': account.csrfToken }
        });
        loaded = false;
        if (owner !== ownedOwner) return;
        await loadOwned([0]);
        if (active) await loadPage([0]);
      } catch (error) {
        if (owner === ownedOwner) {
          message(byId('community-owned-status'), error.message, true);
          if (error.status === 401 || error.status === 403) await refreshAccount();
        }
      } finally {
        ownedMutating = false;
        for (const button of byId('community-owned-list').querySelectorAll('button')) button.disabled = false;
      }
    }

    byId('community-owned-prev').onclick = () => loadOwned(ownedCursors.slice(0, -1));
    byId('community-owned-next').onclick = () => { if (ownedNextCursor) loadOwned([...ownedCursors, ownedNextCursor]); };
    byId('community-account-open').onclick = () => {
      accountDialog.showModal();
      startPreview();
      refreshAccountAndPosts();
    };
    byId('community-account-close').onclick = () => accountDialog.close();
    byId('community-account-refresh').onclick = refreshAccountAndPosts;
    byId('community-account-login').onclick = () => login();
    byId('community-account-switch').onclick = () => login(true);
    byId('community-publish-login').onclick = () => login();
    byId('community-publish-visibility').onchange = renderAccount;
    accountDialog.addEventListener('close', () => { ownedController?.abort(); startPreview(); });
    byId('community-account-logout').onclick = async () => {
      if (loggingOut || switchingAccount) return;
      loggingOut = true;
      if (loginWindow && !loginWindow.closed) loginWindow.close();
      loginWindow = null;
      renderAccount();
      try {
        if (accountPending) await accountPending;
        await apiRequest('/account/logout', { method: 'POST', headers: { 'X-CSRF-Token': account.csrfToken } });
        account = { authenticated: false, loginAvailable: account.loginAvailable, switchAvailable: account.switchAvailable };
        accountKnown = true;
        renderAccount();
        message(byId('community-account-status'), '已退出登录。');
      } catch (error) {
        message(byId('community-account-status'), error.message, true);
      } finally {
        loggingOut = false;
        renderAccount();
      }
    };
    window.addEventListener('message', async event => {
      if (event.origin !== apiOrigin || !loginWindow || event.source !== loginWindow || event.data?.type !== 'f87-account') return;
      if (!event.data.ok) {
        message(byId('community-account-status'), '登录未完成，请重新登录。', true);
        message(byId('community-publish-status'), '登录未完成，请重新登录。', true);
        return;
      }
      if (await refreshAccount()) {
        byId('community-publish-visibility').value = 'public';
        renderAccount();
        message(byId('community-account-status'), '登录成功，可以公开发布作品。');
        if (accountDialog.open) await loadOwned([0]);
      }
    });

    byId('community-share-open').onclick = () => {
      byId('community-share-input').value = '';
      message(byId('community-share-status'), '');
      shareDialog.showModal();
      startPreview();
      byId('community-share-input').focus();
    };
    byId('community-share-close').onclick = () => shareDialog.close();
    shareDialog.addEventListener('close', startPreview);
    byId('community-share-form').onsubmit = async event => {
      event.preventDefault();
      const code = byId('community-share-input').value.trim();
      if (!/^[A-Za-z0-9_-]{22}$/.test(code)) {
        message(byId('community-share-status'), '请输入完整的 22 位分享码。', true);
        return;
      }
      byId('community-share-submit').disabled = true;
      message(byId('community-share-status'), '正在打开作品…');
      try {
        const animation = await apiRequest('/shares/' + code);
        if (!validMetadata(animation) || !validSequence(animation.sequence) || animation.visibility !== 'unlisted' || animation.shareCode !== code) throw Error('分享作品数据无效。');
        if (!active || !shareDialog.open) return;
        shareDialog.close();
        detailTrigger = byId('community-share-open');
        detail.showModal();
        showAnimation(animation);
      } catch (error) {
        message(byId('community-share-status'), error.message, true);
      } finally {
        byId('community-share-submit').disabled = false;
      }
    };
    byId('community-share-copy').onclick = async () => {
      try {
        await navigator.clipboard.writeText(byId('community-share-code').value);
        message(byId('community-detail-status'), '分享码已复制。');
      } catch {
        byId('community-share-code').select();
        message(byId('community-detail-status'), '已选中分享码，可手动复制。');
      }
    };

    function pagination() {
      byId('community-previous').hidden = cursors.length <= 1;
      byId('community-next').hidden = !nextCursor;
    }

    async function loadPage(targetCursors = cursors) {
      listController?.abort();
      const controller = new AbortController();
      listController = controller;
      grid.setAttribute('aria-busy', 'true');
      for (const id of ['community-refresh', 'community-previous', 'community-next']) byId(id).disabled = true;
      message(status, '正在加载社区灯效…');
      try {
        const cursor = targetCursors.at(-1);
        const result = await request(cursor ? '?cursor=' + cursor : '', { signal: controller.signal });
        if (!active || controller.signal.aborted) return;
        if (!isObject(result) || !Array.isArray(result.items) || result.items.length > 20
          || !Number.isSafeInteger(result.nextCursor) || result.nextCursor < 0
          || !result.items.every(item => validMetadata(item) && validFrame(item.preview)
            && isInteger(item.frameCount, 1, 120) && isInteger(item.fps, 1, 30))) throw Error('社区作品数据无效。');
        cursors = [...targetCursors];
        nextCursor = result.nextCursor;
        galleryObserver.disconnect();
        for (const state of galleryPreviews.values()) state.controller?.abort();
        galleryPreviews.clear();
        grid.replaceChildren();
        const started = performance.now();
        for (const item of result.items) {
          const card = document.createElement('li');
          card.className = 'community-card';
          const button = document.createElement('button');
          button.type = 'button';
          const canvas = document.createElement('canvas');
          canvas.width = 620;
          canvas.height = 238;
          canvas.setAttribute('aria-hidden', 'true');
          canvas.dataset.animationId = item.id;
          options.drawFrameCard(canvas, item.preview);
          const title = document.createElement('span');
          title.className = 'community-card-title';
          title.textContent = item.name;
          const meta = document.createElement('span');
          meta.className = 'community-card-meta';
          meta.textContent = (item.author || '匿名') + ' · ' + item.frameCount + ' 帧 · ' + item.fps + ' FPS';
          button.append(canvas, title, meta);
          button.onclick = () => openDetail(item.id, button);
          card.append(button);
          grid.append(card);
          galleryPreviews.set(item.id, {
            id: item.id, canvas, button, started, lastFrame: -1, visible: false,
            animation: null, promise: null, controller: null, failed: false
          });
          if (item.frameCount > 1) galleryObserver.observe(canvas);
        }
        startPreview();
        loaded = true;
        message(status, result.items.length ? '' : '还没有社区灯效，可以发布你的第一个作品。');
        byId('community-page-label').textContent = '第 ' + cursors.length + ' 页';
        pagination();
      } catch (error) {
        if (!controller.signal.aborted) message(status, error.message, true);
      } finally {
        if (listController === controller) {
          listController = null;
          grid.setAttribute('aria-busy', 'false');
          for (const id of ['community-refresh', 'community-previous', 'community-next']) byId(id).disabled = false;
        }
      }
    }

    function stopPreview() {
      if (previewRaf !== null) cancelAnimationFrame(previewRaf);
      previewRaf = null;
    }

    function loadGalleryAnimation(state) {
      if (state.animation) return Promise.resolve(state.animation);
      if (state.promise) return state.promise;
      const controller = new AbortController();
      state.controller = controller;
      galleryLoading++;
      state.promise = request('/' + state.id, { signal: controller.signal }).then(animation => {
        if (!validMetadata(animation) || !validSequence(animation.sequence)) throw Error('社区动画数据无效。');
        if (controller.signal.aborted) throw new DOMException('预览已暂停', 'AbortError');
        state.animation = animation;
        state.failed = false;
        state.button.title = '';
        startPreview();
        return animation;
      }).catch(error => {
        if (!controller.signal.aborted) {
          state.failed = true;
          state.button.title = '预览加载失败，点击作品重试';
        }
        throw error;
      }).finally(() => {
        state.promise = null;
        state.controller = null;
        galleryLoading--;
        loadVisibleAnimations();
      });
      return state.promise;
    }

    function loadVisibleAnimations() {
      if (!active || document.hidden || isDialogOpen()) return;
      for (const state of galleryPreviews.values()) {
        if (galleryLoading >= 2) break;
        if (state.visible && !state.animation && !state.promise && !state.failed) {
          loadGalleryAnimation(state).catch(() => {});
        }
      }
    }

    function paintPreview(state, sequence, now) {
      const count = sequence.frames.length;
      const step = Math.floor(Math.max(0, now - state.started) * sequence.fps / 1000);
      const loop = sequence.loopAt ?? 0;
      const index = step < count ? step : loop + (step - count) % (count - loop);
      if (state.lastFrame !== index) {
        options.drawFrameCard(state.canvas, sequence.frames[index]);
        state.lastFrame = index;
      }
      return count > 1;
    }

    function startPreview() {
      stopPreview();
      if (!active || document.hidden || isDialogOpen()) {
        for (const state of galleryPreviews.values()) {
          if (state.id !== detailAnimationID) state.controller?.abort();
        }
      }
      if (!active || document.hidden) return;
      loadVisibleAnimations();
      const tick = now => {
        previewRaf = null;
        let animated = false;
        if (detail.open && selected) {
          animated = paintPreview(detailPreview, selected.sequence, now);
        } else if (!isDialogOpen()) {
          for (const state of galleryPreviews.values()) {
            if (state.visible && state.animation) {
              animated = paintPreview(state, state.animation.sequence, now) || animated;
            }
          }
        }
        if (animated) previewRaf = requestAnimationFrame(tick);
      };
      tick(performance.now());
    }

    function detailActions(enabled) {
      for (const id of ['community-save', 'community-edit', 'community-play']) byId(id).disabled = !enabled;
    }

    function showAnimation(animation) {
      if (!validMetadata(animation) || !validSequence(animation.sequence)) throw Error('社区动画数据无效。');
      selected = animation;
      detailAnimationID = animation.id;
      const galleryState = galleryPreviews.get(animation.id);
      if (galleryState) {
        galleryState.animation = animation;
        galleryState.failed = false;
        galleryState.button.title = '';
      }
      detailPreview.started = performance.now();
      detailPreview.lastFrame = -1;
      byId('community-detail-title').textContent = animation.name;
      byId('community-detail-meta').textContent = (animation.author || '匿名') + ' · '
        + animation.sequence.frames.length + ' 帧 · ' + animation.sequence.fps + ' FPS · '
        + new Date(animation.createdAt * 1000).toLocaleDateString('zh-CN');
      const hasShareCode = animation.visibility === 'unlisted' && typeof animation.shareCode === 'string' && /^[A-Za-z0-9_-]{22}$/.test(animation.shareCode);
      byId('community-share-result').hidden = !hasShareCode;
      byId('community-share-code').value = hasShareCode ? animation.shareCode : '';
      byId('community-detail-canvas').hidden = false;
      detailActions(true);
      message(byId('community-detail-status'), '');
      startPreview();
    }

    async function openDetail(id, trigger, owned = false) {
      detailController?.abort();
      const controller = new AbortController();
      detailController = controller;
      detailAnimationID = id;
      detailOwner = owned ? ownedOwner : '';
      detailTrigger = trigger;
      selected = null;
      detailActions(false);
      byId('community-detail-title').textContent = '加载作品';
      byId('community-detail-meta').textContent = '';
      byId('community-detail-canvas').hidden = true;
      byId('community-share-result').hidden = true;
      message(byId('community-detail-status'), '正在获取动画…');
      detail.showModal();
      startPreview();
      try {
        const state = owned ? null : galleryPreviews.get(id);
        if (state?.controller?.signal.aborted) await state.promise.catch(() => {});
        if (controller.signal.aborted || !active || !detail.open) return;
        const animation = owned ? await apiRequest('/account/animations/' + id, { signal: controller.signal })
          : state ? await loadGalleryAnimation(state) : await request('/' + id, { signal: controller.signal });
        if (!controller.signal.aborted && active && detail.open) showAnimation(animation);
      } catch (error) {
        if (!controller.signal.aborted) message(byId('community-detail-status'), error.message, true);
      }
    }

    function closeDetail() {
      detailController?.abort();
      stopPreview();
      detail.close();
      selected = null;
      detailAnimationID = null;
      detailOwner = '';
      startPreview();
      if (active && detailTrigger?.isConnected) detailTrigger.focus();
    }

    function useSelected(mode) {
      if (!selected) return;
      try {
        options.applyAnimation(localAnimation(selected), mode, { backupName: '社区载入前草稿' });
        closeDetail();
      } catch (error) {
        message(byId('community-detail-status'), error.message, true);
      }
    }

    byId('community-save').onclick = () => {
      if (!selected) return;
      try {
        options.saveAnimation(localAnimation(selected));
        message(byId('community-detail-status'), '已保存到本机“全部动画”。');
      } catch (error) {
        message(byId('community-detail-status'), error.message, true);
      }
    };
    byId('community-edit').onclick = () => useSelected('edit');
    byId('community-play').onclick = () => useSelected('play');
    byId('community-detail-close').onclick = closeDetail;
    detail.addEventListener('cancel', event => { event.preventDefault(); closeDetail(); });
    detail.addEventListener('close', startPreview);
    byId('community-refresh').onclick = () => loadPage([0]);
    byId('community-previous').onclick = () => loadPage(cursors.slice(0, -1));
    byId('community-next').onclick = () => loadPage([...cursors, nextCursor]);

    byId('community-publish-open').onclick = () => {
      snapshot = options.sequenceSnapshot();
      // An end-of-strip marker becomes a loop start after more frames are inserted.
      if (snapshot.loopAt === snapshot.frames.length) snapshot.loopAt = null;
      if (!validSequence(snapshot)) {
        message(status, '请先在编辑动画中创建有效的序列帧，再发布到社区。', true);
        return;
      }
      requestKey = [...crypto.getRandomValues(new Uint8Array(16))].map(value => value.toString(16).padStart(2, '0')).join('');
      submittedBody = null;
      submittedIdentity = null;
      byId('community-publish-name').value = '';
      byId('community-publish-author').value = loadAuthor() || '';
      byId('community-publish-visibility').value = accountKnown && account.authenticated ? 'public' : 'unlisted';
      renderAccount();
      refreshAccount();
      byId('community-publish-meta').textContent = snapshot.frames.length + ' 帧 · ' + snapshot.fps + ' FPS';
      message(byId('community-publish-status'), '');
      publishDialog.showModal();
      startPreview();
      byId('community-publish-name').focus();
    };

    byId('community-publish-close').onclick = () => { if (!posting) publishDialog.close(); };
    publishDialog.addEventListener('cancel', event => { if (posting) event.preventDefault(); });
    publishDialog.addEventListener('close', startPreview);
    form.onsubmit = async event => {
      event.preventDefault();
      if (posting || !snapshot) return;
      if (!accountKnown && !(await refreshAccount())) {
        message(byId('community-publish-status'), '无法确认登录状态，请稍后重试。', true);
        return;
      }
      const name = byId('community-publish-name').value.trim();
      const author = byId('community-publish-author').value.trim();
      if (!validText(name, 40) || !name || !validText(author, 32)) {
        message(byId('community-publish-status'), '请检查名称和署名的长度及字符。', true);
        return;
      }
      const visibility = byId('community-publish-visibility').value;
      const identity = account.authenticated ? account.user.id : '';
      const body = JSON.stringify({ name, author, sequence: snapshot, visibility });
      if (new TextEncoder().encode(body).length > 256 * 1024) {
        message(byId('community-publish-status'), '动画数据超过 256 KiB，请减少帧数。', true);
        return;
      }
      if (submittedBody !== null && (submittedBody !== body || submittedIdentity !== identity)) {
        requestKey = [...crypto.getRandomValues(new Uint8Array(16))].map(value => value.toString(16).padStart(2, '0')).join('');
      }
      submittedBody = body;
      submittedIdentity = identity;
      posting = true;
      for (const control of form.elements) control.disabled = true;
      byId('community-publish-close').disabled = true;
      message(byId('community-publish-status'), '正在发布…');
      try {
        const headers = { 'Content-Type': 'application/json', 'Idempotency-Key': requestKey };
        if (account.authenticated) headers['X-CSRF-Token'] = account.csrfToken;
        const animation = await request('', { method: 'POST', headers, body });
        if (!validMetadata(animation) || !validSequence(animation.sequence)) throw Error('发布响应异常，可重试确认结果。');
        if (animation.visibility !== visibility || (visibility === 'unlisted' && !/^[A-Za-z0-9_-]{22}$/.test(animation.shareCode))) throw Error('分享结果异常，可重试确认结果。');
        if (visibility === 'unlisted') saveAuthor(author);
        publishDialog.close();
        if (visibility === 'public') loaded = false;
        if (active) {
          if (visibility === 'public') await loadPage([0]);
          detailTrigger = byId('community-publish-open');
          detail.showModal();
          showAnimation(animation);
          message(byId('community-detail-status'), visibility === 'public' ? '发布成功，作品已出现在社区。' : '分享码已生成，复制后发给对方即可查看。');
        }
      } catch (error) {
        message(byId('community-publish-status'), error.message, true);
        if (error.status === 401 || error.status === 403) await refreshAccount();
      } finally {
        posting = false;
        for (const control of form.elements) control.disabled = false;
        byId('community-publish-close').disabled = false;
        renderAccount();
      }
    };

    document.addEventListener('visibilitychange', startPreview);
    window.addEventListener('pagehide', stopPreview);
    window.addEventListener('pageshow', startPreview);
    pagination();
    renderAccount();
    return {
      setActive(value) {
        if (active === value) return;
        active = value;
        if (active) {
          refreshAccount();
          if (!loaded) loadPage();
          startPreview();
        } else {
          listController?.abort();
          closeDetail();
          publishDialog.close();
          accountDialog.close();
          shareDialog.close();
        }
      }
    };
  }

  window.F87Community = { create };
})();
