// Local iOS companion fixture. No files or agents are modified.
// node scripts/fixtures/noches-mobile-host.mjs
import { createRequire } from 'node:module';
import { randomUUID } from 'node:crypto';
import http from 'node:http';
import zlib from 'node:zlib';
const require = createRequire(new URL('../../edge/package.json', import.meta.url));
const { WebSocketServer } = require('ws');
const raw = process.argv.includes('--engine');
const port = Number(process.env.NOCHES_FIXTURE_PORT || (raw ? 28778 : 28777));
const profile = {id:'mobile-fixture',name:'Studio · fixture',endpoint:`ws://127.0.0.1:${port}`,token:'a'.repeat(64),deviceId:'fixture-mac'};
const now = () => new Date().toISOString();
const iso = offset => new Date(Date.now() + offset).toISOString();
const fnv = text => { let hash = 2166136261; for (let i = 0; i < text.length; i++) { hash ^= text.charCodeAt(i); hash = Math.imul(hash, 16777619); } return (hash >>> 0).toString(16); };

// ---------------------------------------------------------------------------
// A real 96x64 PNG so the phone decodes the generated-image part it is served.
// ---------------------------------------------------------------------------
const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();
function crc32(buffer) {
  let crc = -1;
  for (const byte of buffer) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ -1) >>> 0;
}
function pngChunk(type, data) {
  const chunk = Buffer.alloc(8 + data.length + 4);
  chunk.writeUInt32BE(data.length, 0);
  chunk.write(type, 4, 'ascii');
  data.copy(chunk, 8);
  chunk.writeUInt32BE(crc32(Buffer.concat([Buffer.from(type, 'ascii'), data])), 8 + data.length);
  return chunk;
}
function makePng(width, height) {
  const stride = width * 4 + 1;
  const raw = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    raw[y * stride] = 0;
    for (let x = 0; x < width; x++) {
      const at = y * stride + 1 + x * 4;
      raw[at] = 56 + Math.round((x / width) * 120);
      raw[at + 1] = 130 + Math.round((y / height) * 90);
      raw[at + 2] = 246;
      raw[at + 3] = 255;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), pngChunk('IHDR', ihdr), pngChunk('IDAT', zlib.deflateSync(raw)), pngChunk('IEND', Buffer.alloc(0))]);
}
const ICON_PNG = makePng(16, 16);
const SHOULD_INFO = 'Received on the fixture host. No real agent was started.';

// ---------------------------------------------------------------------------
// Workspace, chat, session, queue and doc state (real proto shapes).
// ---------------------------------------------------------------------------
const INITIAL_SPACES = [
  {id:'fixture-space',deviceId:profile.deviceId,path:'/tmp/noches-fixture',name:'Noches',gitDetected:true,checkoutId:'fixture-checkout-noches',createdAt:'2026-09-01T09:00:00Z'},
  {id:'fixture-website',deviceId:profile.deviceId,path:'/tmp/noches-fixture-website',name:'Website',gitDetected:true,checkoutId:'fixture-checkout-website',createdAt:'2026-09-02T09:00:00Z'},
];
const SPACES = structuredClone(INITIAL_SPACES);
const CHECKOUT = Object.fromEntries(SPACES.map(space => [space.id, space.checkoutId]));
const TEXTS = {
  'fixture-space': {
    'README.md': '# Noches fixture workspace\n\nNothing here touches a real checkout.\n',
    'Sources/Client.swift': '// Noches companion fixture\nstruct Client {\n    let connected = true\n}\n',
  },
  'fixture-website': {
    'README.md': '# Website fixture\n',
    'index.html': '<!doctype html>\n<title>Fixture site</title>\n',
  },
};
const INITIAL_TEXTS = structuredClone(TEXTS);
const WORKSPACE_IMAGES = {
  'fixture-space': {
    'public/favicon.png': {bytes:ICON_PNG,hash:fnv(`icon-${ICON_PNG.length}`),mimeType:'image/png'},
  },
};
const GENERATED_IMAGE = '/tmp/noches-fixture/.noches/generated/sidebar-preview.png';
const HARNESSES = [
  {id:'claude-code',name:'Claude Code',supportsSteering:true,steeringMode:'step-boundary',reasoningLevels:['low','medium','high','xhigh','max'],installed:true,enabled:true},
  {id:'codex',name:'Codex',supportsSteering:true,steeringMode:'step-boundary',reasoningLevels:['minimal','low','medium','high','xhigh'],installed:true,enabled:true},
  {id:'pi',name:'Pi',supportsSteering:true,steeringMode:'turn-boundary',reasoningLevels:['minimal','low','medium','high','xhigh','max'],installed:true,enabled:true},
];
const MODELS = {
  'claude-code': [
    {id:'anthropic/claude-sonnet-4-5',label:'Claude Sonnet 4.5',reasoningLevels:['low','medium','high','xhigh','max']},
    {id:'anthropic/claude-opus-4-1',label:'Claude Opus 4.1',reasoningLevels:['low','medium','high','xhigh','max']},
    {id:'anthropic/claude-haiku-4-5',label:'Claude Haiku 4.5',reasoningLevels:['minimal','low','medium']},
  ],
  codex: [
    {id:'openai/gpt-5-codex',label:'GPT-5 Codex',reasoningLevels:['minimal','low','medium','high','xhigh']},
    {id:'openai/gpt-5',label:'GPT-5',reasoningLevels:['minimal','low','medium','high','xhigh']},
  ],
  pi: [
    {id:'anthropic/claude-sonnet-4-5',label:'Claude Sonnet 4.5',reasoningLevels:['minimal','low','medium','high','xhigh','max']},
    {id:'google/gemini-3-pro',label:'Gemini 3 Pro',reasoningLevels:['low','medium','high']},
  ],
};
const CLAUDE_CONFIG = {harness:'claude-code',model:'anthropic/claude-sonnet-4-5',reasoning:'high',modelOptions:{},sandbox:'workspace-write'};
const CODEX_CONFIG = {harness:'codex',model:'openai/gpt-5-codex',reasoning:'medium',modelOptions:{},sandbox:'workspace-write'};
const PI_CONFIG = {harness:'pi',model:'anthropic/claude-sonnet-4-5',reasoning:'low',modelOptions:{},sandbox:'workspace-write'};
const USAGE = {
  'fixture-chat': {tokens:21400,window:200000,compactAt:168000,session:{input:18200,output:3400,cacheRead:12000}},
  'fixture-working': {tokens:132850,window:200000,compactAt:168000,session:{input:98400,output:21300,cacheRead:410000}},
};

