import{_ as a,o as n,c as e,ag as i}from"./chunks/framework.DXGyWiRo.js";const k=JSON.parse('{"title":"Frontend Architecture","description":"","frontmatter":{},"headers":[],"relativePath":"architecture/frontend.md","filePath":"architecture/frontend.md","lastUpdated":1775719799000}'),t={name:"architecture/frontend.md"};function l(p,s,r,o,c,h){return n(),e("div",null,[...s[0]||(s[0]=[i(`<h1 id="frontend-architecture" tabindex="-1">Frontend Architecture <a class="header-anchor" href="#frontend-architecture" aria-label="Permalink to &quot;Frontend Architecture&quot;">​</a></h1><p>Deep dive into the RustChat frontend architecture.</p><h2 id="overview" tabindex="-1">Overview <a class="header-anchor" href="#overview" aria-label="Permalink to &quot;Overview&quot;">​</a></h2><p>The frontend is a Single Page Application (SPA) built with:</p><ul><li><strong>Framework:</strong> Vue 3.5</li><li><strong>Language:</strong> TypeScript 5.9+</li><li><strong>State:</strong> Pinia 3+</li><li><strong>Build:</strong> Vite 7+</li><li><strong>Styling:</strong> Tailwind CSS 4+</li></ul><h2 id="directory-structure" tabindex="-1">Directory Structure <a class="header-anchor" href="#directory-structure" aria-label="Permalink to &quot;Directory Structure&quot;">​</a></h2><div class="language- vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang"></span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span>frontend/src/</span></span>
<span class="line"><span>├── api/              # API client functions</span></span>
<span class="line"><span>│   ├── http/        # HTTP client (Fetch-based)</span></span>
<span class="line"><span>│   ├── client.ts    # Native API client</span></span>
<span class="line"><span>│   └── calls.ts     # Calls API client</span></span>
<span class="line"><span>├── components/       # Vue components</span></span>
<span class="line"><span>├── composables/      # Vue composables</span></span>
<span class="line"><span>├── core/            # Shared primitives</span></span>
<span class="line"><span>│   ├── entities/    # Domain entities</span></span>
<span class="line"><span>│   ├── errors/      # Error types</span></span>
<span class="line"><span>│   └── websocket/   # WebSocket infrastructure</span></span>
<span class="line"><span>├── features/        # Domain feature modules</span></span>
<span class="line"><span>│   ├── auth/</span></span>
<span class="line"><span>│   ├── calls/</span></span>
<span class="line"><span>│   ├── channels/</span></span>
<span class="line"><span>│   ├── messages/</span></span>
<span class="line"><span>│   └── ...</span></span>
<span class="line"><span>├── router/          # Vue Router configuration</span></span>
<span class="line"><span>├── stores/          # Legacy Pinia stores</span></span>
<span class="line"><span>└── views/           # Page-level components</span></span></code></pre></div><h2 id="feature-module-pattern" tabindex="-1">Feature Module Pattern <a class="header-anchor" href="#feature-module-pattern" aria-label="Permalink to &quot;Feature Module Pattern&quot;">​</a></h2><p>Each feature follows a consistent structure:</p><div class="language- vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang"></span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span>features/[feature]/</span></span>
<span class="line"><span>├── repositories/    # Data access (API calls)</span></span>
<span class="line"><span>├── services/        # Business logic</span></span>
<span class="line"><span>├── stores/          # Pinia state</span></span>
<span class="line"><span>├── handlers/        # WebSocket event handlers</span></span>
<span class="line"><span>├── components/      # Feature-specific components</span></span>
<span class="line"><span>└── index.ts         # Public API</span></span></code></pre></div><h2 id="state-management" tabindex="-1">State Management <a class="header-anchor" href="#state-management" aria-label="Permalink to &quot;State Management&quot;">​</a></h2><h3 id="legacy-stores-deprecated" tabindex="-1">Legacy Stores (deprecated) <a class="header-anchor" href="#legacy-stores-deprecated" aria-label="Permalink to &quot;Legacy Stores (deprecated)&quot;">​</a></h3><div class="language- vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang"></span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span>stores/</span></span>
<span class="line"><span>├── auth.ts</span></span>
<span class="line"><span>├── channels.ts</span></span>
<span class="line"><span>├── messages.ts</span></span>
<span class="line"><span>└── ...</span></span></code></pre></div><h3 id="modern-feature-stores-recommended" tabindex="-1">Modern Feature Stores (recommended) <a class="header-anchor" href="#modern-feature-stores-recommended" aria-label="Permalink to &quot;Modern Feature Stores (recommended)&quot;">​</a></h3><div class="language-typescript vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang">typescript</span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span style="--shiki-light:#6A737D;--shiki-dark:#6A737D;">// features/channels/stores/channelStore.ts</span></span>
<span class="line"><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;">export</span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;"> const</span><span style="--shiki-light:#005CC5;--shiki-dark:#79B8FF;"> useChannelStore</span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;"> =</span><span style="--shiki-light:#6F42C1;--shiki-dark:#B392F0;"> defineStore</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">(</span><span style="--shiki-light:#032F62;--shiki-dark:#9ECBFF;">&#39;channels&#39;</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">, () </span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;">=&gt;</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;"> {</span></span>
<span class="line"><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;">  const</span><span style="--shiki-light:#005CC5;--shiki-dark:#79B8FF;"> channels</span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;"> =</span><span style="--shiki-light:#6F42C1;--shiki-dark:#B392F0;"> ref</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">&lt;</span><span style="--shiki-light:#6F42C1;--shiki-dark:#B392F0;">Channel</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">[]&gt;([])</span></span>
<span class="line"><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">  </span></span>
<span class="line"><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;">  async</span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;"> function</span><span style="--shiki-light:#6F42C1;--shiki-dark:#B392F0;"> fetchChannels</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">() {</span></span>
<span class="line"><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">    channels.value </span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;">=</span><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;"> await</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;"> channelRepository.</span><span style="--shiki-light:#6F42C1;--shiki-dark:#B392F0;">getChannels</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">()</span></span>
<span class="line"><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">  }</span></span>
<span class="line"><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">  </span></span>
<span class="line"><span style="--shiki-light:#D73A49;--shiki-dark:#F97583;">  return</span><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;"> { channels, fetchChannels }</span></span>
<span class="line"><span style="--shiki-light:#24292E;--shiki-dark:#E1E4E8;">})</span></span></code></pre></div><h2 id="data-flow" tabindex="-1">Data Flow <a class="header-anchor" href="#data-flow" aria-label="Permalink to &quot;Data Flow&quot;">​</a></h2><div class="language- vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang"></span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span>User Action</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Component</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Service (business logic)</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Repository (API call)</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>HTTP Client</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Backend API</span></span></code></pre></div><p>WebSocket events flow in reverse:</p><div class="language- vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang"></span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span>WebSocket Message</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Handler</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Service</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Store Update</span></span>
<span class="line"><span>  ↓</span></span>
<span class="line"><span>Component Re-render</span></span></code></pre></div><h2 id="http-client" tabindex="-1">HTTP Client <a class="header-anchor" href="#http-client" aria-label="Permalink to &quot;HTTP Client&quot;">​</a></h2><p>Custom Fetch-based HTTP client with:</p><ul><li>Request/response interceptors</li><li>Auth token injection</li><li>ID normalization (Mattermost ↔ UUID)</li><li>Error handling</li><li>Upload progress support</li></ul><h2 id="websocket-integration" tabindex="-1">WebSocket Integration <a class="header-anchor" href="#websocket-integration" aria-label="Permalink to &quot;WebSocket Integration&quot;">​</a></h2><p>Native WebSocket client for real-time updates:</p><ul><li>Auto-reconnect</li><li>Heartbeat/ping</li><li>Event subscription management</li></ul><h2 id="component-guidelines" tabindex="-1">Component Guidelines <a class="header-anchor" href="#component-guidelines" aria-label="Permalink to &quot;Component Guidelines&quot;">​</a></h2><ul><li>Use <code>&lt;script setup&gt;</code> syntax</li><li>Composition API preferred</li><li>Props/Emits for component interface</li><li>Composables for reusable logic</li></ul><h2 id="testing" tabindex="-1">Testing <a class="header-anchor" href="#testing" aria-label="Permalink to &quot;Testing&quot;">​</a></h2><ul><li><strong>Unit:</strong> Vitest for composables/services</li><li><strong>E2E:</strong> Playwright for user flows</li><li><strong>Contract:</strong> HTTP client behavior tests</li></ul><hr><p><em>See also: <a href="./backend">Backend Architecture</a></em></p>`,31)])])}const u=a(t,[["render",l]]);export{k as __pageData,u as default};
