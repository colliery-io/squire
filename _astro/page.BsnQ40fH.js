const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["_astro/mermaid.core.Br53GdPF.js","_astro/src.wpmQHQd0.js","_astro/chunk-Y2CYZVJY.DsF7k-Jl.js","_astro/preload-helper.CsKYNmFz.js","_astro/chunk-DU6HZSFF.C5p_Bun3.js","_astro/chunk-75Z2AOVW.DlUE9eEf.js","_astro/dist.DUKuLFbX.js","_astro/chunk-PWAF6VOD.CTaqd-Hq.js","_astro/chunk-GMAD6QVW.Dp6LZWlJ.js","_astro/chunk-P2QGCYS3.D3UDtxrS.js","_astro/chunk-4HAMMTFA.BddKgRpU.js","_astro/rough.esm.Dy-Kn_BL.js","_astro/chunk-GVQU2GXP.sbnJNIxO.js","_astro/chunk-OSK3NFVY.CmeHBoM2.js","_astro/line.WUcDlpNA.js","_astro/path.fybaL0A-.js","_astro/array.BifhSqXX.js","_astro/graphlib.DS17s2tU.js","_astro/chunk-L3NEJ4N5.buUPvR_1.js"])))=>i.map(i=>d[i]);
import{t as e}from"./preload-helper.CsKYNmFz.js";var t={"#08427b":`var(--violet-bg)`,"#073b6f":`var(--violet)`,"#1168bd":`var(--ice-bg)`,"#3c7fc0":`var(--ice)`,"#438dd5":`var(--teal-bg)`,"#85bbf0":`var(--teal-bg)`,"#78a8d8":`var(--teal)`,"#686868":`var(--muted-bg)`,"#999999":`var(--muted-bg)`,"#b3b3b3":`var(--muted-bg)`,"#cccccc":`var(--muted-bg)`,"#8a8a8a":`var(--border-control)`,"#a6a6a6":`var(--border-control)`,"#bfbfbf":`var(--border-control)`,"#ffffff":`var(--fg)`,"#444444":`var(--fg-2)`,"#bbb":`var(--border)`},n=/#[0-9a-f]{3,8}\b/gi;function r(e){return e.replace(n,e=>t[e.toLowerCase()]??e)}function i(e){for(let n of e.querySelectorAll(`[style], [fill], [stroke]`)){let e=n.getAttribute(`style`);if(e){let t=r(e);t!==e&&n.setAttribute(`style`,t)}for(let e of[`fill`,`stroke`]){let r=n.getAttribute(e);r&&t[r.toLowerCase()]&&n.setAttribute(e,t[r.toLowerCase()])}}}function a(e){for(let t of e.querySelectorAll(`svg[aria-roledescription="c4"]`))i(t)}if(typeof document<`u`){let e=()=>{a(document),new MutationObserver(e=>{for(let t of e)for(let e of t.addedNodes)e.nodeType===1&&(e.matches?.(`svg[aria-roledescription="c4"]`)?i(e):a(e))}).observe(document.body,{childList:!0,subtree:!0})};document.readyState===`loading`?document.addEventListener(`DOMContentLoaded`,e):e()}var o={},s=new Set,c=new WeakSet,l=!0,u=void 0,d=!1;function f(e){d||(d=!0,l??=e?.prefetchAll??!1,u??=e?.defaultStrategy??`hover`,p(),m(),h(),_())}function p(){for(let e of[`touchstart`,`mousedown`])document.addEventListener(e,e=>{let t=e.target.closest(`a`);b(t,`tap`)&&v(t.href,{ignoreSlowConnection:!0})},{passive:!0})}function m(){let e;document.body.addEventListener(`focusin`,e=>{let n=e.target.closest(`a`);b(n,`hover`)&&t(n.href)},{passive:!0}),document.body.addEventListener(`focusout`,n,{passive:!0}),S(()=>{for(let e of document.getElementsByTagName(`a`))c.has(e)||b(e,`hover`)&&(c.add(e),e.addEventListener(`mouseenter`,e=>t(e.currentTarget.href),{passive:!0}),e.addEventListener(`mouseleave`,n,{passive:!0}))});function t(t){e&&clearTimeout(e),e=setTimeout(()=>{v(t)},80)}function n(){e&&=(clearTimeout(e),0)}}function h(){let e;S(()=>{for(let t of document.getElementsByTagName(`a`))c.has(t)||b(t,`viewport`)&&(c.add(t),e??=g(),e.observe(t))})}function g(){let e=new WeakMap;return new IntersectionObserver((t,n)=>{for(let r of t){let t=r.target,i=e.get(t);r.isIntersecting?(i&&clearTimeout(i),e.set(t,setTimeout(()=>{n.unobserve(t),e.delete(t),v(t.href)},300))):i&&(clearTimeout(i),e.delete(t))}})}function _(){S(()=>{for(let e of document.getElementsByTagName(`a`))b(e,`load`)&&v(e.href)})}function v(e,t){e=e.replace(/#.*/,``);let n=t?.ignoreSlowConnection??!1;if(y(e,n)){if(s.add(e),document.createElement(`link`).relList?.supports?.(`prefetch`)){let t=document.createElement(`link`);t.rel=`prefetch`,t.setAttribute(`href`,e),document.head.append(t)}else{let t=new Headers;for(let[e,n]of Object.entries(o))t.set(e,n);fetch(e,{priority:`low`,headers:t}).catch(()=>{})}}}function y(e,t){if(!navigator.onLine||!t&&x())return!1;try{let t=new URL(e,location.href);return location.origin===t.origin&&(location.pathname!==t.pathname||location.search!==t.search)&&!s.has(e)}catch{}return!1}function b(e,t){if(e?.tagName!==`A`)return!1;let n=e.dataset.astroPrefetch;return n===`false`?!1:t===`tap`&&(n!=null||l)&&x()?!0:n==null&&l||n===``?t===u:n===t}function x(){if(`connection`in navigator){let e=navigator.connection;return e.saveData||/2g/.test(e.effectiveType)}return!1}function S(e){e();let t=!1;document.addEventListener(`astro:page-load`,()=>{t?e():t=!0}),new MutationObserver(t=>{for(let n of t)for(let t of n.addedNodes)if(t instanceof Element&&(t.tagName===`A`||t.querySelector?.(`a`))){e();return}}).observe(document.body,{childList:!0,subtree:!0})}var C=(...e)=>console.log(`[astro-mermaid]`,...e),w=(...e)=>console.error(`[astro-mermaid]`,...e),T=()=>document.querySelectorAll(`pre.mermaid`).length>0,E=null;async function D(){return E||(C(`Loading mermaid.js...`),E=e(async()=>{let{default:e}=await import(`./mermaid.core.Br53GdPF.js`);return{default:e}},__vite__mapDeps([0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18])).then(async({default:e})=>{let t=[];if(t&&t.length>0){C(`Registering`,t.length,`icon packs`);let n=t.map(e=>e.icons?{name:e.name,icons:e.icons}:{name:e.name,loader:()=>fetch(e.url).then(e=>e.json())});await e.registerIconPacks(n)}return e}).catch(e=>{throw w(`Failed to load mermaid:`,e),E=null,e}),E)}var O={startOnLoad:!1,theme:`dark`},k={light:`default`,dark:`dark`};async function A(){C(`Initializing mermaid diagrams...`);let e=document.querySelectorAll(`pre.mermaid`);if(C(`Found`,e.length,`mermaid diagrams`),e.length===0)return;let t=await D(),n=O.theme;{let e=document.documentElement.getAttribute(`data-theme`),t=document.body.getAttribute(`data-theme`);n=k[e||t]||O.theme,C(`Using theme:`,n,`from`,e?`html`:`body`)}t.initialize({...O,theme:n,gitGraph:{mainBranchName:`main`,showCommitLabel:!0,showBranches:!0,rotateCommitLabel:!0}});for(let n of e){if(n.hasAttribute(`data-processed`))continue;n.hasAttribute(`data-diagram`)||n.setAttribute(`data-diagram`,n.textContent||``);let e=n.getAttribute(`data-diagram`)||``,r=`mermaid-`+Math.random().toString(36).slice(2,11);C(`Rendering diagram:`,r);try{let i=document.getElementById(r);i&&i.remove();let{svg:a}=await t.render(r,e);n.innerHTML=a,n.setAttribute(`data-processed`,`true`),C(`Successfully rendered diagram:`,r)}catch(e){w(`Mermaid rendering error for diagram:`,r,e);let t=document.createElement(`div`);t.style.cssText=`color: red; padding: 1rem; border: 1px solid red; border-radius: 0.5rem;`;let i=document.createElement(`strong`);i.textContent=`Error rendering diagram:`;let a=document.createElement(`span`);a.textContent=` `+(e.message||`Unknown error`),t.appendChild(i),t.appendChild(a),n.textContent=``,n.appendChild(t),n.setAttribute(`data-processed`,`true`)}}}T()?(C(`Mermaid diagrams detected on initial load`),A()):C(`No mermaid diagrams found on initial load`);{let e=new MutationObserver(e=>{for(let t of e)t.type===`attributes`&&t.attributeName===`data-theme`&&(document.querySelectorAll(`pre.mermaid[data-processed]`).forEach(e=>{e.removeAttribute(`data-processed`)}),A())});e.observe(document.documentElement,{attributes:!0,attributeFilter:[`data-theme`]}),e.observe(document.body,{attributes:!0,attributeFilter:[`data-theme`]})}document.addEventListener(`astro:after-swap`,()=>{C(`View transition detected`),T()&&A()});var j=document.createElement(`style`);j.textContent=`
            /* Prevent layout shifts by setting minimum height */
            pre.mermaid {
              display: flex;
              justify-content: center;
              align-items: center;
              margin: 2rem 0;
              padding: 1rem;
              background-color: transparent;
              border: none;
              overflow: auto;
              min-height: 200px; /* Prevent layout shift */
              position: relative;
            }
            
            /* Loading state with skeleton loader */
            pre.mermaid:not([data-processed]) {
              background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
              background-size: 200% 100%;
              animation: shimmer 1.5s infinite;
            }
            
            /* Dark mode skeleton loader */
            [data-theme="dark"] pre.mermaid:not([data-processed]) {
              background: linear-gradient(90deg, #2a2a2a 25%, #3a3a3a 50%, #2a2a2a 75%);
              background-size: 200% 100%;
            }
            
            @keyframes shimmer {
              0% {
                background-position: -200% 0;
              }
              100% {
                background-position: 200% 0;
              }
            }
            
            /* Show processed diagrams with smooth transition */
            pre.mermaid[data-processed] {
              animation: none;
              background: transparent;
              min-height: auto; /* Allow natural height after render */
            }
            
            /* Ensure responsive sizing for mermaid SVGs */
            pre.mermaid svg {
              max-width: 100%;
              height: auto;
            }
            
            /* Optional: Add subtle background for better visibility */
            @media (prefers-color-scheme: dark) {
              pre.mermaid[data-processed] {
                background-color: rgba(255, 255, 255, 0.02);
                border-radius: 0.5rem;
              }
            }
            
            @media (prefers-color-scheme: light) {
              pre.mermaid[data-processed] {
                background-color: rgba(0, 0, 0, 0.02);
                border-radius: 0.5rem;
              }
            }
            
            /* Respect user's color scheme preference */
            [data-theme="dark"] pre.mermaid[data-processed] {
              background-color: rgba(255, 255, 255, 0.02);
              border-radius: 0.5rem;
            }
            
            [data-theme="light"] pre.mermaid[data-processed] {
              background-color: rgba(0, 0, 0, 0.02);
              border-radius: 0.5rem;
            }
          `,document.head.appendChild(j),f();