let commands = [];
let approvalResolved = false;
let working = true;
let queued = [];
let uploads = new Map();
let attachments = new Map();
let chats = [];
let docs = new Map();
let terminals = new Map();
let cleanCheckouts = new Set();

function checkoutDiff(chat) {
  const clean = cleanCheckouts.has(chat?.checkoutId);
  return {checkoutId:chat?.checkoutId ?? CHECKOUT['fixture-space'],cwd:chat?.cwd ?? '/tmp/noches-fixture',
    checksum:clean ? 'fixture-clean' : 'fixture-dirty',truncated:false,
    files:clean ? [] : [{path:'Sources/Client.swift',status:'modified',additions:1,deletions:1,binary:false}],
    additions:clean ? 0 : 1,deletions:clean ? 0 : 1,
    patch:clean ? '' : 'diff --git a/Sources/Client.swift b/Sources/Client.swift\n--- a/Sources/Client.swift\n+++ b/Sources/Client.swift\n@@ -1 +1 @@\n-let connected = false\n+let connected = true\n'};
}
function openTerminal(chat, output = 'Fixture shell ready\r\n$ ') {
  const terminal = {id:randomUUID(),cwd:chat?.cwd ?? '/tmp/noches-fixture',shell:'zsh'};
  terminals.set(terminal.id,{...terminal,event:{type:'data',seq:1,data:Buffer.from(output).toString('base64')}});
  return terminal;
}

function chatRow(overrides) {
  return {id:'',deviceId:profile.deviceId,title:null,archived:false,cwd:null,branch:null,checkoutId:null,
    spaceId:null,config:null,lastMessagePreview:null,lastMessageAt:null,lastSeenAt:null,
    createdAt:'2026-09-20T09:00:00Z',...overrides};
}
function initialChats() {
  return [
    chatRow({id:'fixture-chat',title:'Build the mobile companion',spaceId:'fixture-space',branch:'companion/mobile',
      checkoutId:CHECKOUT['fixture-space'],cwd:'/tmp/noches-fixture',config:CLAUDE_CONFIG,
      lastMessagePreview:'The companion is ready for a closer look.',lastMessageAt:iso(-2*60*60e3),lastSeenAt:iso(-60*60e3)}),
    chatRow({id:'fixture-approval',title:'Review the authentication flow',spaceId:'fixture-space',branch:'fix/session-auth',
      checkoutId:CHECKOUT['fixture-space'],cwd:'/tmp/noches-fixture',config:CLAUDE_CONFIG,
      lastMessagePreview:'Ready to run the migration. Allow this command?',lastMessageAt:iso(-5*60e3)}),
    chatRow({id:'fixture-working',title:'Polish the desktop sidebar',spaceId:'fixture-space',branch:'design/sidebar',
      checkoutId:CHECKOUT['fixture-space'],cwd:'/tmp/noches-fixture',config:CODEX_CONFIG,
      lastMessagePreview:'Refining spacing and the session status indicators.',lastMessageAt:iso(-60e3),lastSeenAt:iso(-90e3)}),
    chatRow({id:'fixture-failed',title:'Deploy the fixture gateway',spaceId:'fixture-website',branch:'deploy/gateway',
      checkoutId:CHECKOUT['fixture-website'],cwd:'/tmp/noches-fixture-website',config:PI_CONFIG,
      lastMessagePreview:'permission denied: /usr/local/bin/noches-connect',lastMessageAt:iso(-10*60e3)}),
    chatRow({id:'fixture-done',title:'Add keyboard shortcuts',spaceId:'fixture-space',branch:'feat/shortcuts',
      checkoutId:CHECKOUT['fixture-space'],cwd:'/tmp/noches-fixture',config:CLAUDE_CONFIG,
      lastMessagePreview:'All checks passed. Changes are ready to review.',lastMessageAt:iso(-3*60*60e3),lastSeenAt:iso(-4*60*60e3)}),
    chatRow({id:'fixture-archived',title:'Explore connection options',archived:true,spaceId:'fixture-space',branch:'research/remote',
      checkoutId:CHECKOUT['fixture-space'],cwd:'/tmp/noches-fixture',config:CLAUDE_CONFIG,
      lastMessagePreview:'Tailscale keeps the host private across networks.',lastMessageAt:iso(-30*24*60*60e3),lastSeenAt:iso(-30*24*60*60e3)}),
    chatRow({id:'fixture-home',title:'Scratch notes',cwd:'~',config:CLAUDE_CONFIG,
      lastMessagePreview:'Paste the pairing code into the phone.',lastMessageAt:iso(-30*60e3),lastSeenAt:iso(-20*60e3)}),
    chatRow({id:'fixture-website',title:'Pricing page copy pass',spaceId:'fixture-website',branch:'main',
      checkoutId:CHECKOUT['fixture-website'],cwd:'/tmp/noches-fixture-website',config:PI_CONFIG,
      lastMessagePreview:'Tightened the hero copy and the plan names.',lastMessageAt:iso(-5*60*60e3),lastSeenAt:iso(-5*60*60e3+30*60e3)}),
    chatRow({id:'fixture-idle',title:'Fix flaky transport test',spaceId:'fixture-space',branch:'test/transport',
      checkoutId:CHECKOUT['fixture-space'],cwd:'/tmp/noches-fixture',config:CLAUDE_CONFIG,
      lastMessagePreview:'Waiting on a second failing run before reopening.',lastMessageAt:iso(-26*60*60e3),lastSeenAt:iso(-25*60*60e3)}),
  ];
}
function sessionRows() {
  return [
    {lastCompletedTurn:null,chatId:'fixture-working',deviceId:profile.deviceId,status:working?'working':'idle',
      startedAt:iso(-120e3),updatedAt:now()},
    {lastCompletedTurn:null,chatId:'fixture-approval',deviceId:profile.deviceId,status:approvalResolved?'idle':'awaitingInput',
      startedAt:iso(-45e3),updatedAt:now()},
    {lastCompletedTurn:null,chatId:'fixture-failed',deviceId:profile.deviceId,status:'errored',
      startedAt:iso(-12*60e3),updatedAt:iso(-10*60e3)},
    {lastCompletedTurn:null,chatId:'fixture-chat',deviceId:profile.deviceId,status:'idle',startedAt:null,updatedAt:now()},
    {lastCompletedTurn:null,chatId:'fixture-idle',deviceId:profile.deviceId,status:'idle',startedAt:null,updatedAt:now()},
  ];
}
function message(id, role, parts, extra = {}) {
  return {id,role,parts,createdAt:Date.now()-60_000,deviceId:profile.deviceId,...extra};
}
function toolPart(id, call, extra = {}) {
  return {id,kind:'tool',call,resolved:true,isError:false,...extra};
}
function workingTranscript() {
  return [
    message('fixture-working-user','user',[{id:'fixture-working-user-text',kind:'text',
      text:'Polish the desktop sidebar and verify the split-pane regressions.'}]),
    message('fixture-working-turn','assistant',[
      {id:'fixture-working-reasoning',kind:'reasoning',
        text:'The sidebar chrome lives in pane/chrome.rs and the status dot comes from SessionState, so tinting must follow the shared palette rather than local colors.'},
      toolPart('fixture-tool-read',{kind:'readFile',path:'crates/ui/src/shell.rs'},{output:'fn render_chat_row(...)'}),
      toolPart('fixture-tool-search',{kind:'search',pattern:'status_palette',path:'crates/ui'},{output:'9 matches in 4 files'}),
      toolPart('fixture-tool-edit',{kind:'editFile',path:'crates/ui/src/pane/chrome.rs',oldString:'px(6.0)',newString:'px(8.0)'},{output:'Updated 1 file'}),
      toolPart('fixture-tool-write',{kind:'writeFile',path:'crates/ui/src/shell/project_icon.rs',content:'pub(super) const ICON_PATHS'},{output:'Wrote 1 file'}),
      toolPart('fixture-tool-exec',{kind:'exec',command:'cargo test -p zeron-ui --lib pane::'},{output:'test result: ok. 42 passed; 0 failed'}),
      toolPart('fixture-tool-fetch',{kind:'webFetch',url:'https://docs.rs/loro/latest/loro/struct.MovableList.html'},{output:'MovableList in loro - Rust'}),
      toolPart('fixture-tool-mcp',{kind:'mcp',server:'linear',tool:'create_issue',input:{title:'Sidebar polish tracking'}},{output:'LIN-482 created'}),
      toolPart('fixture-tool-failed',{kind:'exec',command:'cargo test -p zeron-ui --lib transcript::'},{isError:true,output:'error: test failed: transcript::follow::pins_to_latest'}),
      {id:'fixture-tool-agent-running',kind:'tool',isError:false,resolved:false,
        call:{kind:'unknown',name:'Agent: audit the sidebar hit targets',input:{description:'audit the sidebar hit targets',subagent_type:'explore'}},
        subagentRef:'fixture-working-agent-1',subagentStatus:'running',subagentTail:'Checking pane::hit_test tolerances...'},
      {id:'fixture-tool-agent-done',kind:'tool',isError:false,resolved:true,
        call:{kind:'unknown',name:'Agent: verify divider drag equalize',input:{description:'verify divider drag equalize'}},
        subagentRef:'fixture-working-agent-2',subagentStatus:'done',subagentTail:'4 regressions green'},
      {id:'fixture-working-text',kind:'text',
        text:'Spacing is tightened across the pane header.\n\n```swift\nfunc paneHeader(_ chat: HostChat) -> some View {\n    ...\n}\n```\n\nThe split-pane regressions are green; the transcript failure above is the known flake.'},
      {id:'fixture-working-image',kind:'image',path:GENERATED_IMAGE,name:'sidebar-preview.png',mimeType:'image/png'},
    ],{status:'streaming'}),
  ];
}
function subagentTranscript(id, description, tail) {
  return [
    message(`${id}-user`,'user',[{id:`${id}-user-text`,kind:'text',text:description}]),
    message(id,'assistant',[{id:`${id}-text`,kind:'text',text:tail}],{status:'streaming'}),
  ];
}
function approvalEntry() {
  return message('approval-message','assistant',[{id:'approval-part',kind:'input',requestId:'approval-request',
    questions:[{id:'approve-command',header:'Run migration',question:'Allow the migration command on your computer?',options:['Allow','Deny']}],
    resolved:approvalResolved}],{status:'streaming'});
}
function initialDocs() {
  return new Map([
    ['fixture-chat',[message('reply','assistant',[{id:'text',kind:'text',
      text:'Your computer is connected.\n\nStart a session, send a message, or review a request from here.'}],{status:'complete'})]],
    ['fixture-approval',[approvalEntry()]],
    ['fixture-working',workingTranscript()],
    ['fixture-working-agent-1',subagentTranscript('fixture-working-agent-1','audit the sidebar hit targets','Checked pane::hit_test tolerances; 14pt drop targets hold at 320pt wide.')],
    ['fixture-working-agent-2',subagentTranscript('fixture-working-agent-2','verify divider drag equalize','Ran the split-pane suite: 4 regressions green.')],
    ['fixture-failed',[
      message('fixture-failed-user','user',[{id:'fixture-failed-user-text',kind:'text',text:'Install the gateway service and start it.'}]),
      message('fixture-failed-turn','assistant',[
        toolPart('fixture-failed-exec',{kind:'exec',command:'noches-connect serve --bind 0.0.0.0:27657'},{isError:true,output:'permission denied: /usr/local/bin/noches-connect'}),
        {id:'fixture-failed-error',kind:'error',message:'permission denied: /usr/local/bin/noches-connect'},
      ],{status:'complete'}),
    ]],
    ['fixture-done',[
      message('fixture-done-user','user',[{id:'fixture-done-user-text',kind:'text',text:'Wrap up the keyboard shortcuts work.'}]),
      message('fixture-done-turn','assistant',[{id:'fixture-done-text',kind:'text',text:'All checks passed. Changes are ready to review.'}],{status:'complete'}),
    ]],
  ]);
}
function resetState() {
  SPACES.splice(0,SPACES.length,...structuredClone(INITIAL_SPACES));
  for (const key of Object.keys(TEXTS)) delete TEXTS[key];
  Object.assign(TEXTS,structuredClone(INITIAL_TEXTS));
  terminals = new Map();
  cleanCheckouts = new Set();
  commands = [];
  approvalResolved = false;
  working = true;
  queued = [
    {chatId:'fixture-working',id:'fixture-queue-1',text:'Also check the compact status row at 320pt wide.',
      issuedBy:profile.deviceId,issuedAt:Date.now()-90e3},
    {chatId:'fixture-working',id:'fixture-queue-2',text:'Then re-run the pane split regressions.',
      issuedBy:profile.deviceId,issuedAt:Date.now()-60e3,editedAt:Date.now()-45e3},
  ];
  uploads = new Map();
  attachments = new Map([[GENERATED_IMAGE,{bytes:makePng(96,64),name:'sidebar-preview.png',mimeType:'image/png'}]]);
  chats = initialChats();
  docs = initialDocs();
}

// ---------------------------------------------------------------------------
// Watch snapshots.
// ---------------------------------------------------------------------------
function baseline(entries) {
  const out = {};
  for (const entry of entries) {
    const parts = {};
    for (const part of entry.parts) parts[part.id] = typeof part.text === 'string' ? Buffer.byteLength(part.text) : 0;
    out[entry.id] = parts;
  }
  return out;
}
function docFrame(chatId) {
  const entries = docs.get(chatId) ?? [];
  const frame = {reset:entries,replayBaseline:{entries:baseline(entries)}};
  const usage = USAGE[chatId];
  if (usage) frame.contextUsage = usage;
  return frame;
}
const snapshots = {
  WatchChats: () => chats,
  WatchSpaces: () => SPACES,
  WatchSessions: () => sessionRows(),
  WatchDocMessages: params => docFrame(params?.chatId),
  WatchQueue: params => ({items:queued.filter(item => item.chatId === params?.chatId).map(({chatId, ...item}) => item)}),
  WatchCheckoutDiffs: () => SPACES.map(space => checkoutDiff({checkoutId:CHECKOUT[space.id],cwd:space.path})),
  SubscribeTerminal: params => terminals.get(params?.terminalId)?.event,
};
function mimeFor(name) {
  const ext = (name.split('.').pop() ?? '').toLowerCase();
  return {png:'image/png',jpg:'image/jpeg',jpeg:'image/jpeg',gif:'image/gif',webp:'image/webp',svg:'image/svg+xml',bmp:'image/bmp',tif:'image/tiff',tiff:'image/tiff',ico:'image/x-icon'}[ext] ?? 'application/octet-stream';
}
function sanitize(name) {
  return name.replace(/[^A-Za-z0-9._-]/g,'_') || 'image';
}
function chatFor(id) { return chats.find(chat => chat.id === id); }
function appendDispatch(chatId, text) {
  const entries = docs.get(chatId) ?? [];
  entries.push(message(randomUUID(),'user',[{id:randomUUID(),kind:'text',text}]));
  entries.push(message(randomUUID(),'assistant',[{id:randomUUID(),kind:'text',text:SHOULD_INFO}],{status:'complete'}));
  docs.set(chatId,entries);
  const chat = chatFor(chatId);
  if (chat) { chat.lastMessageAt = now(); chat.lastMessagePreview = SHOULD_INFO; }
}

resetState();

// ---------------------------------------------------------------------------
// Transport + RPC.
// ---------------------------------------------------------------------------
const connections = new Set();
const host = http.createServer((req,res) => {
  res.setHeader('content-type','application/json');
  if(req.url === '/reset' && req.method === 'POST') {resetState();return res.end('{}');}
  if(req.url === '/health') return res.end(JSON.stringify({fixture:true}));
  if(req.url === '/commands') return res.end(JSON.stringify(commands));
  if(req.url === '/change-file' && req.method === 'POST') {
    TEXTS['fixture-space']['Sources/Client.swift']='// Changed on host\n';
    return res.end('{}');
  }
  res.statusCode=404; res.end('{}');
});
const server = new WebSocketServer({server:host,verifyClient:({req})=>raw || req.headers.authorization === `Bearer ${profile.token}`});
server.on('connection',socket=>{
  let seq=0,received=0,welcomed=raw;
  const watches=new Map();
  const send=frame=>socket.readyState===1 && (!raw || !frame.t) && socket.send(JSON.stringify(frame));
  const reply=frame=>{
    if(raw) {send(frame);return;}
    const payload=JSON.stringify(frame);
    // Split at JS character boundaries; surrogate pairs stay together.
    let chunk='';
    for(const char of payload) {
      if(Buffer.byteLength(chunk+char)>32768) {send({t:'data',seq:++seq,payload:chunk,end:false});chunk='';}
      chunk+=char;
    }
    send({t:'data',seq:++seq,payload:chunk,end:true});
  };
  const reemit=()=>{for(const [id,{method,params}] of watches) reply({id,item:snapshots[method](params)});};
  const refreshSessions=()=>{for(const [id,{method,params}] of watches) if(method==='WatchSessions') reply({id,item:snapshots.WatchSessions(params)});};
  connections.add(refreshSessions);
  socket.on('close',()=>connections.delete(refreshSessions));
  socket.on('message',bytes=>{
    try {
      const frame=raw ? {t:'data',seq:received+1,payload:bytes.toString(),end:true} : JSON.parse(bytes);
      if(frame.t==='hello') {
        if(frame.version!==1 || frame.resume || frame.cursor!==0) return socket.close();
        welcomed=true; return send({t:'welcome',version:1,resumed:false,received:0});
      }
      if(!welcomed) return socket.close();
      if(frame.t==='ping') return send({t:'pong'});
      if(frame.t==='ack') return;
      if(frame.t!=='data' || frame.end!==true || frame.seq!==++received) return socket.close();
      const rpc=JSON.parse(frame.payload);
      if(rpc.cancel) {watches.delete(rpc.id);return send({t:'ack',seq:frame.seq});}
      if(rpc.method==='FixtureDropMutation') {commands.push(rpc);socket.terminate();return;}
      send({t:'ack',seq:frame.seq});
      if(rpc.method==='EngineInfo') return reply({id:rpc.id,ok:{deviceId:profile.deviceId,workspaceScope:'local'}});
      if(rpc.method==='ListHarnesses') return reply({id:rpc.id,ok:HARNESSES});
      if(rpc.method==='ListModels') return reply({id:rpc.id,ok:Object.hasOwn(MODELS,rpc.params?.harness) ? MODELS[rpc.params.harness] : MODELS['claude-code']});
      if(rpc.method==='ListFolders') {
        commands.push(rpc);
        const path=rpc.params?.path ?? '/tmp';
        const folders={
          '/':[{name:'tmp',isDir:true,isRepo:false}],
          '/tmp':[{name:'noches-fixture',isDir:true,isRepo:true},{name:'mobile-qa-project',isDir:true,isRepo:true},{name:'notes',isDir:true,isRepo:false}],
          '/tmp/noches-fixture':[{name:'Sources',isDir:true,isRepo:false}],
          '/tmp/mobile-qa-project':[],
          '/tmp/notes':[],
        };
        if(!Object.hasOwn(folders,path)) return reply({id:rpc.id,err:`Folder not found: ${path}`});
        return reply({id:rpc.id,ok:{path,entries:folders[path],truncated:false}});
      }
      if(rpc.method==='ListDrives') return reply({id:rpc.id,ok:{drives:[{name:'System',path:'/'}]}});
      if(rpc.method==='ListBranches') return reply({id:rpc.id,ok:['main','dev']});
      if(rpc.method==='CreateWorktree') {
        commands.push(rpc);
        return reply({id:rpc.id,ok:{repoPath:rpc.params.repoPath,path:'/tmp/fixture-worktrees/qa-fox',
          branch:'zeron/qa-fox',name:'qa-fox',checkoutId:'fixture-worktree'}});
      }
      if(rpc.method==='ListCommands') return reply({id:rpc.id,ok:[{name:'review',description:'Review local changes'},{name:'compact',description:'Compact context'}]});
      if(['SearchFiles','SearchWorkspaceFiles'].includes(rpc.method)) {
        commands.push(rpc);
        const spaceId=chatFor(rpc.params?.chatId)?.spaceId;
        const paths=Object.keys(TEXTS[spaceId] ?? {}).filter(path=>path.toLowerCase().includes((rpc.params?.query ?? '').toLowerCase()));
        return reply({id:rpc.id,ok:paths.map(path=>rpc.method==='SearchFiles' ? {path,isDir:false} : {path,name:path.split('/').pop(),kind:'file'})});
      }
      if(rpc.method==='ListProjectActions') return reply({id:rpc.id,ok:{actions:[{id:'fixture-test',name:'Fixture tests',command:'printf QA_ACTION_OK',icon:'play'}]}});
      if(rpc.method==='ListGitHistory') return reply({id:rpc.id,ok:{commits:[],comparison:{base:'main',ahead:2,behind:1}}});
      if(rpc.method==='OpenTerminal') {
        commands.push(rpc);
        return reply({id:rpc.id,ok:openTerminal(chatFor(rpc.params?.chatId))});
      }
      if(rpc.method==='RunProjectAction') {
        commands.push(rpc);
        return reply({id:rpc.id,ok:{actionId:'fixture-test',actionName:'Fixture tests',
          terminal:openTerminal(chatFor(rpc.params?.chatId),'QA_ACTION_OK\r\n')}});
      }
      if(['WriteTerminal','ResizeTerminal','CloseTerminal'].includes(rpc.method)) {
        commands.push(rpc);
        const terminal=terminals.get(rpc.params?.terminalId);
        if(!terminal) return reply({id:rpc.id,err:'Unknown terminal'});
        if(rpc.method==='WriteTerminal') {
          const text=Buffer.from(rpc.params.data,'base64').toString();
          terminal.event={type:'data',seq:terminal.event.seq+1,
            data:Buffer.from(text+'\r\nQA_TERMINAL_OK\r\n$ ').toString('base64')};
        }
        if(rpc.method==='CloseTerminal') terminal.event={type:'exit',seq:terminal.event.seq+1,exitCode:0};
        reply({id:rpc.id,ok:{}});
        return reemit();
      }
      if(Object.hasOwn(snapshots,rpc.method)) {watches.set(rpc.id,{method:rpc.method,params:rpc.params});return reply({id:rpc.id,item:snapshots[rpc.method](rpc.params)});}
      if(rpc.method==='ListWorkspaceDirectory') {
        commands.push(rpc);
        const directory=rpc.params?.directory ?? '';
        return reply({id:rpc.id,ok:{directory,entries:directory ? [{path:'Sources/Client.swift',name:'Client.swift',kind:'file'}] : [{path:'Sources',name:'Sources',kind:'directory'},{path:'README.md',name:'README.md',kind:'file'}],truncated:false}});
      }
      if(rpc.method==='ReadWorkspaceFile') {
        commands.push(rpc);
        const spaceId=rpc.params?.spaceId ?? chatFor(rpc.params?.chatId)?.spaceId;
        const path=rpc.params?.path ?? '';
        const text=TEXTS[spaceId]?.[path];
        const image=WORKSPACE_IMAGES[spaceId]?.[path];
        const isImage=Buffer.isBuffer(image?.bytes);
        if(typeof text !== 'string' && !isImage) return reply({id:rpc.id,err:`file not found: ${path}`});
        if(isImage) return reply({id:rpc.id,ok:{checkoutId:CHECKOUT[spaceId],path,contentHash:image.hash,size:image.bytes.length,encoding:'binary',truncated:false}});
        return reply({id:rpc.id,ok:{checkoutId:CHECKOUT[spaceId],path,text,contentHash:fnv(text),size:Buffer.byteLength(text),encoding:'utf8',lineEnding:'lf',truncated:false}});
      }
      if(rpc.method==='WriteWorkspaceFile') {
        commands.push(rpc);
        const spaceId=chatFor(rpc.params?.chatId)?.spaceId, path=rpc.params?.path;
        const text=TEXTS[spaceId]?.[path];
        if(typeof text!=='string') return reply({id:rpc.id,err:'Unknown file'});
        if(rpc.params.expectedCheckoutId!==CHECKOUT[spaceId] || rpc.params.expectedContentHash!==fnv(text))
          return reply({id:rpc.id,ok:{status:'conflict',currentContentHash:fnv(text)}});
        TEXTS[spaceId][path]=rpc.params.text;
        reply({id:rpc.id,ok:{status:'written',file:{contentHash:fnv(rpc.params.text)}}});
        return reemit();
      }
      if(rpc.method==='FixtureChangeFile') {
        TEXTS['fixture-space']['Sources/Client.swift']='// Changed on host\n';
        return reply({id:rpc.id,ok:{}});
      }
      if(rpc.method==='ReadWorkspaceImage') {
        commands.push(rpc);
        const spaceId=rpc.params?.spaceId;
        const image=WORKSPACE_IMAGES[spaceId]?.[rpc.params?.path];
        if(!Buffer.isBuffer(image?.bytes) || rpc.params?.expectedCheckoutId !== CHECKOUT[spaceId]) return reply({id:rpc.id,err:`workspace image not found: ${rpc.params?.path}`});
        const offset=rpc.params?.offset ?? 0, size=image.bytes.length;
        const end=Math.min(offset+384*1024,size);
        return reply({id:rpc.id,ok:{checkoutId:CHECKOUT[spaceId],contentHash:image.hash,mimeType:image.mimeType,
          data:image.bytes.subarray(offset,end).toString('base64'),nextOffset:end,size,done:end>=size}});
      }
      if(rpc.method==='GetCheckoutDiff') {
        commands.push(rpc);
        return reply({id:rpc.id,ok:checkoutDiff(chatFor(rpc.params?.chatId))});
      }
      if(rpc.method==='DiscardWorkingTree') {
        commands.push(rpc);
        if(rpc.params.expectedChecksum!=='fixture-dirty') return reply({id:rpc.id,err:'Stale diff'});
        cleanCheckouts.add(rpc.params.checkoutId);
        reply({id:rpc.id,ok:{}});
        return reemit();
      }
      if(rpc.method==='ReadAttachmentChunk') {
        commands.push(rpc);
        const file=attachments.get(rpc.params?.path);
        if(!file) return reply({id:rpc.id,err:`attachment not found: ${rpc.params?.path}`});
        const offset=rpc.params?.offset ?? 0, size=file.bytes.length;
        const end=Math.min(offset+45_000,size);
        return reply({id:rpc.id,ok:{name:file.name,mimeType:file.mimeType,
          data:file.bytes.subarray(offset,end).toString('base64'),nextOffset:end,done:end>=size}});
      }
      if(rpc.method==='UploadChunk') {
        commands.push(rpc);
        const staged=uploads.get(rpc.params?.uploadId) ?? new Map();
        staged.set(rpc.params?.seq ?? staged.size,rpc.params?.data ?? '');
        uploads.set(rpc.params.uploadId,staged);
        return reply({id:rpc.id,ok:{ok:true}});
      }
      if(rpc.method==='UploadCommit') {
        commands.push(rpc);
        const staged=uploads.get(rpc.params?.uploadId);
        if(!staged) return reply({id:rpc.id,err:'Unknown or expired upload'});
        let encoded='';
        for(const key of [...staged.keys()].sort((a,b)=>a-b)) encoded+=staged.get(key);
        const bytes=Buffer.from(encoded,'base64');
        const name=sanitize(rpc.params?.fileName ?? 'image');
        const path=`/tmp/noches-fixture/.noches/uploads/${rpc.params.uploadId.slice(0,8)}-${name}`;
        attachments.set(path,{bytes,name,mimeType:mimeFor(name)});
        uploads.delete(rpc.params.uploadId);
        return reply({id:rpc.id,ok:{path}});
      }
      if(rpc.method==='BigReply') return reply({id:rpc.id,ok:{text:'🌙'.repeat(15000)}});
      if(rpc.method==='FixtureCommands') return reply({id:rpc.id,ok:commands});
      if(rpc.method==='QueueCommand') {
        commands.push(rpc);
        const chatId=rpc.params?.chatId, command=rpc.params?.command ?? {};
        if(command.kind==='run') appendDispatch(chatId,command.request?.prompt ?? '');
        if(command.kind==='respondInput' && command.requestId==='approval-request') {
          approvalResolved=true;
          docs.set('fixture-approval',[approvalEntry()]);
        }
        if(command.kind==='interrupt' && chatId==='fixture-working') working=false;
        reply({id:rpc.id,ok:{commandId:`fixture-${commands.length}`}});
        reemit();
        return;
      }
      if(rpc.method==='QueueMessage') {
        commands.push(rpc);
        const item={chatId:rpc.params?.chatId,id:randomUUID(),text:rpc.params?.text ?? '',
          issuedBy:profile.deviceId,issuedAt:Date.now()};
        if(rpc.params?.attachments?.length) item.attachments=rpc.params.attachments;
        if(rpc.params?.holdForTurnEnd) item.holdForTurnEnd=true;
        queued.push(item);
        reply({id:rpc.id,ok:{id:item.id}});
        reemit();
        return;
      }
      if(['UpdateQueuedMessage','MoveQueuedMessage','RemoveQueuedMessage','SendQueuedMessageNow','SteerQueuedMessageNow'].includes(rpc.method)) {
        commands.push(rpc);
        const index=queued.findIndex(item => item.chatId===rpc.params?.chatId && item.id===rpc.params?.id);
        if(rpc.method==='UpdateQueuedMessage') {
          if(index<0) reply({id:rpc.id,ok:{changed:false}});
          else if((rpc.params.text ?? '').trim()==='') {queued.splice(index,1);reply({id:rpc.id,ok:{changed:true}});}
          else {queued[index].text=rpc.params.text;queued[index].editedAt=Date.now();reply({id:rpc.id,ok:{changed:true}});}
        } else if(rpc.method==='MoveQueuedMessage') {
          if(index<0 || rpc.params.toIndex===index) reply({id:rpc.id,ok:{changed:false}});
          else {const [item]=queued.splice(index,1);queued.splice(Math.min(rpc.params.toIndex,queued.length),0,item);reply({id:rpc.id,ok:{changed:true}});}
        } else if(rpc.method==='RemoveQueuedMessage') {
          if(index<0) reply({id:rpc.id,ok:{removed:false}});
          else {queued.splice(index,1);reply({id:rpc.id,ok:{removed:true}});}
        } else {
          if(index<0) reply({id:rpc.id,ok:{sent:false}});
          else {const [item]=queued.splice(index,1);appendDispatch(item.chatId,item.text);reply({id:rpc.id,ok:{sent:true}});}
        }
        reemit();
        return;
      }
      if(rpc.method==='BeginQueuedMessageEdit') {
        commands.push(rpc);
        const item=queued.find(entry => entry.chatId===rpc.params?.chatId && entry.id===rpc.params?.id);
        if(!item) return reply({id:rpc.id,ok:{outcome:'missing'}});
        if(item.deliveryGate) return reply({id:rpc.id,ok:{outcome:'locked',ownerDeviceId:item.deliveryGate.ownerDeviceId,expiresAtMs:item.deliveryGate.expiresAtMs}});
        const leaseId=randomUUID();
        const gate={kind:'editing',leaseId,ownerDeviceId:rpc.params.editorDeviceId ?? 'fixture',
          ownerInstanceId:rpc.params.editorInstanceId ?? 'fixture',acquiredAtMs:Date.now(),expiresAtMs:Date.now()+30_000,baseTextHash:fnv(item.text)};
        item.deliveryGate=gate;
        reply({id:rpc.id,ok:{outcome:'acquired',leaseId,text:item.text,attachments:item.attachments ?? [],
          baseTextHash:gate.baseTextHash,expiresAtMs:gate.expiresAtMs}});
        reemit();
        return;
      }
      if(rpc.method==='RenewQueuedMessageEdit') {
        commands.push(rpc);
        const item=queued.find(entry => entry.chatId===rpc.params?.chatId && entry.id===rpc.params?.id);
        if(!item || item.deliveryGate?.leaseId!==rpc.params?.leaseId) return reply({id:rpc.id,ok:{outcome:'lost'}});
        item.deliveryGate.expiresAtMs=Date.now()+30_000;
        reply({id:rpc.id,ok:{outcome:'renewed',expiresAtMs:item.deliveryGate.expiresAtMs}});
        return;
      }
      if(rpc.method==='FinishQueuedMessageEdit') {
        commands.push(rpc);
        const index=queued.findIndex(entry => entry.chatId===rpc.params?.chatId && entry.id===rpc.params?.id);
        if(index<0) {reply({id:rpc.id,ok:{outcome:'missing'}});return reemit();}
        const item=queued[index];
        if(item.deliveryGate?.leaseId!==rpc.params?.leaseId) {reply({id:rpc.id,ok:{outcome:'lost'}});return reemit();}
        if(rpc.params.action==='commit') {
          if(rpc.params.expectedTextHash && rpc.params.expectedTextHash!==item.deliveryGate.baseTextHash) {
            reply({id:rpc.id,ok:{outcome:'conflict',currentText:item.text}});return reemit();
          }
          item.text=rpc.params.text ?? item.text;
          item.editedAt=Date.now();
          if(rpc.params.attachments) item.attachments=rpc.params.attachments;
          delete item.deliveryGate;
          reply({id:rpc.id,ok:{outcome:'committed'}});
        } else if(rpc.params.action==='discard') {
          queued.splice(index,1);
          reply({id:rpc.id,ok:{outcome:'discarded'}});
        } else {
          delete item.deliveryGate;
          reply({id:rpc.id,ok:{outcome:rpc.params.action==='cancel'?'cancelled':'released'}});
        }
        reemit();
        return;
      }
      if(rpc.method==='Mutate') {
        commands.push(rpc);
        const op=rpc.params?.op;
        if(op==='createSpace') {
          const space={id:rpc.params.spaceId,deviceId:rpc.params.deviceId,path:rpc.params.path,name:rpc.params.name,
            gitDetected:rpc.params.gitDetected ?? false,createdAt:now(),checkoutId:`checkout-${rpc.params.spaceId}`};
          SPACES.push(space);CHECKOUT[space.id]=space.checkoutId;TEXTS[space.id]={'README.md':'# New fixture project\n'};
          reply({id:rpc.id,ok:{}});return reemit();
        }
        if(op==='renameSpace' || op==='deleteSpace') {
          const index=SPACES.findIndex(space=>space.id===rpc.params.spaceId);
          if(index<0) return reply({id:rpc.id,err:'Unknown project'});
          if(op==='renameSpace') SPACES[index].name=rpc.params.name;
          else {SPACES.splice(index,1);chats=chats.filter(chat=>chat.spaceId!==rpc.params.spaceId);}
          reply({id:rpc.id,ok:{}});return reemit();
        }
        if(op==='createChat') {
          const space=SPACES.find(entry => entry.id===rpc.params.spaceId);
          chats.push(chatRow({id:rpc.params.chatId,title:'New mobile session',spaceId:rpc.params.spaceId ?? null,
            cwd:rpc.params.cwd ?? space?.path ?? '~',branch:rpc.params.branch ?? null,config:rpc.params.config ?? null,
            checkoutId:space ? CHECKOUT[space.id] : null,createdAt:now()}));
          docs.set(rpc.params.chatId,[]);
          reply({id:rpc.id,ok:{}});
          return reemit();
        }
        const target=chatFor(rpc.params?.chatId);
        if(!target) return reply({id:rpc.id,err:'Unknown chat'});
        if(op==='renameChat') target.title=rpc.params.title;
        else if(op==='setChatArchived') target.archived=rpc.params.archived;
        else if(op==='markChatSeen') target.lastSeenAt=new Date(rpc.params.at ?? Date.now()).toISOString();
        else if(op==='setChatConfig') target.config=rpc.params.config;
        else return reply({id:rpc.id,err:`Unknown fixture op: ${op}`});
        reply({id:rpc.id,ok:{}});
        reemit();
        return;
      }
      reply({id:rpc.id,err:`Unknown fixture method: ${rpc.method}`});
    } catch {socket.close();}
  });
});
// Engines heartbeat live sessions well inside the 45s staleness window.
setInterval(() => { for (const refresh of [...connections]) refresh(); }, 15_000);
host.listen(port,'127.0.0.1',()=>console.log(`Fixture listening on 127.0.0.1:${port}\nnoches-connect:${Buffer.from(JSON.stringify(profile)).toString('base64url')}`));
