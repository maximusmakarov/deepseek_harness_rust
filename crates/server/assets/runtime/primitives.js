/* dshr design-system compatibility shim, ported from the reference implementation.
 *
 * Source of truth: deepseek-ai/deepseek-harness @ 39163dd,
 * packages/client/ui-primitives/src (MIT). Components below follow the reference
 * DOM structure, props, ARIA and behaviour; icons are generated from the
 * reference artwork (artwork.ts) and inlined instead of using the document
 * sprite. Styling uses the reference `--dsw-alias-*` tokens with fallbacks.
 *
 * Ported in full: Tooltip (incl. tooltip-placement.ts, ShortcutKeys,
 * input-modality.ts) and the Menu core (rows/selection/aria/keyboard walk,
 * one-level submenu, pointer grace, portal placement). Still partial:
 * Menu's full submenu machinery (useMenuSubmenu single-panel back-page,
 * MenuGroup/sticky groups) and MarkdownText/JsonBlock (their upstream
 * implementations depend on markdown/shiki/katex packages).
 */
(function () {
  var React = window.React;
  var ReactDOM = window.ReactDOM;
  if (!React) {
    throw new Error("dshr ui primitives: React is missing from the host runtime");
  }
  var h = React.createElement;

  var styleId = "dshr-ui-primitives";
  if (!document.querySelector('style[data-dshr-primitives="' + styleId + '"]')) {
    var style = document.createElement("style");
    style.setAttribute("data-dshr-primitives", styleId);
    style.textContent = [
      ":root{--dshr-radius-md:var(--dsw-radius-md,8px);--dshr-radius-sm:var(--dsw-radius-sm,6px);--dshr-hairline:var(--dsh-hairline-width,.5px)}",
      ".dshr-button{box-sizing:border-box;display:inline-flex;align-items:center;justify-content:center;gap:4px;border:none;border-radius:var(--dshr-radius-md);cursor:pointer;font:inherit;font-size:14px;line-height:22px;color:var(--dsw-alias-label-primary,inherit);background:transparent;padding:0 14px}",
      ".dshr-button:disabled{cursor:not-allowed;opacity:.4}",
      ".dshr-button-md{height:36px}",
      ".dshr-button-sm{height:28px;font-size:12px;line-height:18px;padding:0 10px;border-radius:var(--dshr-radius-sm)}",
      ".dshr-button-primary{background:var(--dsw-alias-button-primary-fill,var(--dsw-alias-brand-primary,#4da3ff));color:var(--dsw-alias-label-primary-foreground,#fff)}",
      ".dshr-button-primary:hover:not(:disabled){background:var(--dsw-alias-button-primary-hover,var(--dsw-alias-brand-primary,#4da3ff))}",
      ".dshr-button-ghost:hover:not(:disabled){background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.12))}",
      ".dshr-button-ghost:active:not(:disabled){background:var(--dsw-alias-interactive-bg-active,rgba(127,127,127,.16))}",
      ".dshr-button-outline{border:0.5px solid transparent;background:transparent;box-shadow:inset 0 0 0 var(--dshr-hairline) var(--dsh-button-outline-color,var(--dsw-alias-border-l3,#454545))}",
      ".dshr-button-outline:hover:not(:disabled){background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.12))}",
      ".dshr-button-toolbar{background:var(--dsw-alias-button-tool-bar-fill,rgba(127,127,127,.10))}",
      ".dshr-button-toolbar:hover:not(:disabled){background:var(--dsw-alias-button-tool-bar-hover,rgba(127,127,127,.16))}",
      ".dshr-button-icon{display:inline-flex;width:16px;height:16px;align-items:center;justify-content:center}",
      ".dshr-input{display:inline-flex;align-items:center;gap:6px;height:32px;padding:0 8px;border:0.5px solid var(--dsw-alias-border-l4,#3a3a3a);border-radius:var(--dshr-radius-md);background:var(--dsw-alias-bg-layer-1,transparent)}",
      ".dshr-input:focus-within{border-color:var(--dsw-alias-state-business-primary,#4da3ff)}",
      ".dshr-input-icon{display:inline-flex;width:16px;height:16px;align-items:center;justify-content:center;color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-input input{flex:1;min-width:0;font:inherit;font-size:14px;line-height:20px;color:var(--dsw-alias-label-primary,inherit);background:transparent;border:0;outline:0}",
      ".dshr-switch{box-sizing:border-box;position:relative;flex:0 0 auto;width:36px;height:20px;padding:2px;border:0;border-radius:999px;background:var(--dsw-alias-border-l3,#454545);cursor:pointer}",
      ".dshr-switch[aria-checked='true']{background:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-switch:disabled{cursor:default;opacity:.5}",
      ".dshr-switch:focus-visible{outline:2px solid var(--dsw-focus-ring-color,var(--dsw-alias-state-business-primary,#4da3ff));outline-offset:2px}",
      ".dshr-switch-thumb{display:block;width:16px;height:16px;border-radius:50%;background:var(--dsw-alias-label-primary-foreground,#fff);transition:transform 120ms ease}",
      ".dshr-switch[aria-checked='false'] .dshr-switch-thumb{background:var(--dsw-alias-switch-thumb,#f2f2f2)}",
      ".dshr-switch[aria-checked='true'] .dshr-switch-thumb{transform:translateX(16px)}",
      ".dshr-checkbox{display:inline-flex;align-items:center;gap:6px;font-size:14px;line-height:20px;color:var(--dsw-alias-label-primary,inherit);cursor:pointer}",
      ".dshr-checkbox input{flex:0 0 auto;width:16px;height:16px;margin:0;accent-color:var(--dsw-alias-brand-primary,#4da3ff);cursor:inherit}",
      ".dshr-checkbox:has(input:disabled){cursor:default;opacity:.5}",
      ".dshr-tag{display:inline-flex;align-items:center;border-radius:999px;padding:1px 8px;font-size:11px;line-height:17px;font-weight:500;white-space:nowrap}",
      ".dshr-tag[data-tone='outline']{border:0.5px solid var(--dsw-alias-border-l4,#3a3a3a);color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-tag[data-tone='solid']{background:var(--dsw-alias-label-primary,#e6e6e6);color:var(--dsw-alias-bg-layer-3,#1f1f1f)}",
      ".dshr-tag[data-tone='neutral']{background:var(--dsw-alias-bg-module-platform,rgba(127,127,127,.12));color:var(--dsw-alias-label-secondary,#bbb)}",
      ".dshr-tag[data-tone='quiet']{color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-tag[data-tone='success']{background:color-mix(in srgb,var(--dsw-alias-state-success-primary,#22c55e) 10%,transparent);color:var(--dsw-alias-state-success-primary,#22c55e)}",
      ".dshr-tag[data-tone='info']{background:color-mix(in srgb,var(--dsw-alias-state-business-primary,#4da3ff) 10%,transparent);color:var(--dsw-alias-state-business-primary,#4da3ff)}",
      ".dshr-tag[data-tone='warning']{background:color-mix(in srgb,var(--dsw-alias-state-warn-primary,#f59e0b) 12%,transparent);color:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-tag[data-tone='danger']{background:color-mix(in srgb,var(--dsw-alias-state-error-primary,#ef4444) 10%,transparent);color:var(--dsw-alias-state-error-primary,#ef4444)}",
      ".dshr-pill{display:inline-flex;align-items:center;gap:4px;height:24px;padding:0 8px;border:none;border-radius:999px;font:inherit;font-size:12px;line-height:18px;color:var(--dsw-alias-label-secondary,#bbb);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12))}",
      ".dshr-pill-interactive{cursor:pointer}",
      ".dshr-pill-interactive:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.16))}",
      ".dshr-pill-active{color:var(--dsw-alias-label-primary,#e6e6e6);background:var(--dsw-alias-button-ghost-active-fill,rgba(127,127,127,.16));box-shadow:inset 0 0 0 1px var(--dsw-alias-button-ghost-active-border,transparent)}",
      ".dshr-segmented{position:relative;display:grid;grid-template-columns:repeat(var(--dsh-segment-count,2),minmax(0,1fr));align-items:center;padding:2px;border-radius:var(--dshr-radius-md);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));gap:2px}",
      ".dshr-segmented-indicator{position:absolute;top:2px;bottom:2px;left:2px;width:calc((100% - 4px)/var(--dsh-segment-count,2));border-radius:var(--dshr-radius-sm);background:var(--dsw-alias-bg-layer-3,#262626);transform:translateX(calc(var(--dsh-segment-index,0) * 100%));transition:transform 140ms ease;box-shadow:0 1px 2px rgba(0,0,0,.25)}",
      ".dshr-segmented-tab{position:relative;z-index:1;font:inherit;font-size:13px;line-height:20px;color:var(--dsw-alias-label-secondary,#bbb);background:transparent;border:0;border-radius:var(--dshr-radius-sm);padding:5px 10px;cursor:pointer}",
      ".dshr-segmented-tab[aria-selected='true']{color:var(--dsw-alias-label-primary,#e6e6e6)}",
      ".dshr-segmented-tab:disabled{opacity:.5;cursor:default}",
      ".dshr-segmented-tabs{position:relative;display:grid;gap:2px;padding:2px;border-radius:var(--dshr-radius-md);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12))}",
      ".dshr-segmented-tabs-indicator{position:absolute;top:2px;bottom:2px;left:2px;border-radius:var(--dshr-radius-sm);background:var(--dsw-alias-bg-layer-3,#262626);transition:transform 140ms ease}",
      ".dshr-segmented-tabs .dshr-pill{position:relative;z-index:1;justify-content:center;height:auto;padding:5px 10px;background:transparent;border-radius:var(--dshr-radius-sm)}",
      ".dshr-segmented-tabs .dshr-pill-active{box-shadow:none}",
      ".dshr-text-shimmer{display:inline-flex;min-width:0}",
      ".dshr-text-shimmer-content{display:inline-flex;min-width:0;align-items:center;gap:4px}",
      ".dshr-text-shimmer-decoration{position:absolute;inset:0;overflow:hidden;pointer-events:none;mask-image:linear-gradient(90deg,transparent,#000 20%,#000 80%,transparent)}",
      ".dshr-text-shimmer[data-shimmer]>.dshr-text-shimmer-content{color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-text-shimmer-sweep{position:absolute;inset:0;display:inline-flex;align-items:center;gap:4px;background:linear-gradient(90deg,transparent,var(--dsw-alias-label-primary,#e6e6e6),transparent);background-size:200% 100%;-webkit-background-clip:text;background-clip:text;color:transparent;animation:dshr-shimmer 1.6s linear infinite}",
      "@keyframes dshr-shimmer{0%{background-position:200% 0}100%{background-position:-200% 0}}",
      ".dshr-state-dot{display:inline-block;flex:none;border-radius:50%;background:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-state-dot[data-state='done']{background:var(--dsw-alias-state-success-primary,#22c55e)}",
      ".dshr-state-dot[data-state='warning']{background:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-state-dot[data-state='error']{background:var(--dsw-alias-state-error-primary,#ef4444)}",
      ".dshr-state-dot[data-state='idle']{background:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-state-step{display:inline-flex;align-items:center;justify-content:center;border-radius:50%;border:1.5px solid var(--dsw-alias-border-l3,#454545);color:var(--dsw-alias-label-primary-foreground,#fff)}",
      ".dshr-state-step[data-state='done']{background:var(--dsw-alias-state-success-primary,#22c55e);border-color:transparent}",
      ".dshr-state-spinner{flex:none;color:var(--dsw-alias-state-business-primary,#4da3ff)}",
      ".dshr-state-spinner-track{fill:none;stroke:var(--dsw-alias-border-l3,#454545);stroke-width:2}",
      ".dshr-state-spinner-arc{fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-dasharray:20 40;animation:dshr-spin 1s linear infinite;transform-origin:12px 12px}",
      "@keyframes dshr-spin{to{transform:rotate(360deg)}}",
      ".dshr-path-label{display:inline-block;max-width:100%;overflow:hidden;white-space:nowrap;vertical-align:bottom}",
      ".dshr-path-label-text{display:inline-flex;min-width:0}",
      ".dshr-path-label-directory{color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-path-label-name{color:var(--dsw-alias-label-secondary,#bbb)}",
      ".dshr-path-label[data-path-clipped] .dshr-path-label-text{mask-image:linear-gradient(90deg,transparent,#000 24px)}",
      ".dshr-disclosure{display:block}",
      ".dshr-disclosure-header{display:flex;align-items:center;gap:4px;min-width:0}",
      ".dshr-disclosure-row{display:flex;align-items:center;gap:6px;min-width:0;flex:1;font-size:12px;line-height:24px;color:var(--dsw-alias-label-tertiary,#9b9b9b);border-radius:var(--dshr-radius-sm);padding:0 2px}",
      ".dshr-disclosure-row:hover{color:var(--dsw-alias-label-secondary,#bbb)}",
      ".dshr-disclosure-row[data-expandable]{cursor:pointer}",
      ".dshr-disclosure-row[data-expandable]:focus-visible{outline:2px solid var(--dsw-focus-ring-color,var(--dsw-alias-state-business-primary,#4da3ff));outline-offset:1px}",
      ".dshr-disclosure-leading{display:inline-flex;align-items:center;justify-content:center;flex:none;background:transparent;border:0;padding:0;color:inherit;cursor:pointer;width:16px;height:16px}",
      ".dshr-disclosure-title{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}",
      ".dshr-disclosure-chevron-hover{display:none}",
      ".dshr-disclosure-row:hover .dshr-disclosure-icon-idle{display:none}",
      ".dshr-disclosure-row:hover .dshr-disclosure-chevron-hover{display:inline-flex}",
      ".dshr-toast{position:fixed;top:16px;left:50%;transform:translateX(-50%);z-index:100;display:inline-flex;align-items:center;gap:8px;max-width:min(560px,90vw);padding:8px 12px;border-radius:var(--dshr-radius-md);background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-l3,#454545);box-shadow:0 8px 24px rgba(0,0,0,.35);font-size:13px;line-height:20px;color:var(--dsw-alias-label-primary,#e6e6e6);animation:dshr-toast-in 160ms ease}",
      ".dshr-toast-icon{display:inline-flex;flex:none}",
      ".dshr-toast-success .dshr-toast-icon{color:var(--dsw-alias-state-success-primary,#22c55e)}",
      ".dshr-toast-icon-warning{color:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-toast-text{min-width:0}",
      ".dshr-toast-action{background:none;border:0;padding:0;font:inherit;color:var(--dsw-alias-state-business-primary,#4da3ff);cursor:pointer}",
      "@keyframes dshr-toast-in{from{opacity:0;transform:translate(-50%,-6px)}to{opacity:1;transform:translate(-50%,0)}}",
      ".dshr-modal-root{position:fixed;inset:0;z-index:90;display:flex;align-items:center;justify-content:center;padding:16px}",
      ".dshr-modal-mask{position:absolute;inset:0;background:rgba(0,0,0,.45);backdrop-filter:blur(2px)}",
      ".dshr-modal-dialog{position:relative;display:flex;flex-direction:column;width:min(560px,100%);max-height:80vh;background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-l3,#454545);border-radius:12px;outline:none;box-shadow:0 16px 48px rgba(0,0,0,.4)}",
      ".dshr-modal-dialog.dshr-modal-warning{border-color:color-mix(in srgb,var(--dsw-alias-state-warn-primary,#f59e0b) 40%,var(--dsw-alias-border-l3,#454545))}",
      ".dshr-modal-content{display:flex;flex-direction:column;min-height:0;padding:16px 16px 0}",
      ".dshr-modal-header{display:flex;align-items:center;gap:8px}",
      ".dshr-modal-title{display:flex;align-items:center;gap:8px;margin:0;font-size:15px;line-height:22px;flex:1;min-width:0}",
      ".dshr-modal-warning-icon{color:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-modal-close{display:inline-flex;align-items:center;justify-content:center;width:24px;height:24px;border:0;border-radius:var(--dshr-radius-sm);background:transparent;color:var(--dsw-alias-label-tertiary,#9b9b9b);cursor:pointer}",
      ".dshr-modal-close:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.16));color:var(--dsw-alias-label-primary,#e6e6e6)}",
      ".dshr-modal-description{margin:8px 0 0;font-size:12px;line-height:18px;color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-modal-body{min-height:0;overflow:auto;padding:12px 0}",
      ".dshr-modal-footer{display:flex;justify-content:flex-end;gap:8px;padding:12px 16px;border-top:1px solid var(--dsw-alias-border-l4,#3a3a3a)}",
      ".dshr-menu{position:fixed;margin:0;padding:4px;list-style:none;min-width:160px;max-height:320px;overflow:auto;z-index:80;background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-l3,#454545);border-radius:var(--dshr-radius-md);box-shadow:0 8px 24px rgba(0,0,0,.35)}",
      ".dshr-menu-row{display:flex;align-items:center;gap:8px;width:100%;text-align:left;background:transparent;border:0;color:inherit;font:inherit;font-size:13px;line-height:20px;padding:5px 8px;border-radius:var(--dshr-radius-sm);cursor:pointer}",
      ".dshr-menu-row:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.16))}",
      ".dshr-menu-row[data-selected]{color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-menu-row:disabled{opacity:.5;cursor:default}",
      ".dshr-menu-separator{height:1px;margin:4px 6px;background:var(--dsw-alias-border-l4,#3a3a3a);list-style:none}",
      ".dshr-menu-label{padding:4px 8px;font-size:11px;line-height:16px;color:var(--dsw-alias-label-tertiary,#9b9b9b);list-style:none}",
      ".dshr-menu-root{display:inline-flex}",
      ".dshr-menu-list{min-width:160px;max-height:min(60vh,320px);overflow:auto;padding:4px}",
      ".dshr-menu-viewport{display:flex;flex-direction:column}",
      ".dshr-menu-row-wrap{position:relative}",
      ".dshr-menu-row{display:flex;align-items:center;gap:8px;width:100%;text-align:left;background:transparent;border:0;color:inherit;font:inherit;font-size:13px;line-height:20px;padding:5px 8px;border-radius:var(--dshr-radius-sm);cursor:pointer}",
      ".dshr-menu-row:hover:not(:disabled){background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.16))}",
      ".dshr-menu-row:disabled{opacity:.5;cursor:default}",
      ".dshr-menu-row[aria-disabled='true']{opacity:.5;cursor:default}",
      ".dshr-menu-row-icon{display:inline-flex;width:16px;height:16px;align-items:center;justify-content:center;flex:none;color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-menu-row-label{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}",
      ".dshr-menu-row-shortcut{display:inline-flex;flex:none;color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-menu-row-check{display:inline-flex;flex:none;color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-menu-row-selected{color:var(--dsw-alias-label-primary,#e6e6e6)}",
      ".dshr-menu-row-selected-fill{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.16));color:var(--dsw-alias-label-primary,#e6e6e6)}",
      ".dshr-menu-row-danger{color:var(--dsw-alias-state-error-primary,#ef4444)}",
      ".dshr-menu-row-danger:hover:not(:disabled){background:color-mix(in srgb,var(--dsw-alias-state-error-primary,#ef4444) 12%,transparent)}",
      ".dshr-menu-footer{border-top:1px solid var(--dsw-alias-border-l4,#3a3a3a);margin-top:4px;padding-top:4px}",
      ".dshr-menu-dense .dshr-menu-row{padding:3px 8px;line-height:18px}",
      ".dshr-menu-compact .dshr-menu-row{font-size:12px;line-height:18px;padding:3px 8px}",
      ".dshr-menu-submenu{z-index:81}",
      ".dshr-menu-surface-compact{border-radius:8px}",
      ".dshr-shortcut-keys-joined{gap:0}",
      ".dshr-menu-group{display:flex;flex-direction:column}",
      ".dshr-menu-group-start{display:block;height:0;overflow:hidden}",
      ".dshr-menu-group-heading{position:sticky;top:0;z-index:1;padding:4px 8px;font-size:11px;line-height:16px;color:var(--dsw-alias-label-tertiary,#9b9b9b);background:var(--dsw-alias-bg-layer-1,#1f1f1f)}",
      ".dshr-menu-group-heading[data-stuck]{background:var(--dsw-alias-bg-layer-1,#1f1f1f);box-shadow:0 1px 0 var(--dsw-alias-border-l4,#3a3a3a)}",
      ".dshr-menu-list[data-submenu-layout='page'] .dshr-menu-viewport{gap:2px}",
      ".dshr-menu-row[data-menu-back]{color:var(--dsw-alias-label-secondary,#bbb)}",
      ".dshr-menu-surface{background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-l3,#454545);border-radius:var(--dshr-radius-md);box-shadow:0 8px 24px rgba(0,0,0,.35);padding:8px;outline:none}",
      ".dshr-tooltip-bubble{position:fixed;z-index:110;display:inline-flex;align-items:center;gap:6px;max-width:min(320px,calc(100vw - 24px));padding:5px 8px;border-radius:var(--dshr-radius-sm);background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-l3,#454545);box-shadow:0 8px 24px rgba(0,0,0,.35);font-size:12px;line-height:16px;color:var(--dsw-alias-label-primary,#e6e6e6);pointer-events:none}",
      ".dshr-tooltip-bubble[data-pinned]{pointer-events:auto;max-height:min(70vh,480px);overflow:auto}",
      ".dshr-tooltip-label{min-width:0}",
      ".dshr-shortcut-keys{display:inline-flex;align-items:center;gap:2px;font-size:11px;line-height:16px}",
      ".dshr-shortcut-key{display:inline-flex;align-items:center;justify-content:center;min-width:18px;height:18px;padding:0 4px;border-radius:4px;border:0.5px solid var(--dsw-alias-border-l4,#3a3a3a);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));color:var(--dsw-alias-label-secondary,#bbb);font:inherit;font-size:11px}",
      ".dshr-shortcut-separator{border:0;background:transparent;min-width:0;padding:0;color:var(--dsw-alias-label-tertiary,#9b9b9b)}",
      ".dshr-shortcut-keys-tooltip .dshr-shortcut-key{background:transparent;border-color:var(--dsw-alias-border-l3,#454545)}",
      ".dshr-spinner{display:inline-block;width:12px;height:12px;border:2px solid var(--dsw-alias-border-l3,#454545);border-top-color:var(--dsw-alias-brand-primary,#4da3ff);border-radius:50%;animation:dshr-spin .8s linear infinite}",
      ".dshr-json-block{margin:6px 0;padding:8px;border-radius:var(--dshr-radius-sm);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));overflow-x:auto;font:12px/1.4 ui-monospace,monospace}",
      ".dshr-markdown-text{white-space:pre-wrap;overflow-wrap:anywhere}",
      ".dshr-markdown{min-width:0;overflow-wrap:anywhere;color:var(--dsw-alias-label-primary,inherit);font-size:14px;line-height:22px}",
      ".dshr-markdown-compact{font-size:12px;line-height:18px}",
      ".dshr-markdown h1,.dshr-markdown h2,.dshr-markdown h3,.dshr-markdown h4,.dshr-markdown h5,.dshr-markdown h6{margin:12px 0 6px;line-height:1.3;font-weight:600}",
      ".dshr-markdown h1{font-size:20px}.dshr-markdown h2{font-size:17px}.dshr-markdown h3{font-size:15px}.dshr-markdown h4,.dshr-markdown h5,.dshr-markdown h6{font-size:14px}",
      ".dshr-markdown p{margin:6px 0}",
      ".dshr-markdown ul,.dshr-markdown ol{margin:6px 0;padding-left:22px}",
      ".dshr-markdown li{margin:2px 0}",
      ".dshr-markdown code{font:12px/1.5 ui-monospace,monospace;padding:1px 4px;border-radius:4px;background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12))}",
      ".dshr-markdown pre{margin:8px 0;padding:10px;border-radius:var(--dshr-radius-sm);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));overflow:auto}",
      ".dshr-markdown pre code{padding:0;background:transparent}",
      ".dshr-markdown blockquote{margin:8px 0;padding:2px 10px;border-left:2px solid var(--dsw-alias-border-l3,#454545);color:var(--dsw-alias-label-secondary,#bbb)}",
      ".dshr-markdown a{color:var(--dsw-alias-state-business-primary,#4da3ff);text-decoration:none}",
      ".dshr-markdown a:hover{text-decoration:underline}",
      ".dshr-markdown img{max-width:100%;border-radius:var(--dshr-radius-sm)}",
      ".dshr-markdown hr{margin:12px 0;border:0;border-top:1px solid var(--dsw-alias-border-l4,#3a3a3a)}",
      ".dshr-json-block-root{margin:6px 0;font:12px/1.5 ui-monospace,monospace}",
      ".dshr-json-block-toggle{display:inline-flex;align-items:center;gap:4px;background:transparent;border:0;padding:0;font:inherit;color:var(--dsw-alias-label-tertiary,#9b9b9b);cursor:pointer}",
      ".dshr-json-block-toggle:hover{color:var(--dsw-alias-label-primary,#e6e6e6)}",
      ".dshr-json-block-body{margin:6px 0;padding:8px;border-radius:var(--dshr-radius-sm);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));overflow:auto}",
      ".dshr-icon{display:inline-flex;align-items:center;justify-content:center;flex:none;line-height:0}",
      ".dshr-icon svg{display:block}",
    ].join("\n");
    document.head.appendChild(style);
  }

  function classNames() {
    var out = [];
    for (var index = 0; index < arguments.length; index += 1) {
      var value = arguments[index];
      if (value) out.push(value);
    }
    return out.join(" ");
  }

  function passThrough(props, excluded) {
    var rest = {};
    if (props) {
      Object.keys(props).forEach(function (key) {
        if (excluded.indexOf(key) === -1) rest[key] = props[key];
      });
    }
    return rest;
  }

  function forwardRef(render) {
    if (typeof React.forwardRef !== "function") {
      var fallback = function FallbackRef(props) {
        return render(props, null);
      };
      fallback.displayName = render.name || "ForwardRefFallback";
      return fallback;
    }
    return React.forwardRef(render);
  }


  // ----- Button (reference Button.tsx) --------------------------------------
  var Button = forwardRef(function Button(props, ref) {
    var variant = props.variant || "ghost";
    var size = props.size || "md";
    var rest = passThrough(props, ["variant", "size", "icon", "className", "children"]);
    return h(
      "button",
      Object.assign({ ref: ref, type: "button", className: classNames("dshr-button", "dshr-button-" + variant, "dshr-button-" + size, props.className) }, rest),
      props.icon != null ? h("span", { className: "dshr-button-icon" }, props.icon) : null,
      props.children,
    );
  });

  // ----- Input (reference Input.tsx) ----------------------------------------
  var Input = forwardRef(function Input(props, ref) {
    var rest = passThrough(props, ["icon", "className"]);
    return h(
      "span",
      { className: classNames("dshr-input", props.className) },
      props.icon != null ? h("span", { className: "dshr-input-icon" }, props.icon) : null,
      h("input", Object.assign({ ref: ref, className: "dshr-input-input" }, rest)),
    );
  });

  // ----- Switch (reference Switch.tsx) ---------------------------------------
  function Switch(props) {
    var checked = props.checked === true;
    return h(
      "button",
      {
        type: "button",
        role: "switch",
        "aria-checked": checked,
        "aria-label": props.label,
        "aria-describedby": props.describedBy,
        title: props.title,
        disabled: props.disabled === true,
        autoFocus: props.autoFocus,
        className: classNames("dshr-switch", props.className),
        onClick: function () {
          if (typeof props.onChange === "function") props.onChange(!checked);
        },
      },
      h("span", { className: "dshr-switch-thumb" }),
    );
  }

  // ----- Checkbox (reference Checkbox.tsx) -----------------------------------
  function Checkbox(props) {
    return h(
      "label",
      { className: classNames("dshr-checkbox", props.className), title: props.title },
      h("input", {
        type: "checkbox",
        checked: props.checked === true,
        disabled: props.disabled === true,
        "data-modal-autofocus": props.modalAutofocus ? "" : undefined,
        onChange: function (event) {
          if (typeof props.onChange === "function") props.onChange(event.target.checked);
        },
      }),
      h("span", null, props.label),
    );
  }

  // ----- Tag (reference Tag.tsx) ---------------------------------------------
  function Tag(props) {
    return h(
      "span",
      { className: classNames("dshr-tag", props.className), "data-tone": props.tone || "outline" },
      props.children,
    );
  }

  // ----- Pill (reference Pill.tsx) -------------------------------------------
  function Pill(props) {
    var active = props.active === true;
    if (!props.onClick) {
      return h("span", { className: classNames("dshr-pill", active && "dshr-pill-active", props.className) }, props.children);
    }
    var rest = passThrough(props, ["active", "className", "children", "onClick"]);
    return h(
      "button",
      Object.assign({ type: "button", className: classNames("dshr-pill", "dshr-pill-interactive", active && "dshr-pill-active", props.className), onClick: props.onClick }, rest),
      props.children,
    );
  }

  // ----- SegmentedControl (reference SegmentedControl.tsx) -------------------
  var SEGMENT_WALK_KEYS = { ArrowLeft: true, ArrowRight: true, ArrowUp: true, ArrowDown: true, Home: true, End: true };

  function segmentWalk(options, from, key) {
    var enabled = options.filter(function (option) {
      return option.disabled !== true;
    });
    if (key === "Home") return enabled[0];
    if (key === "End") return enabled[enabled.length - 1];
    var step = key === "ArrowRight" || key === "ArrowDown" ? 1 : -1;
    var count = options.length;
    for (var offset = 1; offset < count; offset += 1) {
      var candidate = options[(((from + step * offset) % count) + count) % count];
      if (candidate !== undefined && candidate.disabled !== true) return candidate;
    }
    return undefined;
  }

  function SegmentedControl(props) {
    var options = Array.isArray(props.options) ? props.options : [];
    var selected = options.findIndex(function (option) {
      return option.value === props.value;
    });
    var rootRef = React.useRef(null);
    React.useEffect(function () {
      var root = rootRef.current;
      if (root === null || typeof document === "undefined") return;
      if (!root.contains(document.activeElement)) return;
      var active = root.querySelector('[role="tab"][aria-selected="true"]');
      if (active && typeof active.focus === "function") active.focus();
    }, [props.value]);
    function onKeyDown(event) {
      if (!SEGMENT_WALK_KEYS[event.key]) return;
      event.preventDefault();
      var target = segmentWalk(options, selected, event.key);
      if (target !== undefined && target.value !== props.value && typeof props.onChange === "function") {
        props.onChange(target.value);
      }
    }
    var indicatorStyle = {};
    indicatorStyle["--dsh-segment-count"] = String(options.length);
    indicatorStyle["--dsh-segment-index"] = String(selected < 0 ? 0 : selected);
    return h(
      "div",
      {
        ref: rootRef,
        role: "tablist",
        "aria-label": props.label,
        className: classNames("dshr-segmented", props.className),
        style: indicatorStyle,
      },
      h("span", { "aria-hidden": "true", className: "dshr-segmented-indicator" }),
      options.map(function (option) {
        var active = option.value === props.value;
        return h(
          "button",
          {
            key: option.value,
            id: props.id + "-" + option.value,
            type: "button",
            role: "tab",
            "aria-selected": active,
            "aria-controls": props.id + "-" + option.value + "-panel",
            tabIndex: active ? 0 : -1,
            disabled: props.disabled === true || option.disabled === true,
            title: option.title,
            className: "dshr-segmented-tab",
            onClick: function () {
              if (!active && typeof props.onChange === "function") props.onChange(option.value);
            },
            onKeyDown: onKeyDown,
          },
          option.label,
        );
      }),
    );
  }

  // ----- SegmentedTabs (reference SegmentedTabs.tsx) -------------------------
  function SegmentedTabs(props) {
    var items = Array.isArray(props.items) ? props.items : [];
    var value = props.value;
    var selectedIndex = items.findIndex(function (item) {
      return item.value === value;
    });
    function onKeyDown(event, index) {
      var next;
      switch (event.key) {
        case "ArrowLeft":
          next = (index + items.length - 1) % items.length;
          break;
        case "ArrowRight":
          next = (index + 1) % items.length;
          break;
        case "Home":
          next = 0;
          break;
        case "End":
          next = items.length - 1;
          break;
        default:
          return;
      }
      event.preventDefault();
      event.stopPropagation();
      var tablist = event.currentTarget.parentElement;
      var nextItem = items[next];
      if (tablist === null || nextItem === undefined) return;
      var buttons = tablist.querySelectorAll('[role="tab"]');
      var node = buttons.item(next);
      if (node && typeof node.focus === "function") node.focus();
      if (typeof props.onChange === "function") props.onChange(nextItem.value);
    }
    var style = { gridTemplateColumns: "repeat(" + items.length + ", minmax(0, 1fr))" };
    return h(
      "div",
      {
        role: "tablist",
        "data-size": props.size || "md",
        "aria-label": props.label,
        className: classNames("dshr-segmented-tabs", props.className),
        style: style,
      },
      h("span", {
        className: "dshr-segmented-tabs-indicator",
        "aria-hidden": "true",
        style: {
          width: "calc((100% - 4px) / " + items.length + ")",
          transform: "translateX(" + (selectedIndex < 0 ? 0 : selectedIndex) * 100 + "%)",
        },
      }),
      items.map(function (item, index) {
        return h(
          Pill,
          {
            key: item.value,
            id: item.id,
            role: "tab",
            className: "dshr-segmented-tabs-tab",
            "aria-selected": value === item.value,
            "aria-controls": item.panelId,
            tabIndex: value === item.value ? 0 : -1,
            onClick: function () {
              if (typeof props.onChange === "function") props.onChange(item.value);
            },
            onKeyDown: function (event) {
              onKeyDown(event, index);
            },
          },
          item.label,
        );
      }),
    );
  }

  // ----- TextShimmer (reference TextShimmer.tsx) -----------------------------
  var DecorativeCopy = React.createContext(undefined);

  function shimmerText(children, className) {
    var decorative = React.useContext(DecorativeCopy);
    var generated = decorative === true && typeof children === "string";
    return h(
      "span",
      { className: classNames("dshr-text-shimmer-text", className), "data-shimmer-text": generated ? children : undefined },
      generated ? null : children,
    );
  }

  function TextShimmer(props) {
    var decorative = React.useContext(DecorativeCopy);
    if (decorative !== undefined) {
      return shimmerText(props.children, props.className);
    }
    var content = typeof props.children === "string" ? shimmerText(props.children, undefined) : props.children;
    return h(
      "span",
      { className: classNames("dshr-text-shimmer", props.className), "data-shimmer": props.active ? "" : undefined },
      h(
        DecorativeCopy.Provider,
        { value: false },
        h("span", { className: classNames("dshr-text-shimmer-content", props.contentClassName) }, content),
      ),
      props.active
        ? h(
            "span",
            { className: "dshr-text-shimmer-decoration", "aria-hidden": "true" },
            h(
              "span",
              { className: "dshr-text-shimmer-sweep" },
              h(DecorativeCopy.Provider, { value: true },
                h("span", { className: classNames("dshr-text-shimmer-content", props.contentClassName) }, content)),
            ),
          )
        : null,
    );
  }

  // ----- StateDot (reference StateDot.tsx) -----------------------------------
  function StateDot(props) {
    var state = props.state || "idle";
    if (state === "ongoing") {
      var edge = props.size || 14;
      return h(
        "svg",
        { className: classNames("dshr-state-spinner", props.className), "data-state": "ongoing", width: edge, height: edge, viewBox: "0 0 24 24", "aria-hidden": "true" },
        h("circle", { className: "dshr-state-spinner-track", cx: 12, cy: 12, r: 9.5 }),
        h("circle", { className: "dshr-state-spinner-arc", cx: 12, cy: 12, r: 9.5 }),
      );
    }
    var size = props.size || 10;
    var step = (props.appearance || "dot") === "step";
    return h(
      step ? "span" : "span",
      {
        className: classNames(step ? "dshr-state-step" : "dshr-state-dot", props.className),
        "data-state": state,
        style: { width: size, height: size },
        "aria-hidden": "true",
      },
      step && state === "done" ? h(IconCheckOutlineRegular, { size: size - 2 }) : null,
    );
  }

  // ----- PathLabel (reference PathLabel.tsx) --------------------------------
  function pathPartsOf(path) {
    var value = String(path === undefined ? "" : path);
    var index = Math.max(value.lastIndexOf("/"), value.lastIndexOf("\\"));
    if (index < 0) return { directory: "", name: value };
    if (index === 0) return { directory: value.slice(0, 1), name: value.slice(1) };
    return { directory: value.slice(0, index + 1), name: value.slice(index + 1) };
  }

  var PathLabel = forwardRef(function PathLabel(props, ref) {
    var parts = pathPartsOf(props.path);
    var rest = passThrough(props, ["path", "className"]);
    var outer = React.useRef(null);
    var inner = React.useRef(null);
    React.useLayoutEffect(function () {
      var box = outer.current;
      var text = inner.current;
      if (!box || !text || typeof box.toggleAttribute !== "function") return undefined;
      function apply() {
        box.toggleAttribute("data-path-clipped", text.offsetWidth > box.clientWidth);
      }
      apply();
      var observer;
      if (typeof ResizeObserver !== "undefined") {
        observer = new ResizeObserver(apply);
        observer.observe(box);
        observer.observe(text);
      }
      return function () {
        if (observer) observer.disconnect();
      };
    }, [props.path]);
    return h(
      "span",
      Object.assign({}, rest, { ref: function (node) { outer.current = node; if (typeof ref === "function") ref(node); }, className: classNames("dshr-path-label", props.className), title: props.path, "data-path-label": "" }),
      h(
        "span",
        { ref: inner, className: "dshr-path-label-text" },
        parts.directory !== "" ? h("span", { className: "dshr-path-label-directory" }, parts.directory) : null,
        h("span", { className: "dshr-path-label-name" }, parts.name),
      ),
    );
  });

  // ----- DisclosureRow (reference DisclosureRow.tsx) ------------------------
  var DisclosureRow = React.memo(function DisclosureRow(props) {
    var open = props.open === true;
    var expandable = props.expandable === true;
    var rowExpands = expandable && props.expandOnRowClick === true;
    var previewChevron = props.previewChevron === undefined ? expandable : props.previewChevron === true;
    var collapsedLeading = previewChevron
      ? [
          h("span", { key: "idle", className: "dshr-disclosure-icon-idle" }, props.icon),
          h(IconChevronDownOutlineRegular, { key: "chevron", className: classNames("dshr-disclosure-chevron-hover", props.chevronClassName) }),
        ]
      : props.icon;
    var leading = open
      ? h(IconChevronUpOutlineRegular, { className: props.chevronClassName })
      : collapsedLeading;
    function toggleFromLeading(event) {
      event.stopPropagation();
      if (typeof props.onToggle === "function") props.onToggle();
    }
    function toggleFromKeyboard(event) {
      if (!rowExpands || (event.key !== "Enter" && event.key !== " ")) return;
      event.preventDefault();
      if (typeof props.onToggle === "function") props.onToggle();
    }
    return h(
      "div",
      { className: classNames("dshr-disclosure", props.className), "data-open": open ? "" : undefined },
      h(
        "div",
        { className: "dshr-disclosure-header", "data-disclosure-header": "", "data-has-accessory": props.headerAccessory ? "" : undefined },
        h(
          "div",
          {
            className: classNames("dshr-disclosure-row", props.rowClassName),
            "data-disclosure-row": "",
            "data-expandable": rowExpands ? "" : undefined,
            role: rowExpands ? "button" : undefined,
            tabIndex: rowExpands ? 0 : undefined,
            "aria-expanded": rowExpands ? open : undefined,
            onClick: rowExpands ? props.onToggle : undefined,
            onKeyDown: rowExpands ? toggleFromKeyboard : undefined,
          },
          expandable && !rowExpands
            ? h(
                "button",
                {
                  type: "button",
                  className: classNames("dshr-disclosure-leading", props.leadingClassName),
                  "aria-label": props.title,
                  "aria-expanded": open,
                  onClick: toggleFromLeading,
                },
                leading,
              )
            : h("span", { className: classNames("dshr-disclosure-leading", props.leadingClassName) }, leading),
          h(
            TextShimmer,
            { active: props.running === true, className: props.contentClassName, contentClassName: props.contentLayoutClassName },
            h(TextShimmer, { className: classNames("dshr-disclosure-title", props.titleClassName) }, props.title),
            props.keepContentWhenOpen || !open ? props.collapsedContent : null,
          ),
        ),
        props.headerAccessory,
      ),
      open ? props.children : null,
    );
  });


  // ----- Toast (reference Toast.tsx) ----------------------------------------
  var TOAST_FADE_MS = 1000;

  function Toast(props) {
    var holdMs = typeof props.holdMs === "number" ? props.holdMs : 3000;
    var latestOnDone = React.useRef(props.onDone);
    React.useLayoutEffect(function () {
      latestOnDone.current = props.onDone;
    }, [props.onDone]);
    React.useEffect(function () {
      var timer = setTimeout(function () {
        if (typeof latestOnDone.current === "function") latestOnDone.current();
      }, holdMs + TOAST_FADE_MS);
      return function () {
        clearTimeout(timer);
      };
    }, [holdMs]);
    var anchorState = React.useState(null);
    var left = anchorState[0];
    var setLeft = anchorState[1];
    React.useLayoutEffect(function () {
      var anchor = props.anchor;
      if (anchor == null) return undefined;
      function measure() {
        var rect = anchor.getBoundingClientRect();
        setLeft(rect.left + rect.width / 2);
      }
      measure();
      window.addEventListener("resize", measure);
      return function () {
        window.removeEventListener("resize", measure);
      };
    }, [props.anchor]);
    var body = h(
      "div",
      {
        className: classNames("dshr-toast", props.tone === "success" ? "dshr-toast-success" : "", props.className),
        role: "alert",
        style: Object.assign(
          left === null ? {} : { left: left },
          { "--dsh-toast-hold": holdMs + "ms" },
        ),
      },
      props.tone === "success"
        ? h("span", { className: "dshr-toast-icon", "aria-hidden": "true" }, h(IconCheckCircleOutlineRegular, { size: 16 }))
        : props.icon !== undefined
          ? h("span", { className: classNames("dshr-toast-icon", "dshr-toast-icon-warning"), "aria-hidden": "true" }, props.icon)
          : null,
      h(
        "span",
        { className: "dshr-toast-text" },
        props.text,
        Array.isArray(props.actions)
          ? props.actions.map(function (action) {
              return h(
                React.Fragment,
                { key: action.label },
                action.prefix,
                h(
                  "button",
                  { type: "button", className: "dshr-toast-action", onClick: action.onClick },
                  action.label,
                ),
              );
            })
          : null,
      ),
    );
    if (!ReactDOM || typeof document === "undefined") return body;
    return ReactDOM.createPortal(body, document.body);
  }

  // ----- Modal (reference Modal.tsx) ----------------------------------------
  function Modal(props) {
    var dialogRef = React.useRef(null);
    var open = props.open === true;
    React.useEffect(function () {
      if (!open) return undefined;
      function onKeyDown(event) {
        if (event.key !== "Escape" || event.repeat) return;
        if (typeof props.onClose === "function") {
          event.preventDefault();
          event.stopPropagation();
          props.onClose();
        }
      }
      document.addEventListener("keydown", onKeyDown, true);
      var dialog = dialogRef.current;
      var autofocus = dialog ? dialog.querySelector("[data-modal-autofocus]") : null;
      if (autofocus && typeof autofocus.focus === "function") autofocus.focus();
      else if (dialog && typeof dialog.focus === "function") dialog.focus();
      return function () {
        document.removeEventListener("keydown", onKeyDown, true);
      };
    }, [open, props.onClose]);
    if (!open) return null;
    var headless = props.headless === true;
    var dialog = h(
      "div",
      {
        ref: dialogRef,
        tabIndex: -1,
        "data-shortcut-modal": props.shortcutModal,
        className: classNames("dshr-modal-dialog", !headless && props.tone === "warning" && "dshr-modal-warning", props.className),
        role: "dialog",
        "aria-modal": "true",
        "aria-label": props.title,
      },
      headless
        ? props.children
        : [
            h(
              "div",
              { key: "content", className: classNames("dshr-modal-content", props.contentClassName) },
              h(
                "div",
                { className: "dshr-modal-header" },
                h(
                  "h2",
                  { className: "dshr-modal-title" },
                  props.tone === "warning"
                    ? h(IconWarningOutlineRegular, { size: 18, className: "dshr-modal-warning-icon", "aria-hidden": "true" })
                    : null,
                  props.title,
                ),
                h(
                  "button",
                  { type: "button", className: "dshr-modal-close", "aria-label": props.closeLabel, onClick: props.onClose },
                  h(IconCloseOutlineRegular, { size: 14 }),
                ),
              ),
              props.description !== undefined && props.description !== ""
                ? h("p", { className: "dshr-modal-description" }, props.description)
                : null,
              props.children !== undefined ? h("div", { className: "dshr-modal-body" }, props.children) : null,
            ),
            props.footer !== undefined ? h("div", { key: "footer", className: "dshr-modal-footer" }, props.footer) : null,
          ],
    );
    var tree = h(
      "div",
      { className: "dshr-modal-root", role: "presentation", onKeyDownCapture: props.onKeyDownCapture },
      h("div", {
        className: "dshr-modal-mask",
        "aria-hidden": "true",
        style: props.backdropBlur === false ? { backdropFilter: "none" } : undefined,
        onClick: props.onClose,
      }),
      dialog,
    );
    if (!ReactDOM || typeof document === "undefined") return tree;
    return ReactDOM.createPortal(tree, document.body);
  }

  // ----- MenuSurface (reference MenuSurface.tsx, without the macOS backing) --
  var MenuSurface = forwardRef(function MenuSurface(props, ref) {
    var rest = passThrough(props, ["className", "children", "compact", "phase", "style"]);
    var style = Object.assign({}, props.style || {});
    style["--dsh-menu-enter"] = "40ms";
    style["--dsh-menu-dismiss"] = "80ms";
    style["--dsh-menu-replace"] = "10ms";
    return h(
      "div",
      Object.assign({}, rest, {
        ref: ref,
        "data-menu-material": "translucent",
        "data-menu-phase": props.phase,
        className: classNames("dshr-menu-surface", props.compact && "dshr-menu-surface-compact", props.className),
        style: style,
      }),
      props.children,
    );
  });

  // ----- Focus helpers (reference focus.ts) ---------------------------------
  var REMOVE_RING_KEYS = { Tab: true, ArrowUp: true, ArrowDown: true, ArrowLeft: true, ArrowRight: true, Home: true, End: true };
  var mobileDevice;

  function isMobileDevice() {
    if (mobileDevice !== undefined) return mobileDevice;
    var nav = typeof navigator === "undefined" ? {} : navigator;
    mobileDevice =
      /Android|iPhone|iPad|iPod/i.test(String(nav.userAgent || "")) ||
      (nav.platform === "MacIntel" && (nav.maxTouchPoints || 0) > 1);
    return mobileDevice;
  }

  function focusWithoutRing(element, options) {
    function release() {
      element.removeAttribute("data-dsh-automatic-focus");
      element.removeEventListener("blur", release);
      element.removeEventListener("keydown", navigate, true);
    }
    function navigate(event) {
      if (!event.isComposing && !event.ctrlKey && !event.altKey && !event.metaKey && REMOVE_RING_KEYS[event.key]) release();
    }
    element.setAttribute("data-dsh-automatic-focus", "");
    element.addEventListener("blur", release);
    element.addEventListener("keydown", navigate, true);
    element.focus(options);
    if (typeof element.matches === "function" && !element.matches(":focus")) release();
  }

  // ----- Pointer grace (reference pointer-grace.ts) -------------------------
  var POINTER_GRACE_MS = 200;

  function usePointerGrace(close) {
    var timerRef = React.useRef(null);
    var closeRef = React.useRef(close);
    closeRef.current = close;
    var cancel = React.useCallback(function () {
      if (timerRef.current === null) return;
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }, []);
    var arm = React.useCallback(
      function () {
        cancel();
        timerRef.current = setTimeout(function () {
          timerRef.current = null;
          closeRef.current();
        }, POINTER_GRACE_MS);
      },
      [cancel],
    );
    React.useEffect(function () {
      return cancel;
    }, [cancel]);
    return { arm: arm, cancel: cancel };
  }

  var MENU_TRANSITION_MS = { enter: 40, dismiss: 80, replace: 10, prepare: 20 };

  // Reference menuHoverAllowed: hover interactions need a non-touch pointer and
  // a hover-capable device.
  function menuHoverAllowed(event) {
    if (!event || event.pointerType === "touch") return false;
    return typeof window.matchMedia !== "function" || window.matchMedia("(any-hover: hover)").matches;
  }

  // ----- Submenu placement (reference menu-submenu-position.ts) -------------
  var SUBMENU_GAP = 4;
  var SUBMENU_MARGIN = 12;

  function submenuNeedsPage(parentRect, width, viewportWidth) {
    var right = parentRect.right + SUBMENU_GAP + width <= viewportWidth - SUBMENU_MARGIN;
    var left = parentRect.left - SUBMENU_GAP - width >= SUBMENU_MARGIN;
    return !right && !left;
  }

  // ----- MenuGroup (reference MenuGroup.tsx) ---------------------------------
  function MenuGroup(props) {
    var headingId = React.useId ? React.useId() : "dshr-menu-group";
    return h(
      "section",
      {
        role: "group",
        "aria-labelledby": headingId,
        "data-menu-group": "",
        className: "dshr-menu-group",
      },
      h("span", { "aria-hidden": "true", "data-menu-group-start": "", className: "dshr-menu-group-start" }),
      h("div", { id: headingId, "data-menu-group-heading": "", className: "dshr-menu-group-heading" }, props.label),
      props.children,
    );
  }

  function observeStickyMenuGroups(viewport) {
    var groups = Array.prototype.slice
      .call(viewport.querySelectorAll(":scope > [data-menu-group]"))
      .flatMap(function (section) {
        var heading = section.querySelector(":scope > [data-menu-group-heading]");
        var start = section.querySelector(":scope > [data-menu-group-start]");
        return heading === null || start === null
          ? []
          : [{ section: section, heading: heading, start: start, atTop: false, above: false, topTime: -Infinity, aboveTime: -Infinity }];
      });
    if (groups.length === 0 || typeof IntersectionObserver === "undefined" || typeof ResizeObserver === "undefined") {
      return function () {};
    }
    var sections = new Map(groups.map(function (group) { return [group.section, group]; }));
    var starts = new Map(groups.map(function (group) { return [group.start, group]; }));
    var disposed = false;
    var stripObserver;
    var viewportHeight;
    function render(group) {
      var stuck = group.atTop && group.above;
      if (group.heading.hasAttribute("data-stuck") !== stuck) group.heading.toggleAttribute("data-stuck", stuck);
    }
    var startObserver = new IntersectionObserver(
      function (entries) {
        if (disposed) return;
        entries.forEach(function (entry) {
          var group = starts.get(entry.target);
          if (!group || entry.rootBounds === null || entry.time < group.aboveTime) return;
          group.above = entry.boundingClientRect.top < entry.rootBounds.top;
          group.aboveTime = entry.time;
          render(group);
        });
      },
      { root: viewport, threshold: [0, 1] },
    );
    groups.forEach(function (group) { startObserver.observe(group.start); });
    var sizeObserver = new ResizeObserver(function (entries) {
      if (disposed) return;
      entries.forEach(function (entry) {
        if (entry.target !== viewport || entry.contentRect.height === viewportHeight) return;
        viewportHeight = entry.contentRect.height;
        if (stripObserver) stripObserver.disconnect();
        var observer = new IntersectionObserver(
          function (intersections) {
            if (disposed || stripObserver !== observer) return;
            intersections.forEach(function (intersection) {
              var group = sections.get(intersection.target);
              if (!group || intersection.rootBounds === null) return;
              var top = intersection.rootBounds.top;
              if (intersection.time >= group.topTime) {
                group.atTop = intersection.isIntersecting && intersection.boundingClientRect.bottom > top;
                group.topTime = intersection.time;
              }
              if (intersection.time >= group.aboveTime) {
                group.above = intersection.boundingClientRect.top < top;
                group.aboveTime = intersection.time;
              }
              render(group);
            });
          },
          { root: viewport, rootMargin: "0px 0px " + (Math.min(1, viewportHeight) - viewportHeight) + "px 0px", threshold: 0 },
        );
        stripObserver = observer;
        groups.forEach(function (group) { observer.observe(group.section); });
      });
    });
    sizeObserver.observe(viewport);
    return function () {
      if (disposed) return;
      disposed = true;
      startObserver.disconnect();
      if (stripObserver) stripObserver.disconnect();
      sizeObserver.disconnect();
      groups.forEach(function (group) {
        if (group.heading.hasAttribute("data-stuck")) group.heading.removeAttribute("data-stuck");
      });
    };
  }

  // ----- Menu (reference Menu.tsx: data/component rows, decorations, footer,
  // selection markers, one-level submenu and the reference keyboard walk) -----
  function renderMenuDecoration(entry) {
    if (entry.type === "separator") {
      return h("div", { key: entry.id, className: "dshr-menu-separator", role: "separator" });
    }
    return h("div", { key: entry.id, className: "dshr-menu-label", role: "presentation" }, entry.text);
  }

  function MenuRow(props) {
    var reasonId = React.useId ? React.useId() : "dshr-menu-reason";
    var explained = props.disabled === true && props.disabledReason !== undefined;
    var rest = passThrough(props, [
      "children", "icon", "shortcut", "current", "disabled", "disabledReason", "danger",
      "separatorBefore", "selected", "selection", "reserveIcon", "decorativeIcon", "wrapperProps",
      "onClick", "hasSubmenu", "submenuOpen", "rowRef", "rowId",
    ]);
    var button = h(
      "button",
      Object.assign({}, rest, {
        ref: props.rowRef,
        type: "button",
        role: "menuitem",
        "data-menu-row-id": props.rowId,
        className: classNames(
          "dshr-menu-row",
          props.selected && (props.selection === "fill" ? "dshr-menu-row-selected-fill" : "dshr-menu-row-selected"),
          props.danger && "dshr-menu-row-danger",
        ),
        disabled: props.disabled === true && !explained,
        "aria-disabled": explained ? "true" : undefined,
        "aria-describedby": explained ? reasonId : undefined,
        "aria-keyshortcuts": props.shortcut ? props.shortcut.aria : undefined,
        "aria-current": props.current === true ? "page" : undefined,
        "aria-haspopup": props.hasSubmenu ? "menu" : rest["aria-haspopup"],
        "aria-expanded": props.hasSubmenu ? (props.submenuOpen === true ? "true" : "false") : rest["aria-expanded"],
        onClick: explained ? undefined : props.onClick,
      }),
      props.icon !== undefined || props.reserveIcon
        ? h("span", { "aria-hidden": props.decorativeIcon === false ? undefined : "true", className: "dshr-menu-row-icon" }, props.icon)
        : null,
      h("span", { className: "dshr-menu-row-label" }, props.children),
      props.shortcut !== undefined
        ? h("span", { "aria-hidden": "true", className: "dshr-menu-row-shortcut" }, h(ShortcutKeys, { keys: props.shortcut.keys }))
        : null,
      props.selected && (props.selection || "check") === "check"
        ? h(IconCheckOutlineRegular, { className: "dshr-menu-row-check" })
        : null,
      props.hasSubmenu ? h(IconChevronRightOutlineRegular, { "aria-hidden": "true", className: "dshr-menu-row-check" }) : null,
    );
    return h(
      "div",
      Object.assign({}, props.wrapperProps, { className: "dshr-menu-row-wrap" }),
      props.separatorBefore ? h("div", { className: "dshr-menu-separator", role: "separator" }) : null,
      explained ? h(Tooltip, { label: props.disabledReason, side: "bottom", delayMs: 500, maxWidth: 300, portal: true }, button) : button,
      explained ? h("span", { id: reasonId, hidden: true }, props.disabledReason) : null,
    );
  }

  function Menu(props) {
    var open = props.open === true;
    var items = Array.isArray(props.items) ? props.items : [];
    var footer = Array.isArray(props.footer) ? props.footer : [];
    var selection = props.selection || "check";
    var align = props.align || "start";
    var side = props.side || "bottom";
    var portal = props.portal === true;
    var listRef = React.useRef(null);
    var rootRef = React.useRef(null);
    var triggerRef = React.useRef(null);
    var walkIndex = React.useRef(null);
    var selectingWithTab = React.useRef(false);
    var submenuState = React.useState(null);
    var submenuId = submenuState[0];
    var setSubmenuId = submenuState[1];
    var submenuPageState = React.useState(false);
    var submenuPage = submenuPageState[0];
    var setSubmenuPage = submenuPageState[1];
    var submenuPanelRef = React.useRef(null);
    var focusChildRef = React.useRef(false);
    var submenuGraceRef = React.useRef(null);
    function cancelSubmenuClose() {
      if (submenuGraceRef.current !== null) {
        clearTimeout(submenuGraceRef.current);
        submenuGraceRef.current = null;
      }
    }
    function armSubmenuClose() {
      cancelSubmenuClose();
      submenuGraceRef.current = setTimeout(function () {
        submenuGraceRef.current = null;
        if (openRef.current) {
          setSubmenuId(null);
          setSubmenuPage(false);
        }
      }, POINTER_GRACE_MS);
    }
    React.useEffect(function () {
      return function () {
        cancelSubmenuClose();
      };
    }, []);
    var styleState = React.useState(null);
    var portalStyle = styleState[0];
    var setPortalStyle = styleState[1];
    var submenuStyleState = React.useState(null);
    var submenuStyle = submenuStyleState[0];
    var setSubmenuStyle = submenuStyleState[1];
    var closeRef = React.useRef(props.onClose);
    closeRef.current = props.onClose;
    var openRef = React.useRef(open);
    openRef.current = open;
    var grace = usePointerGrace(function () {
      if (openRef.current) closeRef.current();
    });

    function refocusAnchor(navigation) {
      var trigger = triggerRef.current;
      var target =
        trigger !== null && trigger.isConnected && trigger.disabled !== true
          ? trigger
          : rootRef.current
            ? rootRef.current.querySelector("button:not(:disabled)")
            : null;
      if (target === null) return;
      if (navigation) target.focus();
      else focusWithoutRing(target);
    }

    function refocusAfterSelection() {
      var navigation = selectingWithTab.current;
      queueMicrotask(function () {
        if (openRef.current) return;
        var active = document.activeElement;
        if (active === null || active === document.body || (listRef.current && listRef.current.contains(active))) {
          refocusAnchor(navigation);
        }
      });
    }

    React.useEffect(function () {
      if (!open) return;
      var active = document.activeElement;
      triggerRef.current =
        active instanceof HTMLElement && active !== document.body && rootRef.current && rootRef.current.contains(active)
          ? active
          : rootRef.current
            ? rootRef.current.querySelector("button:not(:disabled)")
            : null;
    }, [open]);

    React.useLayoutEffect(function () {
      if (!open) {
        setPortalStyle(null);
        return;
      }
      if (!portal || !rootRef.current) return;
      var rect = rootRef.current.getBoundingClientRect();
      var next = { position: "fixed", zIndex: 80 };
      if (props.minWidthFromAnchor) next.minWidth = rect.width;
      if (props.anchorPoint) {
        next.left = props.anchorPoint.x;
        next.top = props.anchorPoint.y;
      } else {
        if (side === "top") next.bottom = Math.max(8, window.innerHeight - rect.top + 4);
        else next.top = rect.bottom + 4;
        if (align === "end") next.right = Math.max(8, window.innerWidth - rect.right);
        else next.left = rect.left;
      }
      setPortalStyle(next);
    }, [open, portal, props.anchorPoint, props.minWidthFromAnchor, side, align]);

    React.useLayoutEffect(function () {
      if (submenuId === null || !listRef.current) {
        setSubmenuStyle(null);
        return;
      }
      var row = listRef.current.querySelector('[data-menu-row-id="' + submenuId + '"]');
      if (!row) {
        setSubmenuStyle(null);
        return;
      }
      var rect = row.getBoundingClientRect();
      var next = { position: "fixed", zIndex: 81, top: rect.top, minWidth: rect.width };
      if (rect.right + 220 > window.innerWidth) next.right = Math.max(8, window.innerWidth - rect.left + 4);
      else next.left = rect.right + 4;
      setSubmenuStyle(next);
    }, [submenuId]);

    React.useEffect(function () {
      if (!open) return undefined;
      function onPointerDown(event) {
        var target = event.target;
        if (target instanceof Node) {
          if (rootRef.current && rootRef.current.contains(target)) return;
          if (props.anchorRef && props.anchorRef.current && props.anchorRef.current.contains(target)) return;
          if (listRef.current && listRef.current.contains(target)) return;
          if (typeof props.isExternalTrigger === "function" && props.isExternalTrigger(target)) return;
        }
        closeRef.current();
      }
      function onWindowBlur() {
        if (document.activeElement instanceof HTMLIFrameElement) closeRef.current();
      }
      document.addEventListener("pointerdown", onPointerDown);
      window.addEventListener("blur", onWindowBlur);
      return function () {
        document.removeEventListener("pointerdown", onPointerDown);
        window.removeEventListener("blur", onWindowBlur);
      };
    }, [open, props.anchorRef, props.isExternalTrigger]);

    React.useEffect(function () {
      if (!open) {
        setSubmenuId(null);
        setSubmenuPage(false);
        cancelSubmenuClose();
      }
    }, [open]);

    // Reference submenu placement: a side card when either side of the parent
    // card fits, otherwise the child replaces the parent as a single page.
    React.useLayoutEffect(function () {
      if (submenuId === null) {
        setSubmenuPage(false);
        return;
      }
      var panel = submenuPanelRef.current;
      var list = listRef.current;
      if (!panel || !list || typeof window === "undefined") return;
      // Measure the natural card width while the panel is still hidden (the
      // reference measures before choosing side vs single-page layout).
      panel.style.visibility = "hidden";
      var width = panel.offsetWidth;
      var viewportWidth = document.documentElement.clientWidth || window.innerWidth;
      var page = submenuNeedsPage(list.getBoundingClientRect(), width, viewportWidth);
      setSubmenuPage(page);
      if (!page) {
        var row = list.querySelector('[data-menu-row-id="' + submenuId + '"]');
        if (row) {
          var rect = row.getBoundingClientRect();
          var top = rect.top;
          var height = panel.offsetHeight;
          if (top + height > window.innerHeight - SUBMENU_MARGIN) {
            top = Math.max(SUBMENU_MARGIN, window.innerHeight - SUBMENU_MARGIN - height);
          }
          var style = { position: "fixed", zIndex: 81, top: top, minWidth: rect.width, visibility: "hidden" };
          if (rect.right + SUBMENU_GAP + width > viewportWidth - SUBMENU_MARGIN) {
            style.right = Math.max(SUBMENU_MARGIN, viewportWidth - rect.left + SUBMENU_GAP);
          } else {
            style.left = rect.right + SUBMENU_GAP;
          }
          setSubmenuStyle(style);
        }
      }
    }, [submenuId]);

    // Keyboard/click opening a submenu moves focus into the child card; hover does not.
    React.useEffect(function () {
      if (submenuId === null || !focusChildRef.current || !listRef.current) return;
      focusChildRef.current = false;
      var scope = listRef.current.querySelector('[data-submenu-for="' + submenuId + '"]');
      var first = scope ? scope.querySelector('button[role="menuitem"]:not(:disabled):not([data-menu-back])') : null;
      if (first === null) first = listRef.current.querySelector('button[role="menuitem"]:not(:disabled):not([data-menu-back])');
      if (first !== null) first.focus();
    }, [submenuId, submenuPage]);

    React.useEffect(function () {
      if (!open) {
        walkIndex.current = null;
        return undefined;
      }
      function onKeyDown(event) {
        if (event.ctrlKey || event.altKey || event.metaKey || event.defaultPrevented) return;
        var focused = document.activeElement;
        var insideList = listRef.current ? listRef.current.contains(focused) : false;
        var childPanel =
          submenuId !== null && listRef.current
            ? listRef.current.querySelector('[data-submenu-for="' + submenuId + '"]')
            : null;
        var insideChild = childPanel ? childPanel.contains(focused) : false;
        var anchored =
          focused === triggerRef.current ||
          (props.anchorRef && props.anchorRef.current && props.anchorRef.current.contains(focused)) ||
          (rootRef.current && rootRef.current.contains(focused)) ||
          insideList;
        if (event.key === "Escape" && !event.shiftKey) {
          event.preventDefault();
          if (event.repeat) return;
          if (submenuId !== null && (insideChild || (childPanel && childPanel.contains(focused)))) {
            var parentRow = listRef.current ? listRef.current.querySelector('[data-menu-row-id="' + submenuId + '"]') : null;
            setSubmenuId(null);
            setSubmenuPage(false);
            if (parentRow && typeof parentRow.focus === "function") parentRow.focus();
            return;
          }
          if (submenuId !== null) {
            var parentRow2 = listRef.current ? listRef.current.querySelector('[data-menu-row-id="' + submenuId + '"]') : null;
            setSubmenuId(null);
            setSubmenuPage(false);
            if (parentRow2 && typeof parentRow2.focus === "function") parentRow2.focus();
            return;
          }
          closeRef.current();
          if (anchored || props.autoFocus === true) refocusAnchor(false);
          return;
        }
        if (event.key === "ArrowLeft" && anchored && submenuId !== null && (insideChild || !insideList)) {
          event.preventDefault();
          var returnRow = listRef.current ? listRef.current.querySelector('[data-menu-row-id="' + submenuId + '"]') : null;
          setSubmenuId(null);
          setSubmenuPage(false);
          if (returnRow && typeof returnRow.focus === "function") returnRow.focus();
          return;
        }
        if (event.key === "ArrowRight" && insideList && focused instanceof HTMLButtonElement) {
          var rowId = focused.getAttribute("data-menu-row-id");
          var parent = items.filter(function (entry) {
            return entry && !entry.type && entry.id === rowId && Array.isArray(entry.submenu) && entry.submenu.length > 0;
          })[0];
          if (parent) {
            event.preventDefault();
            focusChildRef.current = true;
            setSubmenuId(parent.id);
            return;
          }
        }
        if (event.key === "Tab") {
          if (!anchored) return;
          if (event.shiftKey) {
            event.preventDefault();
            closeRef.current();
            refocusAnchor(true);
            return;
          }
          if (insideList) {
            if (focused instanceof Element && focused.getAttribute("role") === "menuitem") {
              event.preventDefault();
              if (focused.getAttribute("aria-disabled") === "true") {
                closeRef.current();
                refocusAnchor(true);
                return;
              }
              selectingWithTab.current = true;
              try {
                focused.click();
              } finally {
                selectingWithTab.current = false;
              }
            }
            return;
          }
          var row = listRef.current ? listRef.current.querySelector("button:not(:disabled)") : null;
          if (row === null) return;
          event.preventDefault();
          row.focus();
          walkIndex.current = 0;
          return;
        }
        if (["ArrowDown", "ArrowUp", "Home", "End"].indexOf(event.key) === -1) return;
        if (!anchored) return;
        var scope = insideChild && childPanel ? childPanel : listRef.current;
        if (scope === null) return;
        var buttons = Array.prototype.slice.call(scope.querySelectorAll("button:not(:disabled)"));
        if (buttons.length === 0) return;
        var index = buttons.indexOf(focused);
        var from = index >= 0 ? index : walkIndex.current;
        var next =
          event.key === "Home"
            ? 0
            : event.key === "End"
              ? buttons.length - 1
              : from === null
                ? event.key === "ArrowDown"
                  ? 0
                  : buttons.length - 1
                : (from + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
        event.preventDefault();
        walkIndex.current = next;
        if (buttons[next] && typeof buttons[next].focus === "function") buttons[next].focus();
      }
      document.addEventListener("keydown", onKeyDown);
      return function () {
        document.removeEventListener("keydown", onKeyDown);
      };
    }, [open, submenuId, items, props.autoFocus, props.anchorRef]);

    React.useEffect(function () {
      if (!open || !props.autoFocus || !listRef.current) return;
      if (listRef.current.contains(document.activeElement)) return;
      var first = listRef.current.querySelector("button:not(:disabled)");
      walkIndex.current = first === null ? null : 0;
      if (first !== null) focusWithoutRing(first);
    }, [open, props.autoFocus, portalStyle, items.length, footer.length]);

    function selectionOf(entry) {
      return (
        entry.id === props.selectedId ||
        (Array.isArray(props.selectedIds) && props.selectedIds.indexOf(entry.id) !== -1)
      );
    }

    function alignedColumns(entries) {
      var aligned = [];
      if (!props.contextual) return aligned;
      var group = [];
      function finish() {
        if (group.some(function (entry) { return !("type" in entry) && entry.icon !== undefined; })) {
          aligned = aligned.concat(group);
        }
        group = [];
      }
      entries.forEach(function (entry) {
        if (entry && "type" in entry && entry.type === "separator") finish();
        else group.push(entry);
      });
      finish();
      return aligned;
    }

    function renderEntry(entry, columns, inSubmenu) {
      if (!entry || typeof entry !== "object") return null;
      if ("type" in entry) return renderMenuDecoration(entry);
      var hasSub = !inSubmenu && Array.isArray(entry.submenu) && entry.submenu.length > 0;
      return h(
        MenuRow,
        {
          key: entry.id,
          rowId: entry.id,
          icon: entry.icon,
          reserveIcon: columns.indexOf(entry) !== -1,
          disabled: entry.disabled === true,
          disabledReason: entry.disabledReason,
          danger: entry.danger,
          shortcut: entry.shortcut,
          current: entry.current,
          selected: selectionOf(entry),
          selection: selection,
          hasSubmenu: hasSub,
          submenuOpen: submenuId === entry.id,
          wrapperProps: hasSub
            ? {
                onPointerEnter: function (event) {
                  if (!menuHoverAllowed(event) || entry.disabled === true) return;
                  cancelSubmenuClose();
                  focusChildRef.current = false;
                  setSubmenuId(entry.id);
                },
              }
            : {
                onPointerEnter: function (event) {
                  if (!menuHoverAllowed(event)) return;
                  if (submenuId !== null && !submenuPage) {
                    setSubmenuId(null);
                    setSubmenuPage(false);
                  }
                },
              },
          onClick: function () {
            if (hasSub) {
              focusChildRef.current = true;
              setSubmenuId(entry.id);
              return;
            }
            if (entry.disabled === true) return;
            if (typeof props.onSelect === "function") props.onSelect(entry.id);
            refocusAfterSelection();
          },
        },
        entry.label,
      );
    }

    var list = null;
    if (open) {
      var itemColumns = alignedColumns(items);
      var footerColumns = alignedColumns(footer);
      var activeParent = submenuId === null
        ? null
        : items.filter(function (entry) {
            return entry && !entry.type && entry.id === submenuId;
          })[0];
      var childRows = activeParent && Array.isArray(activeParent.submenu)
        ? activeParent.submenu.map(function (entry) {
            return renderEntry(entry, [], true);
          })
        : [];
      var submenuPanel =
        activeParent && !submenuPage
          ? h(
              MenuSurface,
              {
                ref: submenuPanelRef,
                className: classNames("dshr-menu dshr-menu-submenu", props.compact && "dshr-menu-compact", props.dense && "dshr-menu-dense"),
                style: submenuStyle || { position: "fixed", display: "none" },
                "data-submenu-for": submenuId,
                role: "menu",
                onPointerEnter: cancelSubmenuClose,
                onPointerLeave: function (event) {
                  if (event.pointerType === "mouse") armSubmenuClose();
                },
                onClick: function (event) {
                  event.stopPropagation();
                  var row = event.target instanceof Element ? event.target.closest('button[role="menuitem"]') : null;
                  if (row !== null && row.getAttribute("aria-haspopup") !== "menu") refocusAfterSelection();
                },
              },
              h("div", { className: "dshr-menu-viewport", role: "presentation" }, childRows),
            )
          : null;
      var surfaceChildren;
      if (submenuPage && activeParent) {
        // Reference single-page layout: the child rows replace the parent page and
        // a non-selecting back row returns to it.
        surfaceChildren = [
          h(
            "div",
            { key: "page", className: "dshr-menu-viewport", role: "presentation" },
            h(
              MenuRow,
              {
                key: "back",
                "data-menu-back": "",
                icon: h(IconChevronLeftOutlineRegular, { className: "dshr-menu-row-check" }),
                onClick: function () {
                  var parentRow = listRef.current ? listRef.current.querySelector('[data-menu-row-id="' + submenuId + '"]') : null;
                  setSubmenuPage(false);
                  setSubmenuId(null);
                  if (parentRow && typeof parentRow.focus === "function") parentRow.focus();
                },
              },
              props.submenuBackLabel !== undefined ? props.submenuBackLabel : activeParent.label,
            ),
            h("div", { key: "sep", className: "dshr-menu-separator", role: "separator" }),
            childRows,
          ),
        ];
      } else {
        surfaceChildren = [
          h(
            "div",
            { key: "viewport", className: "dshr-menu-viewport", role: "presentation" },
            items.map(function (entry) {
              return renderEntry(entry, itemColumns, false);
            }),
            props.children,
          ),
        ];
        if (footer.length > 0) {
          surfaceChildren.push(
            h(
              "div",
              { key: "footer", className: "dshr-menu-footer", role: "presentation" },
              footer.map(function (entry) {
                return renderEntry(entry, footerColumns, false);
              }),
            ),
          );
        }
      }
      var surface = h(
        MenuSurface,
        {
          ref: listRef,
          id: props.contextual ? "dshr-context-menu" : undefined,
          className: classNames(
            "dshr-menu",
            "dshr-menu-list",
            props.listClassName,
            props.dense && "dshr-menu-dense",
            props.compact && "dshr-menu-compact",
            portal && "dshr-menu-portal",
          ),
          style: portal
            ? portalStyle || { position: "fixed", visibility: "hidden" }
            : {
                position: "absolute",
                zIndex: 80,
                minWidth: "100%",
                top: side === "top" ? undefined : "calc(100% + 4px)",
                bottom: side === "top" ? "calc(100% + 4px)" : undefined,
                left: align === "end" ? undefined : 0,
                right: align === "end" ? 0 : undefined,
              },
          role: "menu",
          "data-context-menu": props.contextual ? "" : undefined,
          "data-anchor-side": portal ? side : undefined,
          "data-submenu-layout": submenuId === null ? undefined : submenuPage ? "page" : "side",
          onPointerLeave: function (event) {
            if (submenuId !== null && !submenuPage && event.pointerType === "mouse") armSubmenuClose();
          },
          onPointerEnter: cancelSubmenuClose,
          onClick: function (event) {
            event.stopPropagation();
            var row = event.target instanceof Element ? event.target.closest('button[role="menuitem"]') : null;
            if (row !== null && row.getAttribute("aria-haspopup") !== "menu") refocusAfterSelection();
          },
        },
        surfaceChildren,
      );
      list = h(React.Fragment, null, surface, submenuPanel);
    }
    var rootStyle = props.anchorPoint
      ? { position: "fixed", left: props.anchorPoint.x, top: props.anchorPoint.y, width: 0, height: 0 }
      : { position: "relative" };
    return h(
      "span",
      {
        ref: rootRef,
        className: classNames("dshr-menu-root", props.className),
        style: rootStyle,
        onPointerEnter: props.closeOnPointerLeave ? grace.cancel : undefined,
        onPointerLeave: props.closeOnPointerLeave
          ? function (event) {
              if (open && event.pointerType === "mouse") grace.arm();
            }
          : undefined,
      },
      props.anchor,
      open && portal && ReactDOM ? ReactDOM.createPortal(list, document.body) : list,
    );
  }

  function MenuItemButton(props) {
    var rest = passThrough(props, ["onSelect", "children"]);
    return h(
      MenuRow,
      Object.assign({}, rest, {
        decorativeIcon: false,
        onClick: function () {
          if (typeof props.onSelect === "function") props.onSelect();
        },
      }),
      props.children,
    );
  }

  // ----- Input modality (reference input-modality.ts) ------------------------
  var INPUT_MODALITY_ATTRIBUTE = "data-input-modality";
  var FOCUS_NAVIGATION = { Tab: true, Home: true, End: true, PageUp: true, PageDown: true };
  var pointerInput = false;
  var pointerOwnsFocus = false;
  var keyFocusOrigin;
  function publishInputModality() {
    if (typeof document === "undefined" || !document.documentElement) return;
    document.documentElement.setAttribute(INPUT_MODALITY_ATTRIBUTE, pointerOwnsFocus ? "pointer" : "keyboard");
  }
  function pointerModality() {
    return pointerInput;
  }
  if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
    window.addEventListener(
      "pointerdown",
      function () {
        pointerInput = true;
        pointerOwnsFocus = true;
        keyFocusOrigin = undefined;
        publishInputModality();
      },
      true,
    );
    window.addEventListener(
      "keydown",
      function (event) {
        pointerInput = false;
        if (event.isComposing) {
          keyFocusOrigin = undefined;
          return;
        }
        var path = typeof event.composedPath === "function" ? event.composedPath() : [];
        keyFocusOrigin = path[0];
        if (!FOCUS_NAVIGATION[event.key] && String(event.key).indexOf("Arrow") !== 0) return;
        pointerOwnsFocus = false;
        publishInputModality();
      },
      true,
    );
    window.addEventListener(
      "focusin",
      function (event) {
        if (keyFocusOrigin === undefined) return;
        var path = typeof event.composedPath === "function" ? event.composedPath() : [];
        if (path[0] === keyFocusOrigin) return;
        keyFocusOrigin = undefined;
        if (!pointerOwnsFocus) return;
        pointerOwnsFocus = false;
        publishInputModality();
      },
      true,
    );
    window.addEventListener("blur", function () {
      keyFocusOrigin = undefined;
    });
  }

  // ----- Tooltip placement (reference tooltip-placement.ts) -----------------
  var TOOLTIP_OPPOSITE = { right: "left", left: "right", top: "bottom", bottom: "top" };

  function placeTooltip(input) {
    var anchor = input.anchor;
    var viewportWidth = input.viewportWidth;
    var viewportHeight = input.viewportHeight;
    var margin = 12;
    var maxWidth = Math.max(0, viewportWidth - margin * 2);
    var maxHeight = Math.max(0, viewportHeight - margin * 2);
    var width = Math.min(input.width, maxWidth);
    var height = Math.min(input.height, maxHeight);
    function clampX(x) {
      return Math.max(margin, Math.min(x, viewportWidth - margin - width));
    }
    function clampY(y) {
      return Math.max(margin, Math.min(y, viewportHeight - margin - height));
    }
    var side = input.side;
    var align = input.align;
    var gap = input.gap;
    var horizontal = side === "right" || side === "left";
    var candidates = [side, TOOLTIP_OPPOSITE[side]].concat(horizontal ? ["bottom", "top"] : ["right", "left"]);
    var positions = {
      right: { left: anchor.right + 10, top: clampY((anchor.top + anchor.bottom - height) / 2) },
      left: { left: anchor.left - 10 - width, top: clampY((anchor.top + anchor.bottom - height) / 2) },
      bottom: {
        left: clampX(align === "end" ? anchor.right - width : (anchor.left + anchor.right - width) / 2),
        top: anchor.bottom + gap,
      },
      top: {
        left: clampX(align === "end" ? anchor.right - width : (anchor.left + anchor.right - width) / 2),
        top: anchor.top - gap - height,
      },
    };
    for (var index = 0; index < candidates.length; index += 1) {
      var candidate = candidates[index];
      var position = positions[candidate];
      if (
        position.left >= margin &&
        position.top >= margin &&
        position.left + width <= viewportWidth - margin &&
        position.top + height <= viewportHeight - margin
      ) {
        return { left: position.left, top: position.top, side: candidate, width: width, height: height, overflow: false };
      }
    }
    var fallback = positions[side];
    return { left: clampX(fallback.left), top: clampY(fallback.top), side: side, width: width, height: height, overflow: true };
  }

  // ----- ShortcutKeys (reference ShortcutKeys.tsx) --------------------------
  function ShortcutKeys(props) {
    var keys = Array.isArray(props.keys) ? props.keys : [];
    var tooltip = props.variant === "tooltip";
    return h(
      "span",
      {
        className: classNames(
          "dshr-shortcut-keys",
          tooltip && "dshr-shortcut-keys-tooltip",
          tooltip && keys.indexOf("+") !== -1 && "dshr-shortcut-keys-joined",
          props.className,
        ),
      },
      keys.map(function (key, index) {
        return h("kbd", { key: index, className: key === "+" ? "dshr-shortcut-separator" : "dshr-shortcut-key" }, key);
      }),
    );
  }

  // ----- Tooltip (reference Tooltip.tsx) ------------------------------------
  var TooltipSuppression = React.createContext(null);

  function tooltipClipped(anchor) {
    var nodes = [anchor].concat(Array.prototype.slice.call(anchor.querySelectorAll("*")));
    return nodes.some(function (node) {
      return node instanceof HTMLElement && node.scrollWidth > node.clientWidth;
    });
  }

  function Tooltip(props) {
    var id = React.useId ? React.useId() : "dshr-tooltip";
    var label = props.label !== undefined ? props.label : props.content;
    var shortcutKeys = props.shortcutKeys;
    var side = props.side || "right";
    var align = props.align || "center";
    var delayMs = typeof props.delayMs === "number" ? props.delayMs : 0;
    var focusDelayMs = typeof props.focusDelayMs === "number" ? props.focusDelayMs : 0;
    var gap = typeof props.gap === "number" ? props.gap : 8;
    var disabled = props.disabled === true;
    var portal = props.portal === true;
    var maxWidth = props.maxWidth;
    var openOnClick = props.openOnClick === true;
    var onlyWhenClipped = props.onlyWhenClipped === true;
    var pinnedPair = React.useState(false);
    var pinned = pinnedPair[0];
    var setPinned = pinnedPair[1];
    var anchor = React.useRef(null);
    var positionPair = React.useState(null);
    var pos = positionPair[0];
    var setPos = positionPair[1];
    var bubble = React.useRef(null);
    var showTimer = React.useRef(null);
    var triggers = React.useRef({ hover: false, focus: false });
    var suppressAncestors = React.useContext(TooltipSuppression);
    var suppressedPair = React.useState(false);
    var suppressed = suppressedPair[0];
    var setSuppressed = suppressedPair[1];
    var announce = React.useCallback(
      function (active) {
        if (typeof suppressAncestors === "function") suppressAncestors(active);
      },
      [suppressAncestors],
    );
    var visible = pos !== null && !disabled;
    var resolvedLabel = pos === null ? null : typeof label === "function" ? label() : label;
    var childRef = props.children ? props.children.ref : undefined;
    var mergedRef = React.useCallback(
      function (el) {
        anchor.current = el;
        if (typeof childRef === "function") childRef(el);
        else if (childRef != null) childRef.current = el;
      },
      [childRef],
    );
    React.useEffect(function () {
      var el = bubble.current;
      if (pos === null || !visible || suppressed || el === null) return undefined;
      var size;
      var anchorRect = pos;
      function fit() {
        if (size === undefined) return;
        var placement = placeTooltip({
          anchor: anchorRect,
          width: size.inlineSize,
          height: size.blockSize,
          viewportWidth: window.innerWidth,
          viewportHeight: window.innerHeight,
          side: side,
          align: align,
          gap: gap,
        });
        var horizontal = placement.side === "right" || placement.side === "left";
        var offsetX = horizontal ? 0 : align === "end" ? placement.width : placement.width / 2;
        var offsetY = horizontal ? placement.height / 2 : placement.side === "top" ? placement.height : 0;
        el.style.left = placement.left + offsetX + "px";
        el.style.top = placement.top + offsetY + "px";
        el.dataset.side = placement.side;
        el.style.visibility = "visible";
      }
      function onResize() {
        if (anchor.current !== null) anchorRect = anchor.current.getBoundingClientRect();
        fit();
      }
      var observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(function (entries) {
        var entry = entries[0];
        if (entry && entry.borderBoxSize && entry.borderBoxSize[0]) size = entry.borderBoxSize[0];
        else if (entry) size = { inlineSize: entry.contentRect.width, blockSize: entry.contentRect.height };
        fit();
      });
      if (observer) observer.observe(el, { box: "border-box" });
      window.addEventListener("resize", onResize);
      return function () {
        if (observer) observer.disconnect();
        window.removeEventListener("resize", onResize);
      };
    }, [align, gap, pos, side, suppressed, visible]);
    React.useEffect(function () {
      announce(visible);
      return function () {
        announce(false);
      };
    }, [announce, visible]);
    var cancelShow = React.useCallback(function () {
      if (showTimer.current === null) return;
      clearTimeout(showTimer.current);
      showTimer.current = null;
    }, []);
    React.useEffect(function () {
      if (pinned && (disabled || !openOnClick)) setPinned(false);
      if (disabled) {
        cancelShow();
        triggers.current = { hover: false, focus: false };
        setPos(null);
      }
      return cancelShow;
    }, [cancelShow, disabled, openOnClick, pinned]);
    function show() {
      if (disabled) return;
      var el = anchor.current;
      if (el === null) return;
      if (onlyWhenClipped && !tooltipClipped(el)) return;
      setPos(el.getBoundingClientRect());
      announce(true);
    }
    function showAfterDelay(delay) {
      cancelShow();
      if (delay <= 0) {
        show();
        return;
      }
      showTimer.current = setTimeout(function () {
        showTimer.current = null;
        show();
      }, delay);
    }
    var withdraw = React.useCallback(function () {
      setPinned(false);
      setPos(null);
      announce(false);
    }, [announce]);
    function hide() {
      cancelShow();
      if (!triggers.current.hover && !triggers.current.focus && !pinned) withdraw();
    }
    var dismiss = React.useCallback(function () {
      cancelShow();
      triggers.current = { hover: false, focus: false };
      withdraw();
    }, [cancelShow, withdraw]);
    useDismissOnOutsidePointer(anchor, openOnClick && visible, dismiss, bubble);
    React.useEffect(function () {
      if (!openOnClick || !visible) return undefined;
      function onKeyDown(event) {
        if (event.key !== "Escape" && event.key !== "Tab") return;
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
        }
        dismiss();
      }
      document.addEventListener("keydown", onKeyDown, true);
      return function () {
        document.removeEventListener("keydown", onKeyDown, true);
      };
    }, [dismiss, openOnClick, visible]);
    var content = null;
    if (visible && !suppressed) {
      content = h(
        "span",
        {
          ref: bubble,
          id: openOnClick ? id : undefined,
          className: "dshr-tooltip-bubble",
          "data-side": side,
          "data-portal": portal ? "" : undefined,
          "data-pinned": pinned ? "" : undefined,
          "data-align": align,
          "data-has-shortcut": shortcutKeys && shortcutKeys.length ? "" : undefined,
          style: Object.assign(
            { left: pos.left, top: pos.top, visibility: "hidden" },
            maxWidth === undefined ? {} : { maxWidth: "min(" + maxWidth + "px, calc(100vw - 24px))" },
          ),
          role: "tooltip",
          "aria-label":
            shortcutKeys && shortcutKeys.length
              ? [resolvedLabel, shortcutKeys.join(" ")].filter(Boolean).join(" ")
              : undefined,
        },
        resolvedLabel ? h("span", { className: "dshr-tooltip-label" }, resolvedLabel) : null,
        shortcutKeys !== undefined && shortcutKeys.length > 0
          ? h(ShortcutKeys, { keys: shortcutKeys, variant: "tooltip" })
          : null,
      );
    }
    var anchorElement = React.cloneElement(props.children, {
      ref: mergedRef,
      "aria-describedby":
        openOnClick && visible
          ? [props.children.props["aria-describedby"], id].filter(Boolean).join(" ")
          : props.children.props["aria-describedby"],
      onPointerEnter: function (event) {
        if (props.children.props.onPointerEnter) props.children.props.onPointerEnter(event);
        if (event.pointerType === "touch") return;
        if (
          event.pointerType === "mouse" &&
          typeof window.matchMedia === "function" &&
          !window.matchMedia("(any-hover: hover)").matches
        ) {
          return;
        }
        triggers.current.hover = true;
        showAfterDelay(delayMs);
      },
      onPointerLeave: function (event) {
        if (props.children.props.onPointerLeave) props.children.props.onPointerLeave(event);
        triggers.current.hover = false;
        cancelShow();
        if (!pinned) withdraw();
      },
      onPointerDown: function (event) {
        if (props.children.props.onPointerDown) props.children.props.onPointerDown(event);
        if (event.pointerType === "touch" && !pinned) dismiss();
      },
      onClick: function (event) {
        if (props.children.props.onClick) props.children.props.onClick(event);
        triggers.current.focus = false;
        cancelShow();
        if (openOnClick && !disabled && !pinned) {
          setPinned(true);
          show();
        } else {
          withdraw();
        }
      },
      onFocus: function (event) {
        if (props.children.props.onFocus) props.children.props.onFocus(event);
        if (pointerModality()) return;
        triggers.current.focus = true;
        showAfterDelay(focusDelayMs);
      },
      onBlur: function (event) {
        if (props.children.props.onBlur) props.children.props.onBlur(event);
        triggers.current.focus = false;
        hide();
      },
    });
    var renderedBubble = content;
    if (content !== null) {
      renderedBubble = portal && ReactDOM ? ReactDOM.createPortal(content, document.body) : content;
    }
    return h(
      TooltipSuppression.Provider,
      { value: setSuppressed },
      anchorElement,
      renderedBubble,
    );
  }

  // ----- JsonBlock (reference markdown/JsonBlock.tsx) ------------------------
  var JSON_BLOCK_MAX_CHARS = 20000;

  function JsonBlock(props) {
    var openPair = React.useState(props.defaultOpen === true);
    var open = openPair[0];
    var setOpen = openPair[1];
    var body = "";
    if (open) {
      var serialized;
      try {
        serialized = JSON.stringify(props.payload, null, 2);
        if (serialized === undefined) serialized = String(props.payload);
      } catch (error) {
        serialized = String(props.payload);
      }
      body =
        serialized.length > JSON_BLOCK_MAX_CHARS
          ? serialized.slice(0, JSON_BLOCK_MAX_CHARS) +
            "\n" +
            (typeof props.truncatedLabel === "function"
              ? props.truncatedLabel(serialized.length)
              : "truncated")
          : serialized;
    }
    return h(
      "div",
      { className: classNames("dshr-json-block-root", props.className) },
      h(
        "button",
        {
          type: "button",
          className: "dshr-json-block-toggle",
          onClick: function () {
            setOpen(!open);
          },
        },
        (open ? "▾ " : "▸ ") + props.label,
      ),
      open ? h("pre", { className: "dshr-json-block-body" }, body) : null,
    );
  }

  // ----- MarkdownText -------------------------------------------------------
  // dshr renders a dependency-free GFM subset with the reference prop surface
  // (text/labels/variant/omitHtml/textInteractionSource). The upstream stack
  // (micromark/mdast, shiki highlighting, KaTeX, footnotes) is not bundled; see
  // the module header.
  function sanitizeHref(url) {
    // Reference sanitizeUrl: absolute http(s)/mailto only; relative and other
    // protocols render as plain text (no opener vocabulary in dshr).
    try {
      var parsed = new URL(String(url || ""));
      if (parsed.protocol === "http:" || parsed.protocol === "https:" || parsed.protocol === "mailto:") {
        return parsed.href;
      }
      return null;
    } catch (error) {
      return null;
    }
  }

  function isAbsoluteHttp(url) {
    return /^https?:\/\//i.test(String(url || ""));
  }

  var INLINE_TOKEN = /(`[^`]*`|\*\*[^*]+\*\*|__[^_]+__|\*[^*]+\*|_[^_]+_|~~[^~]+~~|!\[[^\]]*\]\([^)]*\)|\[[^\]]*\]\([^)]*\)|<[^>]+>)/g;

  function renderInline(source, keyPrefix) {
    var textValue = String(source === undefined || source === null ? "" : source);
    var parts = textValue.split(INLINE_TOKEN);
    return parts.map(function (part, index) {
      if (part === undefined || part === "") return null;
      var key = keyPrefix + "i" + index;
      if (part.charAt(0) === "`" && part.length > 1 && part.charAt(part.length - 1) === "`") {
        return h("code", { key: key }, part.slice(1, -1));
      }
      if (/^\*\*[\s\S]+\*\*$/.test(part) || /^__[\s\S]+__$/.test(part)) {
        return h("strong", { key: key }, renderInline(part.slice(2, -2), key + "-"));
      }
      if (/^\*[\s\S]+\*$/.test(part) || /^_[\s\S]+_$/.test(part)) {
        return h("em", { key: key }, renderInline(part.slice(1, -1), key + "-"));
      }
      if (/^~~[\s\S]+~~$/.test(part)) {
        return h("del", { key: key }, renderInline(part.slice(2, -2), key + "-"));
      }
      var image = /^!\[([^\]]*)\]\(([^)]*)\)$/.exec(part);
      if (image) {
        if (isAbsoluteHttp(image[2])) {
          return h("img", { key: key, src: image[2], alt: image[1], className: "dshr-markdown-image" });
        }
        return image[1] || null;
      }
      var link = /^\[([^\]]*)\]\(([^)]*)\)$/.exec(part);
      if (link) {
        var target = sanitizeHref(link[2]);
        if (target === null) return h(React.Fragment, { key: key }, link[1]);
        return h(
          "a",
          { key: key, href: target, target: "_blank", rel: "noreferrer noopener" },
          link[1],
        );
      }
      if (part.charAt(0) === "<" && part.charAt(part.length - 1) === ">") {
        return null;
      }
      return part;
    });
  }

  function markdownBlocks(source) {
    var lines = String(source === undefined || source === null ? "" : source).replace(/\r\n?/g, "\n").split("\n");
    var blocks = [];
    var index = 0;
    var keySeq = 0;
    function nextKey() {
      keySeq += 1;
      return "md" + keySeq;
    }
    while (index < lines.length) {
      var line = lines[index];
      if (line.trim() === "") {
        index += 1;
        continue;
      }
      var fence = /^```([A-Za-z0-9_+-]*)\s*$/.exec(line);
      if (fence) {
        var language = fence[1];
        var code = [];
        index += 1;
        while (index < lines.length && !/^```\s*$/.test(lines[index])) {
          code.push(lines[index]);
          index += 1;
        }
        if (index < lines.length) index += 1;
        blocks.push(
          h("pre", { key: nextKey(), className: "dshr-markdown-code", "data-language": language || undefined },
            h("code", null, code.join("\n"))),
        );
        continue;
      }
      var heading = /^(#{1,6})\s+(.*)$/.exec(line);
      if (heading) {
        var level = heading[1].length;
        blocks.push(h("h" + level, { key: nextKey() }, renderInline(heading[2], nextKey())));
        index += 1;
        continue;
      }
      if (/^(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
        blocks.push(h("hr", { key: nextKey() }));
        index += 1;
        continue;
      }
      if (/^>\s?/.test(line)) {
        var quote = [];
        while (index < lines.length && /^>\s?/.test(lines[index])) {
          quote.push(lines[index].replace(/^>\s?/, ""));
          index += 1;
        }
        blocks.push(h("blockquote", { key: nextKey() }, markdownBlocks(quote.join("\n"))));
        continue;
      }
      if (/^[-*+]\s+/.test(line)) {
        var unordered = [];
        while (index < lines.length && /^[-*+]\s+/.test(lines[index])) {
          unordered.push(lines[index].replace(/^[-*+]\s+/, ""));
          index += 1;
        }
        blocks.push(
          h("ul", { key: nextKey() }, unordered.map(function (item, itemIndex) {
            return h("li", { key: "li" + itemIndex }, renderInline(item, "u" + itemIndex));
          })),
        );
        continue;
      }
      if (/^\d+\.\s+/.test(line)) {
        var ordered = [];
        while (index < lines.length && /^\d+\.\s+/.test(lines[index])) {
          ordered.push(lines[index].replace(/^\d+\.\s+/, ""));
          index += 1;
        }
        blocks.push(
          h("ol", { key: nextKey() }, ordered.map(function (item, itemIndex) {
            return h("li", { key: "li" + itemIndex }, renderInline(item, "o" + itemIndex));
          })),
        );
        continue;
      }
      var paragraph = [];
      while (
        index < lines.length &&
        lines[index].trim() !== "" &&
        !/^(#{1,6}\s|```|>\s?|[-*+]\s+|\d+\.\s+|(-{3,}|\*{3,}|_{3,})\s*$)/.test(lines[index])
      ) {
        paragraph.push(lines[index]);
        index += 1;
      }
      var key = nextKey();
      blocks.push(
        h("p", { key: key }, renderInline(paragraph.join("\n"), key + "-")),
      );
    }
    return blocks;
  }

  function MarkdownText(props) {
    var variant = props.variant || "body";
    return h(
      "div",
      {
        className: classNames("dshr-markdown", variant === "compact" && "dshr-markdown-compact", props.className),
        "data-text-interaction-source": props.textInteractionSource ? "" : undefined,
        "data-markdown-variant": variant === "compact" ? "compact" : undefined,
      },
      markdownBlocks(props.text),
    );
  }

  // ----- Extensions kept by dshr (not part of the reference export set) ------
  function Spinner(props) {
    return h("span", { className: classNames("dshr-spinner", props && props.className), role: "status", "aria-label": (props && props.label) || "loading" });
  }

  async function writeClipboard(text) {
    var value = String(text === undefined ? "" : text);
    if (typeof navigator !== "undefined" && navigator.clipboard && typeof navigator.clipboard.writeText === "function") {
      try {
        await navigator.clipboard.writeText(value);
        return true;
      } catch (error) {
        // fall through to execCommand
      }
    }
    var exec = typeof document !== "undefined" && typeof document.execCommand === "function" ? document.execCommand.bind(document) : undefined;
    if (exec === undefined) return false;
    var focused = document.activeElement;
    var area = document.createElement("textarea");
    area.value = value;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.left = "-9999px";
    try {
      document.body.appendChild(area);
      area.select();
      return exec("copy");
    } catch (error) {
      return false;
    } finally {
      area.remove();
      if (focused && typeof focused.focus === "function") focused.focus({ preventScroll: true });
    }
  }

  // dshr extension: the superagent client probes this hook before using its own
  // popover fallback (the reference package has no such export).
  function useAnchoredPosition(options) {
    var opts = options || {};
    var pair = React.useState(null);
    var position = pair[0];
    var setPosition = pair[1];
    React.useLayoutEffect(function () {
      if (!opts.open) {
        setPosition(null);
        return undefined;
      }
      function compute() {
        var anchor = opts.anchorRef && opts.anchorRef.current;
        var panel = opts.panelRef && opts.panelRef.current;
        if (!anchor) {
          setPosition(null);
          return;
        }
        var rect = anchor.getBoundingClientRect();
        var gap = opts.gap === undefined ? 6 : opts.gap;
        var margin = opts.margin === undefined ? 8 : opts.margin;
        var width = panel ? panel.offsetWidth : 0;
        var height = panel ? panel.offsetHeight : 0;
        var left = opts.align === "end" ? rect.right - width : rect.left;
        var top = opts.side === "top" ? rect.top - height - gap : rect.bottom + gap;
        left = Math.max(margin, Math.min(left, window.innerWidth - width - margin));
        top = Math.max(margin, Math.min(top, window.innerHeight - height - margin));
        setPosition({ left: left, top: top });
      }
      compute();
      window.addEventListener("resize", compute);
      window.addEventListener("scroll", compute, true);
      return function () {
        window.removeEventListener("resize", compute);
        window.removeEventListener("scroll", compute, true);
      };
    }, [opts.open, opts.anchorRef, opts.panelRef, opts.side, opts.align, opts.gap, opts.margin]);
    return position;
  }

  // Reference signature: useDismissOnOutsidePointer(root, open, setOpen, portal).
  function useDismissOnOutsidePointer(root, open, setOpen, portal) {
    var callbackRef = React.useRef(setOpen);
    callbackRef.current = setOpen;
    React.useEffect(function () {
      if (!open) return undefined;
      function closeOutside(event) {
        var target = event.target;
        if (root && root.current && root.current.contains(target)) return;
        if (portal && portal.current && portal.current.contains(target)) return;
        if (typeof callbackRef.current === "function") callbackRef.current(false);
      }
      document.addEventListener("pointerdown", closeOutside);
      return function () {
        document.removeEventListener("pointerdown", closeOutside);
      };
    }, [root, open, portal]);
  }

  // Generated from the reference repository artwork:
  // packages/client/ui-primitives/src/icons/{artwork.ts,index.tsx}
  // (deepseek-ai/deepseek-harness @ 39163dd). The reference renders <use>
  // against a document sprite; dshr inlines the same artwork per icon.
  var ICON_ART = {"layers-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M8.481371 2.304371 a1.159971 1.159971 0 0 0-.962742 0 L2.547971 4.566371 a.580029 .580029 0 0 0 0 1.0614 l4.9764 2.267829 a1.159971 1.159971 0 0 0 .962829 0 l4.9764-2.262 a.580029 .580029 0 0 0 0-1.0614Z\" stroke=\"currentColor\"></path><path d=\"M2.199971 8 a.580029 .580029 0 0 0 .336429 .527829 l4.987971 2.267742 a1.159971 1.159971 0 0 0 .957 0 l4.9764-2.262 A.580029 .580029 0 0 0 13.800029 8\" stroke=\"currentColor\"></path><path d=\"M2.199971 10.899971 a.580029 .580029 0 0 0 .336429 .527829 l4.987971 2.267829 a1.159971 1.159971 0 0 0 .957 0 l4.9764-2.262 A.580029 .580029 0 0 0 13.800029 10.899971\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M8.481371 2.304371 a1.159971 1.159971 0 0 0-.962742 0 L2.547971 4.566371 a.580029 .580029 0 0 0 0 1.0614 l4.9764 2.267829 a1.159971 1.159971 0 0 0 .962829 0 l4.9764-2.262 a.580029 .580029 0 0 0 0-1.0614Z\" stroke=\"currentColor\"></path><path d=\"M2.199971 8 a.580029 .580029 0 0 0 .336429 .527829 l4.987971 2.267742 a1.159971 1.159971 0 0 0 .957 0 l4.9764-2.262 A.580029 .580029 0 0 0 13.800029 8\" stroke=\"currentColor\"></path><path d=\"M2.199971 10.899971 a.580029 .580029 0 0 0 .336429 .527829 l4.987971 2.267829 a1.159971 1.159971 0 0 0 .957 0 l4.9764-2.262 A.580029 .580029 0 0 0 13.800029 10.899971\" stroke=\"currentColor\"></path></svg>"},"new-chat-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.37091 11.2501C1.58745 9.89288 1.32067 8.29835 1.61969 6.76006C1.91872 5.22177 2.76342 3.8433 3.99826 2.87846C5.2331 1.91362 6.77494 1.42737 8.33988 1.50925C9.90482 1.59113 11.3875 2.23562 12.5149 3.32406C13.6425 4.41269 14.3387 5.87206 14.4754 7.4334C14.612 8.99474 14.18 10.5529 13.2587 11.8209C12.3375 13.0888 10.9891 13.9813 9.46194 14.3337C8.18691 14.628 6.85895 14.5294 5.64989 14.0605C5.1712 13.8748 4.76962 13.4932 4.26534 13.3967C3.67413 13.2835 2.95257 13.5598 2.03794 14.3337\" stroke=\"currentColor\"></path><path d=\"M8 5V11\" stroke=\"currentColor\"></path><path d=\"M5 8H11\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.37091 11.2501C1.58745 9.89288 1.32067 8.29835 1.61969 6.76006C1.91872 5.22177 2.76342 3.8433 3.99826 2.87846C5.2331 1.91362 6.77494 1.42737 8.33988 1.50925C9.90482 1.59113 11.3875 2.23562 12.5149 3.32406C13.6425 4.41269 14.3387 5.87206 14.4754 7.4334C14.612 8.99474 14.18 10.5529 13.2587 11.8209C12.3375 13.0888 10.9891 13.9813 9.46194 14.3337C8.18691 14.628 6.85895 14.5294 5.64989 14.0605C5.1712 13.8748 4.76962 13.4932 4.26534 13.3967C3.67413 13.2835 2.95257 13.5598 2.03794 14.3337\" stroke=\"currentColor\"></path><path d=\"M8 5V11\" stroke=\"currentColor\"></path><path d=\"M5 8H11\" stroke=\"currentColor\"></path></svg>"},"search-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.58727 11.8586C9.55061 11.8586 11.9529 9.45637 11.9529 6.49304C11.9529 3.5297 9.55061 1.12744 6.58727 1.12744C3.62394 1.12744 1.22168 3.5297 1.22168 6.49304C1.22168 9.45637 3.62394 11.8586 6.58727 11.8586Z\" stroke=\"currentColor\"></path><path d=\"M10.2991 10.3933L14.7783 14.8725\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.58727 11.8586C9.55061 11.8586 11.9529 9.45637 11.9529 6.49304C11.9529 3.5297 9.55061 1.12744 6.58727 1.12744C3.62394 1.12744 1.22168 3.5297 1.22168 6.49304C1.22168 9.45637 3.62394 11.8586 6.58727 11.8586Z\" stroke=\"currentColor\"></path><path d=\"M10.2991 10.3933L14.7783 14.8725\" stroke=\"currentColor\"></path></svg>"},"eye-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M1.5 8C3.36 4.41 5.52 2.63 8 2.63C10.48 2.63 12.64 4.41 14.5 8C12.64 11.59 10.48 13.37 8 13.37C5.52 13.37 3.36 11.59 1.5 8Z\" stroke=\"currentColor\"></path><circle cx=\"8\" cy=\"8\" r=\"2.25\" stroke=\"currentColor\"></circle></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M1.5 8C3.36 4.41 5.52 2.63 8 2.63C10.48 2.63 12.64 4.41 14.5 8C12.64 11.59 10.48 13.37 8 13.37C5.52 13.37 3.36 11.59 1.5 8Z\" stroke=\"currentColor\"></path><circle cx=\"8\" cy=\"8\" r=\"2.25\" stroke=\"currentColor\"></circle></svg>"},"eye-off-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.25 2.25L13.75 13.75\" stroke=\"currentColor\"></path><path d=\"M6.35 3.03C6.88 2.76 7.42 2.63 8 2.63C10.48 2.63 12.64 4.41 14.5 8C13.75 9.48 12.95 10.6 12.08 11.36\" stroke=\"currentColor\"></path><path d=\"M10.28 12.81C9.54 13.19 8.78 13.37 8 13.37C5.52 13.37 3.36 11.59 1.5 8C2.26 6.49 3.08 5.36 3.97 4.6\" stroke=\"currentColor\"></path><path d=\"M6.55 6.55A2.05 2.05 0 0 0 9.45 9.45\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.25 2.25L13.75 13.75\" stroke=\"currentColor\"></path><path d=\"M6.35 3.03C6.88 2.76 7.42 2.63 8 2.63C10.48 2.63 12.64 4.41 14.5 8C13.75 9.48 12.95 10.6 12.08 11.36\" stroke=\"currentColor\"></path><path d=\"M10.28 12.81C9.54 13.19 8.78 13.37 8 13.37C5.52 13.37 3.36 11.59 1.5 8C2.26 6.49 3.08 5.36 3.97 4.6\" stroke=\"currentColor\"></path><path d=\"M6.55 6.55A2.05 2.05 0 0 0 9.45 9.45\" stroke=\"currentColor\"></path></svg>"},"globe-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M7.99986 14.0887C11.3626 14.0887 14.0886 11.3627 14.0886 7.99998C14.0886 4.63727 11.3626 1.91125 7.99986 1.91125C4.63715 1.91125 1.91113 4.63727 1.91113 7.99998C1.91113 11.3627 4.63715 14.0887 7.99986 14.0887Z\" stroke=\"currentColor\"></path><path d=\"M2.34619 8H13.6538\" stroke=\"currentColor\" stroke-linecap=\"square\"></path><path d=\"M7.99976 14.0889C9.23509 14.0889 10.1743 11.3629 10.1743 8.00006C10.1743 4.63739 9.23509 1.91138 7.99976 1.91138\" stroke=\"currentColor\"></path><path d=\"M7.99973 14.0889C6.76445 14.0889 5.8252 11.3629 5.8252 8.00006C5.8252 4.63739 6.76445 1.91138 7.99973 1.91138\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M7.99986 14.0887C11.3626 14.0887 14.0886 11.3627 14.0886 7.99998C14.0886 4.63727 11.3626 1.91125 7.99986 1.91125C4.63715 1.91125 1.91113 4.63727 1.91113 7.99998C1.91113 11.3627 4.63715 14.0887 7.99986 14.0887Z\" stroke=\"currentColor\"></path><path d=\"M2.34619 8H13.6538\" stroke=\"currentColor\" stroke-linecap=\"square\"></path><path d=\"M7.99976 14.0889C9.23509 14.0889 10.1743 11.3629 10.1743 8.00006C10.1743 4.63739 9.23509 1.91138 7.99976 1.91138\" stroke=\"currentColor\"></path><path d=\"M7.99973 14.0889C6.76445 14.0889 5.8252 11.3629 5.8252 8.00006C5.8252 4.63739 6.76445 1.91138 7.99973 1.91138\" stroke=\"currentColor\"></path></svg>"},"settings-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 9.75012C8.9665 9.75012 9.75 8.96662 9.75 8.00012C9.75 7.03362 8.9665 6.25012 8 6.25012C7.0335 6.25012 6.25 7.03362 6.25 8.00012C6.25 8.96662 7.0335 9.75012 8 9.75012Z\" stroke=\"currentColor\"></path><path d=\"M13.0107 7.79377C12.9505 7.89401 12.9205 7.94413 12.9205 7.99951C12.9205 8.0549 12.9505 8.10502 13.0106 8.20528L13.9849 9.83006C14.045 9.93029 14.0751 9.9804 14.0751 10.0358C14.0751 10.0911 14.045 10.1413 13.9849 10.2415L13.0037 11.8777C12.9468 11.9726 12.9184 12.0201 12.8725 12.0461C12.8267 12.072 12.7713 12.072 12.6607 12.072H10.6704C10.5598 12.072 10.5045 12.072 10.4586 12.098C10.4128 12.1239 10.3843 12.1714 10.3274 12.2662L9.33825 13.9142C9.28133 14.009 9.25287 14.0564 9.20703 14.0823C9.16118 14.1083 9.10588 14.1083 8.99529 14.1083H7.00486C6.89426 14.1083 6.83896 14.1083 6.79312 14.0823C6.74727 14.0564 6.71881 14.009 6.6619 13.9142L5.67273 12.2662C5.61581 12.1714 5.58735 12.1239 5.54151 12.098C5.49566 12.072 5.44036 12.072 5.32977 12.072H3.33945C3.2288 12.072 3.17347 12.072 3.12761 12.0461C3.08176 12.0201 3.0533 11.9726 2.9964 11.8777L2.0152 10.2415C1.9551 10.1413 1.92505 10.0911 1.92505 10.0358C1.92505 9.9804 1.9551 9.93029 2.0152 9.83006L2.98951 8.20528C3.04963 8.10502 3.07969 8.0549 3.07969 7.99951C3.07968 7.94413 3.04961 7.89401 2.98946 7.79377L2.01529 6.17011C1.95514 6.06987 1.92507 6.01975 1.92507 5.96437C1.92506 5.90899 1.95512 5.85886 2.01524 5.7586L2.9964 4.1224C3.0533 4.0275 3.08176 3.98005 3.12761 3.95408C3.17347 3.92811 3.2288 3.92811 3.33945 3.92811H5.32977C5.44036 3.92811 5.49566 3.92811 5.54151 3.90216C5.58735 3.87621 5.61581 3.82879 5.67273 3.73397L6.6619 2.08599C6.71881 1.99116 6.74727 1.94375 6.79312 1.9178C6.83896 1.89185 6.89426 1.89185 7.00486 1.89185H8.99529C9.10588 1.89185 9.16118 1.89185 9.20703 1.9178C9.25287 1.94375 9.28133 1.99116 9.33825 2.08599L10.3274 3.73397C10.3843 3.82879 10.4128 3.87621 10.4586 3.90216C10.5045 3.92811 10.5598 3.92811 10.6704 3.92811H12.6607C12.7713 3.92811 12.8267 3.92811 12.8725 3.95408C12.9184 3.98005 12.9468 4.0275 13.0037 4.1224L13.9849 5.7586C14.045 5.85886 14.0751 5.90899 14.0751 5.96437C14.0751 6.01975 14.045 6.06987 13.9849 6.17011L13.0107 7.79377Z\" stroke=\"currentColor\" stroke-miterlimit=\"10\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 9.75012C8.9665 9.75012 9.75 8.96662 9.75 8.00012C9.75 7.03362 8.9665 6.25012 8 6.25012C7.0335 6.25012 6.25 7.03362 6.25 8.00012C6.25 8.96662 7.0335 9.75012 8 9.75012Z\" stroke=\"currentColor\"></path><path d=\"M13.0107 7.79377C12.9505 7.89401 12.9205 7.94413 12.9205 7.99951C12.9205 8.0549 12.9505 8.10502 13.0106 8.20528L13.9849 9.83006C14.045 9.93029 14.0751 9.9804 14.0751 10.0358C14.0751 10.0911 14.045 10.1413 13.9849 10.2415L13.0037 11.8777C12.9468 11.9726 12.9184 12.0201 12.8725 12.0461C12.8267 12.072 12.7713 12.072 12.6607 12.072H10.6704C10.5598 12.072 10.5045 12.072 10.4586 12.098C10.4128 12.1239 10.3843 12.1714 10.3274 12.2662L9.33825 13.9142C9.28133 14.009 9.25287 14.0564 9.20703 14.0823C9.16118 14.1083 9.10588 14.1083 8.99529 14.1083H7.00486C6.89426 14.1083 6.83896 14.1083 6.79312 14.0823C6.74727 14.0564 6.71881 14.009 6.6619 13.9142L5.67273 12.2662C5.61581 12.1714 5.58735 12.1239 5.54151 12.098C5.49566 12.072 5.44036 12.072 5.32977 12.072H3.33945C3.2288 12.072 3.17347 12.072 3.12761 12.0461C3.08176 12.0201 3.0533 11.9726 2.9964 11.8777L2.0152 10.2415C1.9551 10.1413 1.92505 10.0911 1.92505 10.0358C1.92505 9.9804 1.9551 9.93029 2.0152 9.83006L2.98951 8.20528C3.04963 8.10502 3.07969 8.0549 3.07969 7.99951C3.07968 7.94413 3.04961 7.89401 2.98946 7.79377L2.01529 6.17011C1.95514 6.06987 1.92507 6.01975 1.92507 5.96437C1.92506 5.90899 1.95512 5.85886 2.01524 5.7586L2.9964 4.1224C3.0533 4.0275 3.08176 3.98005 3.12761 3.95408C3.17347 3.92811 3.2288 3.92811 3.33945 3.92811H5.32977C5.44036 3.92811 5.49566 3.92811 5.54151 3.90216C5.58735 3.87621 5.61581 3.82879 5.67273 3.73397L6.6619 2.08599C6.71881 1.99116 6.74727 1.94375 6.79312 1.9178C6.83896 1.89185 6.89426 1.89185 7.00486 1.89185H8.99529C9.10588 1.89185 9.16118 1.89185 9.20703 1.9178C9.25287 1.94375 9.28133 1.99116 9.33825 2.08599L10.3274 3.73397C10.3843 3.82879 10.4128 3.87621 10.4586 3.90216C10.5045 3.92811 10.5598 3.92811 10.6704 3.92811H12.6607C12.7713 3.92811 12.8267 3.92811 12.8725 3.95408C12.9184 3.98005 12.9468 4.0275 13.0037 4.1224L13.9849 5.7586C14.045 5.85886 14.0751 5.90899 14.0751 5.96437C14.0751 6.01975 14.045 6.06987 13.9849 6.17011L13.0107 7.79377Z\" stroke=\"currentColor\" stroke-miterlimit=\"10\"></path></svg>"},"panel-left-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M13.5 1.5H2.5C1.94772 1.5 1.5 1.94772 1.5 2.5V13.5C1.5 14.0523 1.94772 14.5 2.5 14.5H13.5C14.0523 14.5 14.5 14.0523 14.5 13.5V2.5C14.5 1.94772 14.0523 1.5 13.5 1.5Z\" stroke=\"currentColor\"></path><path d=\"M5.5 1.5V14.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M13.5 1.5H2.5C1.94772 1.5 1.5 1.94772 1.5 2.5V13.5C1.5 14.0523 1.94772 14.5 2.5 14.5H13.5C14.0523 14.5 14.5 14.0523 14.5 13.5V2.5C14.5 1.94772 14.0523 1.5 13.5 1.5Z\" stroke=\"currentColor\"></path><path d=\"M5.5 1.5V14.5\" stroke=\"currentColor\"></path></svg>"},"ellipsis-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3 9C3.55228 9 4 8.55228 4 8C4 7.44772 3.55228 7 3 7C2.44772 7 2 7.44772 2 8C2 8.55228 2.44772 9 3 9Z\" fill=\"currentColor\"></path><path d=\"M8 9C8.55228 9 9 8.55228 9 8C9 7.44772 8.55228 7 8 7C7.44772 7 7 7.44772 7 8C7 8.55228 7.44772 9 8 9Z\" fill=\"currentColor\"></path><path d=\"M13 9C13.5523 9 14 8.55228 14 8C14 7.44772 13.5523 7 13 7C12.4477 7 12 7.44772 12 8C12 8.55228 12.4477 9 13 9Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3 9C3.55228 9 4 8.55228 4 8C4 7.44772 3.55228 7 3 7C2.44772 7 2 7.44772 2 8C2 8.55228 2.44772 9 3 9Z\" fill=\"currentColor\"></path><path d=\"M8 9C8.55228 9 9 8.55228 9 8C9 7.44772 8.55228 7 8 7C7.44772 7 7 7.44772 7 8C7 8.55228 7.44772 9 8 9Z\" fill=\"currentColor\"></path><path d=\"M13 9C13.5523 9 14 8.55228 14 8C14 7.44772 13.5523 7 13 7C12.4477 7 12 7.44772 12 8C12 8.55228 12.4477 9 13 9Z\" fill=\"currentColor\"></path></svg>"},"plus-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 2V14\" stroke=\"currentColor\"></path><path d=\"M2 8H14\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 2V14\" stroke=\"currentColor\"></path><path d=\"M2 8H14\" stroke=\"currentColor\"></path></svg>"},"check-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.25 8.5L5.49732 11.7473C5.90519 12.1552 6.57263 12.1344 6.95426 11.7018L13.75 4\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.25 8.5L5.49732 11.7473C5.90519 12.1552 6.57263 12.1344 6.95426 11.7018L13.75 4\" stroke=\"currentColor\"></path></svg>"},"mail-unread-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M8.5 3.5H3C2.44772 3.5 2 3.94772 2 4.5V11.5C2 12.0523 2.44772 12.5 3 12.5H13C13.5523 12.5 14 12.0523 14 11.5V7\" stroke=\"currentColor\"></path><path d=\"M2.5 4.5L8 8.5L10.5 6.7\" stroke=\"currentColor\"></path><circle cx=\"12.5\" cy=\"3.5\" r=\"1.5\" fill=\"currentColor\"></circle></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M8.5 3.5H3C2.44772 3.5 2 3.94772 2 4.5V11.5C2 12.0523 2.44772 12.5 3 12.5H13C13.5523 12.5 14 12.0523 14 11.5V7\" stroke=\"currentColor\"></path><path d=\"M2.5 4.5L8 8.5L10.5 6.7\" stroke=\"currentColor\"></path><circle cx=\"12.5\" cy=\"3.5\" r=\"1.5\" fill=\"currentColor\"></circle></svg>"},"mail-read-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M2 6.5L7.4 2.45C7.75556 2.18333 8.24444 2.18333 8.6 2.45L14 6.5V12.5C14 13.0523 13.5523 13.5 13 13.5H3C2.44772 13.5 2 13.0523 2 12.5V6.5Z\" stroke=\"currentColor\"></path><path d=\"M2.5 7L8 10.5L13.5 7M2.5 13L6 9.5M13.5 13L10 9.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M2 6.5L7.4 2.45C7.75556 2.18333 8.24444 2.18333 8.6 2.45L14 6.5V12.5C14 13.0523 13.5523 13.5 13 13.5H3C2.44772 13.5 2 13.0523 2 12.5V6.5Z\" stroke=\"currentColor\"></path><path d=\"M2.5 7L8 10.5L13.5 7M2.5 13L6 9.5M13.5 13L10 9.5\" stroke=\"currentColor\"></path></svg>"},"branch-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M1.01503 8.0001L5.6964 8.0001C6.41913 8.0001 6.78049 8.0001 7.12115 7.91951C7.4232 7.84804 7.71233 7.73014 7.97821 7.57C8.27809 7.38939 8.5364 7.13669 9.05303 6.63129L11.3281 4.40564\" stroke=\"currentColor\"></path><path d=\"M1.01221 7.9999L5.6964 7.9999C6.41913 7.9999 6.78049 7.9999 7.12115 8.08049C7.4232 8.15196 7.71233 8.26986 7.97821 8.43C8.27809 8.61061 8.5364 8.86331 9.05303 9.36871L11.3281 11.5944\" stroke=\"currentColor\"></path><circle cx=\"12.4502\" cy=\"3.3079\" r=\"1.56962\" stroke=\"currentColor\"></circle><circle cx=\"12.4502\" cy=\"12.6921\" r=\"1.56962\" stroke=\"currentColor\"></circle></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M1.01503 8.0001L5.6964 8.0001C6.41913 8.0001 6.78049 8.0001 7.12115 7.91951C7.4232 7.84804 7.71233 7.73014 7.97821 7.57C8.27809 7.38939 8.5364 7.13669 9.05303 6.63129L11.3281 4.40564\" stroke=\"currentColor\"></path><path d=\"M1.01221 7.9999L5.6964 7.9999C6.41913 7.9999 6.78049 7.9999 7.12115 8.08049C7.4232 8.15196 7.71233 8.26986 7.97821 8.43C8.27809 8.61061 8.5364 8.86331 9.05303 9.36871L11.3281 11.5944\" stroke=\"currentColor\"></path><circle cx=\"12.4502\" cy=\"3.3079\" r=\"1.56962\" stroke=\"currentColor\"></circle><circle cx=\"12.4502\" cy=\"12.6921\" r=\"1.56962\" stroke=\"currentColor\"></circle></svg>"},"chevron-down-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M4 6L7.29289 9.29289C7.68342 9.68342 8.31658 9.68342 8.70711 9.29289L12 6\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4 6L7.29289 9.29289C7.68342 9.68342 8.31658 9.68342 8.70711 9.29289L12 6\" stroke=\"currentColor\"></path></svg>"},"chevron-left-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M10 4L6.70711 7.29289C6.31658 7.68342 6.31658 8.31658 6.70711 8.70711L10 12\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M10 4L6.70711 7.29289C6.31658 7.68342 6.31658 8.31658 6.70711 8.70711L10 12\" stroke=\"currentColor\"></path></svg>"},"chevron-right-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6 12L9.29289 8.70711C9.68342 8.31658 9.68342 7.68342 9.29289 7.29289L6 4\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6 12L9.29289 8.70711C9.68342 8.31658 9.68342 7.68342 9.29289 7.29289L6 4\" stroke=\"currentColor\"></path></svg>"},"triangle-right-fill":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M5.5 4.5C5.5 4.40714 5.52586 4.31612 5.57467 4.23713C5.62349 4.15815 5.69334 4.09431 5.77639 4.05279C5.85945 4.01126 5.95242 3.99368 6.0449 4.00202C6.13738 4.01036 6.22572 4.04429 6.3 4.1L10.967 7.6C11.0291 7.64657 11.0795 7.70697 11.1142 7.77639C11.1489 7.84582 11.167 7.92238 11.167 8C11.167 8.07762 11.1489 8.15418 11.1142 8.22361C11.0795 8.29303 11.0291 8.35343 10.967 8.4L6.3 11.9C6.22572 11.9557 6.13738 11.9896 6.0449 11.998C5.95242 12.0063 5.85945 11.9887 5.77639 11.9472C5.69334 11.9057 5.62349 11.8419 5.57467 11.7629C5.52586 11.6839 5.5 11.5929 5.5 11.5V4.5Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M5.5 4.5C5.5 4.40714 5.52586 4.31612 5.57467 4.23713C5.62349 4.15815 5.69334 4.09431 5.77639 4.05279C5.85945 4.01126 5.95242 3.99368 6.0449 4.00202C6.13738 4.01036 6.22572 4.04429 6.3 4.1L10.967 7.6C11.0291 7.64657 11.0795 7.70697 11.1142 7.77639C11.1489 7.84582 11.167 7.92238 11.167 8C11.167 8.07762 11.1489 8.15418 11.1142 8.22361C11.0795 8.29303 11.0291 8.35343 10.967 8.4L6.3 11.9C6.22572 11.9557 6.13738 11.9896 6.0449 11.998C5.95242 12.0063 5.85945 11.9887 5.77639 11.9472C5.69334 11.9057 5.62349 11.8419 5.57467 11.7629C5.52586 11.6839 5.5 11.5929 5.5 11.5V4.5Z\" fill=\"currentColor\"></path></svg>"},"chevron-up-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M12 10L8.70711 6.70711C8.31658 6.31658 7.68342 6.31658 7.29289 6.70711L4 10\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12 10L8.70711 6.70711C8.31658 6.31658 7.68342 6.31658 7.29289 6.70711L4 10\" stroke=\"currentColor\"></path></svg>"},"close-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.5 2.5L13.5 13.5\" stroke=\"currentColor\"></path><path d=\"M13.5 2.5L2.5 13.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.5 2.5L13.5 13.5\" stroke=\"currentColor\"></path><path d=\"M13.5 2.5L2.5 13.5\" stroke=\"currentColor\"></path></svg>"},"close-fill":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3.5 3.5L12.5 12.5\" stroke=\"currentColor\"></path><path d=\"M12.5 3.5L3.5 12.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3.5 3.5L12.5 12.5\" stroke=\"currentColor\"></path><path d=\"M12.5 3.5L3.5 12.5\" stroke=\"currentColor\"></path></svg>"},"close-circle-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path fill-rule=\"evenodd\" clip-rule=\"evenodd\" d=\"M15 8A7 7 0 1 1 1 8A7 7 0 1 1 15 8ZM6.409 10.652L5.348 9.591L6.939 8L5.348 6.409L6.409 5.348L8 6.939L9.591 5.348L10.652 6.409L9.061 8L10.652 9.591L9.591 10.652L8 9.061Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path fill-rule=\"evenodd\" clip-rule=\"evenodd\" d=\"M15 8A7 7 0 1 1 1 8A7 7 0 1 1 15 8ZM6.409 10.652L5.348 9.591L6.939 8L5.348 6.409L6.409 5.348L8 6.939L9.591 5.348L10.652 6.409L9.061 8L10.652 9.591L9.591 10.652L8 9.061Z\" fill=\"currentColor\"></path></svg>"},"copy-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><rect x=\"1.52075\" y=\"4.07373\" width=\"10.3932\" height=\"10.3932\" rx=\"2\" stroke=\"currentColor\"></rect><path d=\"M11.9792 1.53296C13.36 1.53296 14.4792 2.65225 14.4792 4.03296V9.42847C14.4792 10.3756 13.9521 11.1987 13.1755 11.6228V10.3298C13.3652 10.0787 13.4792 9.7674 13.4792 9.42847V4.03296C13.4792 3.20453 12.8077 2.53296 11.9792 2.53296H6.58374C6.27966 2.53301 5.99684 2.6235 5.7605 2.77905H4.42358C4.85652 2.03463 5.66056 1.53304 6.58374 1.53296H11.9792Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><rect x=\"1.52075\" y=\"4.07373\" width=\"10.3932\" height=\"10.3932\" rx=\"2\" stroke=\"currentColor\"></rect><path d=\"M11.9792 1.53296C13.36 1.53296 14.4792 2.65225 14.4792 4.03296V9.42847C14.4792 10.3756 13.9521 11.1987 13.1755 11.6228V10.3298C13.3652 10.0787 13.4792 9.7674 13.4792 9.42847V4.03296C13.4792 3.20453 12.8077 2.53296 11.9792 2.53296H6.58374C6.27966 2.53301 5.99684 2.6235 5.7605 2.77905H4.42358C4.85652 2.03463 5.66056 1.53304 6.58374 1.53296H11.9792Z\" fill=\"currentColor\"></path></svg>"},"refresh-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M14.5001 8C14.5 9.28552 14.1188 10.5422 13.4045 11.611C12.6903 12.6799 11.6752 13.5129 10.4875 14.0049C9.29982 14.4968 7.99295 14.6255 6.73212 14.3747C5.4713 14.124 4.31314 13.505 3.4041 12.596C2.49514 11.687 1.87614 10.5288 1.62537 9.26798C1.37459 8.00716 1.50331 6.70028 1.99525 5.51261C2.48719 4.32494 3.32025 3.30981 4.3891 2.59557C5.45795 1.88134 6.71458 1.50008 8.0001 1.5C9.9001 1.5 11.7001 2.3 13.0001 3.6L14.5001 5.1\" stroke=\"currentColor\"></path><path d=\"M14.4999 1.5V5.1H10.8999\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M14.5001 8C14.5 9.28552 14.1188 10.5422 13.4045 11.611C12.6903 12.6799 11.6752 13.5129 10.4875 14.0049C9.29982 14.4968 7.99295 14.6255 6.73212 14.3747C5.4713 14.124 4.31314 13.505 3.4041 12.596C2.49514 11.687 1.87614 10.5288 1.62537 9.26798C1.37459 8.00716 1.50331 6.70028 1.99525 5.51261C2.48719 4.32494 3.32025 3.30981 4.3891 2.59557C5.45795 1.88134 6.71458 1.50008 8.0001 1.5C9.9001 1.5 11.7001 2.3 13.0001 3.6L14.5001 5.1\" stroke=\"currentColor\"></path><path d=\"M14.4999 1.5V5.1H10.8999\" stroke=\"currentColor\"></path></svg>"},"like-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M13.537 8.12098L12.3983 12.8455C12.1818 13.7438 11.378 14.3769 10.454 14.3769L9.35595 14.3769H7.43799H5.16577C3.50892 14.3769 2.16577 13.0337 2.16577 11.3769V7.88668C2.16577 7.33439 2.61349 6.88668 3.16577 6.88668H4.02665C5.84943 6.88668 7.38083 3.28711 7.67689 2.54578C7.71259 2.45639 7.73501 2.36373 7.77922 2.27824C7.86506 2.11221 8.08228 1.87578 8.59039 2.07775C10.3291 2.76886 9.23144 6.04071 8.96955 6.75058C8.94502 6.81707 8.99495 6.88668 9.06581 6.88668H12.5648C13.2119 6.88668 13.6886 7.49192 13.537 8.12098Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M13.537 8.12098L12.3983 12.8455C12.1818 13.7438 11.378 14.3769 10.454 14.3769L9.35595 14.3769H7.43799H5.16577C3.50892 14.3769 2.16577 13.0337 2.16577 11.3769V7.88668C2.16577 7.33439 2.61349 6.88668 3.16577 6.88668H4.02665C5.84943 6.88668 7.38083 3.28711 7.67689 2.54578C7.71259 2.45639 7.73501 2.36373 7.77922 2.27824C7.86506 2.11221 8.08228 1.87578 8.59039 2.07775C10.3291 2.76886 9.23144 6.04071 8.96955 6.75058C8.94502 6.81707 8.99495 6.88668 9.06581 6.88668H12.5648C13.2119 6.88668 13.6886 7.49192 13.537 8.12098Z\" stroke=\"currentColor\"></path></svg>"},"like-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M13.537 8.12098L12.3983 12.8455C12.1818 13.7438 11.378 14.3769 10.454 14.3769L9.35595 14.3769H7.43799H5.16577C3.50892 14.3769 2.16577 13.0337 2.16577 11.3769V7.88668C2.16577 7.33439 2.61349 6.88668 3.16577 6.88668H4.02665C5.84943 6.88668 7.38083 3.28711 7.67689 2.54578C7.71259 2.45639 7.73501 2.36373 7.77922 2.27824C7.86506 2.11221 8.08228 1.87578 8.59039 2.07775C10.3291 2.76886 9.23144 6.04071 8.96955 6.75058C8.94502 6.81707 8.99495 6.88668 9.06581 6.88668H12.5648C13.2119 6.88668 13.6886 7.49192 13.537 8.12098Z\" fill=\"currentColor\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M13.537 8.12098L12.3983 12.8455C12.1818 13.7438 11.378 14.3769 10.454 14.3769L9.35595 14.3769H7.43799H5.16577C3.50892 14.3769 2.16577 13.0337 2.16577 11.3769V7.88668C2.16577 7.33439 2.61349 6.88668 3.16577 6.88668H4.02665C5.84943 6.88668 7.38083 3.28711 7.67689 2.54578C7.71259 2.45639 7.73501 2.36373 7.77922 2.27824C7.86506 2.11221 8.08228 1.87578 8.59039 2.07775C10.3291 2.76886 9.23144 6.04071 8.96955 6.75058C8.94502 6.81707 8.99495 6.88668 9.06581 6.88668H12.5648C13.2119 6.88668 13.6886 7.49192 13.537 8.12098Z\" fill=\"currentColor\" stroke=\"currentColor\"></path></svg>"},"dislike-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.46302 8.06749L3.60171 3.34299C3.81822 2.44467 4.62196 1.81162 5.546 1.8116L6.64406 1.81158L8.56202 1.81158L10.8342 1.81158C12.4911 1.81158 13.8342 3.15473 13.8342 4.81158L13.8342 8.3018C13.8342 8.85408 13.3865 9.3018 12.8342 9.3018L11.9734 9.3018C10.1506 9.3018 8.61918 12.9014 8.32311 13.6427C8.28741 13.7321 8.26499 13.8247 8.22078 13.9102C8.13494 14.0763 7.91772 14.3127 7.40961 14.1107C5.67089 13.4196 6.76856 10.1478 7.03045 9.43789C7.05498 9.37141 7.00505 9.3018 6.93419 9.3018L3.43519 9.3018C2.78811 9.3018 2.31141 8.69656 2.46302 8.06749Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.46302 8.06749L3.60171 3.34299C3.81822 2.44467 4.62196 1.81162 5.546 1.8116L6.64406 1.81158L8.56202 1.81158L10.8342 1.81158C12.4911 1.81158 13.8342 3.15473 13.8342 4.81158L13.8342 8.3018C13.8342 8.85408 13.3865 9.3018 12.8342 9.3018L11.9734 9.3018C10.1506 9.3018 8.61918 12.9014 8.32311 13.6427C8.28741 13.7321 8.26499 13.8247 8.22078 13.9102C8.13494 14.0763 7.91772 14.3127 7.40961 14.1107C5.67089 13.4196 6.76856 10.1478 7.03045 9.43789C7.05498 9.37141 7.00505 9.3018 6.93419 9.3018L3.43519 9.3018C2.78811 9.3018 2.31141 8.69656 2.46302 8.06749Z\" stroke=\"currentColor\"></path></svg>"},"dislike-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.46302 8.06749L3.60171 3.34299C3.81822 2.44467 4.62196 1.81162 5.546 1.8116L6.64406 1.81158L8.56202 1.81158L10.8342 1.81158C12.4911 1.81158 13.8342 3.15473 13.8342 4.81158L13.8342 8.3018C13.8342 8.85408 13.3865 9.3018 12.8342 9.3018L11.9734 9.3018C10.1506 9.3018 8.61918 12.9014 8.32311 13.6427C8.28741 13.7321 8.26499 13.8247 8.22078 13.9102C8.13494 14.0763 7.91772 14.3127 7.40961 14.1107C5.67089 13.4196 6.76856 10.1478 7.03045 9.43789C7.05498 9.37141 7.00505 9.3018 6.93419 9.3018L3.43519 9.3018C2.78811 9.3018 2.31141 8.69656 2.46302 8.06749Z\" fill=\"currentColor\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.46302 8.06749L3.60171 3.34299C3.81822 2.44467 4.62196 1.81162 5.546 1.8116L6.64406 1.81158L8.56202 1.81158L10.8342 1.81158C12.4911 1.81158 13.8342 3.15473 13.8342 4.81158L13.8342 8.3018C13.8342 8.85408 13.3865 9.3018 12.8342 9.3018L11.9734 9.3018C10.1506 9.3018 8.61918 12.9014 8.32311 13.6427C8.28741 13.7321 8.26499 13.8247 8.22078 13.9102C8.13494 14.0763 7.91772 14.3127 7.40961 14.1107C5.67089 13.4196 6.76856 10.1478 7.03045 9.43789C7.05498 9.37141 7.00505 9.3018 6.93419 9.3018L3.43519 9.3018C2.78811 9.3018 2.31141 8.69656 2.46302 8.06749Z\" fill=\"currentColor\" stroke=\"currentColor\"></path></svg>"},"share-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M14.1256 7.58723C14.3483 7.81942 14.3482 8.18589 14.1254 8.41799L8.6646 14.1077C8.53985 14.2377 8.32031 14.1494 8.32031 13.9692L8.32035 10.2039C8.32035 10.1943 8.31534 10.1864 8.30592 10.1849C8.08306 10.148 5.30067 9.7729 1.50993 13.2904C1.49711 13.3023 1.47561 13.2943 1.4757 13.2768C1.49273 9.87168 3.42001 5.07166 8.29999 5.05835C8.31103 5.05832 8.32035 5.04937 8.32035 5.03832L8.32031 2.03109C8.32031 1.85088 8.53993 1.76259 8.66466 1.89266L14.1256 7.58723Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M14.1256 7.58723C14.3483 7.81942 14.3482 8.18589 14.1254 8.41799L8.6646 14.1077C8.53985 14.2377 8.32031 14.1494 8.32031 13.9692L8.32035 10.2039C8.32035 10.1943 8.31534 10.1864 8.30592 10.1849C8.08306 10.148 5.30067 9.7729 1.50993 13.2904C1.49711 13.3023 1.47561 13.2943 1.4757 13.2768C1.49273 9.87168 3.42001 5.07166 8.29999 5.05835C8.31103 5.05832 8.32035 5.04937 8.32035 5.03832L8.32031 2.03109C8.32031 1.85088 8.53993 1.76259 8.66466 1.89266L14.1256 7.58723Z\" stroke=\"currentColor\"></path></svg>"},"deliver-doc":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.15479 4.91687H9.84543\" stroke=\"currentColor\"></path><path d=\"M11.8798 9.55347V2.71525C11.8798 2.37416 11.564 2.09766 11.1744 2.09766H4.82577C4.43618 2.09766 4.12036 2.37416 4.12036 2.71525V9.55347\" stroke=\"currentColor\"></path><path d=\"M2.28735 13.8022V8.84792C2.28735 8.77514 2.36262 8.72673 2.42884 8.75693L13.2936 13.7112C13.3914 13.7558 13.3596 13.9022 13.2521 13.9022H2.38735C2.33213 13.9022 2.28735 13.8575 2.28735 13.8022Z\" stroke=\"currentColor\"></path><path d=\"M7.46929 10.979L13.5783 8.7416C13.6435 8.7177 13.7126 8.76601 13.7126 8.83551L13.7125 13.8022C13.7125 13.8574 13.6678 13.9022 13.6125 13.9022H7.99999\" stroke=\"currentColor\"></path><path d=\"M6.15479 7.2395H9.05644\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.15479 4.91687H9.84543\" stroke=\"currentColor\"></path><path d=\"M11.8798 9.55347V2.71525C11.8798 2.37416 11.564 2.09766 11.1744 2.09766H4.82577C4.43618 2.09766 4.12036 2.37416 4.12036 2.71525V9.55347\" stroke=\"currentColor\"></path><path d=\"M2.28735 13.8022V8.84792C2.28735 8.77514 2.36262 8.72673 2.42884 8.75693L13.2936 13.7112C13.3914 13.7558 13.3596 13.9022 13.2521 13.9022H2.38735C2.33213 13.9022 2.28735 13.8575 2.28735 13.8022Z\" stroke=\"currentColor\"></path><path d=\"M7.46929 10.979L13.5783 8.7416C13.6435 8.7177 13.7126 8.76601 13.7126 8.83551L13.7125 13.8022C13.7125 13.8574 13.6678 13.9022 13.6125 13.9022H7.99999\" stroke=\"currentColor\"></path><path d=\"M6.15479 7.2395H9.05644\" stroke=\"currentColor\"></path></svg>"},"edit-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8.85596 2.69971H4.19971C3.37141 2.69971 2.69992 3.37146 2.69971 4.19971V11.8003C2.69992 12.6285 3.37141 13.3003 4.19971 13.3003H11.8003C12.6283 13.2999 13.3001 12.6283 13.3003 11.8003V7.89893H14.3003V11.8003C14.3001 13.1806 13.1806 14.2999 11.8003 14.3003H4.19971C2.81913 14.3003 1.69992 13.1808 1.69971 11.8003V4.19971C1.69992 2.81918 2.81913 1.69971 4.19971 1.69971H8.85596V2.69971Z\" fill=\"currentColor\"></path><path d=\"M7.7849 8.23878L13.888 2.13574\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8.85596 2.69971H4.19971C3.37141 2.69971 2.69992 3.37146 2.69971 4.19971V11.8003C2.69992 12.6285 3.37141 13.3003 4.19971 13.3003H11.8003C12.6283 13.2999 13.3001 12.6283 13.3003 11.8003V7.89893H14.3003V11.8003C14.3001 13.1806 13.1806 14.2999 11.8003 14.3003H4.19971C2.81913 14.3003 1.69992 13.1808 1.69971 11.8003V4.19971C1.69992 2.81918 2.81913 1.69971 4.19971 1.69971H8.85596V2.69971Z\" fill=\"currentColor\"></path><path d=\"M7.7849 8.23878L13.888 2.13574\" stroke=\"currentColor\"></path></svg>"},"think-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M10.2854 5.71481C12.9673 8.39663 14.1182 11.5938 12.8562 12.8559C11.5942 14.1179 8.39706 12.9669 5.71518 10.2851C3.03333 7.60323 1.88236 4.40608 3.14441 3.14403C4.40644 1.882 7.6036 3.03297 10.2854 5.71481Z\" stroke=\"currentColor\"></path><path d=\"M10.2854 10.2851C7.6036 12.9669 4.40644 14.1179 3.14441 12.8559C1.88236 11.5938 3.03333 8.39663 5.71518 5.71481C8.39706 3.03297 11.5942 1.882 12.8562 3.14403C14.1182 4.40608 12.9673 7.60323 10.2854 10.2851Z\" stroke=\"currentColor\"></path><path d=\"M8.86291 8.0002C8.86291 8.47549 8.47762 8.86087 8.00224 8.86087C7.52694 8.86087 7.1416 8.47549 7.1416 8.0002C7.1416 7.52485 7.52694 7.13953 8.00224 7.13953C8.47762 7.13953 8.86291 7.52485 8.86291 8.0002Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M10.2854 5.71481C12.9673 8.39663 14.1182 11.5938 12.8562 12.8559C11.5942 14.1179 8.39706 12.9669 5.71518 10.2851C3.03333 7.60323 1.88236 4.40608 3.14441 3.14403C4.40644 1.882 7.6036 3.03297 10.2854 5.71481Z\" stroke=\"currentColor\"></path><path d=\"M10.2854 10.2851C7.6036 12.9669 4.40644 14.1179 3.14441 12.8559C1.88236 11.5938 3.03333 8.39663 5.71518 5.71481C8.39706 3.03297 11.5942 1.882 12.8562 3.14403C14.1182 4.40608 12.9673 7.60323 10.2854 10.2851Z\" stroke=\"currentColor\"></path><path d=\"M8.86291 8.0002C8.86291 8.47549 8.47762 8.86087 8.00224 8.86087C7.52694 8.86087 7.1416 8.47549 7.1416 8.0002C7.1416 7.52485 7.52694 7.13953 8.00224 7.13953C8.47762 7.13953 8.86291 7.52485 8.86291 8.0002Z\" fill=\"currentColor\"></path></svg>"},"agent-preset-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.51867 12.3282C7.29816 12.6011 8.16475 12.6514 9.02269 12.4216C9.57879 12.2726 10.0784 12.0185 10.5087 11.6888C10.7819 12.0555 11.1606 12.3304 11.5913 12.4805C10.9688 13.029 10.2149 13.4478 9.35911 13.6771C8.13946 14.0038 6.90632 13.8971 5.82126 13.4533C6.15821 13.1562 6.4021 12.7652 6.51867 12.3282ZM9.17629 2.89409C11.1101 3.34433 12.739 4.81872 13.2889 6.87043C13.4219 7.3665 13.4811 7.8649 13.4774 8.35466C13.0924 8.13213 12.6422 8.01837 12.1741 8.05276L12.1711 8.05257C12.1539 7.77199 12.109 7.48889 12.0334 7.20684C11.6363 5.72533 10.5048 4.6372 9.13549 4.22844C9.25559 3.87667 9.29214 3.49087 9.22309 3.09892C9.2108 3.02922 9.19451 2.96108 9.17629 2.89409ZM4.7311 3.89107L4.78302 4.11879C4.87648 4.4488 5.04146 4.74263 5.25579 4.98896C3.98078 6.01355 3.35848 7.72904 3.8089 9.41059C3.81828 9.44559 3.82866 9.48025 3.83885 9.51479C3.38217 9.61268 2.98548 9.84137 2.68107 10.1556C2.63414 10.022 2.5897 9.88632 2.55244 9.74726C1.93301 7.43489 2.86717 5.07173 4.71504 3.76697L4.7311 3.89107Z\" fill=\"currentColor\"></path><path d=\"M7.99136 5.28105C8.87501 5.28105 9.59136 4.56471 9.59136 3.68105C9.59136 2.7974 8.87501 2.08105 7.99136 2.08105C7.1077 2.08105 6.39136 2.7974 6.39136 3.68105C6.39136 4.56471 7.1077 5.28105 7.99136 5.28105Z\" stroke=\"currentColor\"></path><path d=\"M3.94009 12.9417C4.82374 12.9417 5.54009 12.2254 5.54009 11.3417C5.54009 10.458 4.82374 9.7417 3.94009 9.7417C3.05643 9.7417 2.34009 10.458 2.34009 11.3417C2.34009 12.2254 3.05643 12.9417 3.94009 12.9417Z\" stroke=\"currentColor\"></path><path d=\"M12.0851 12.9417C12.9688 12.9417 13.6851 12.2254 13.6851 11.3417C13.6851 10.458 12.9688 9.7417 12.0851 9.7417C11.2015 9.7417 10.4851 10.458 10.4851 11.3417C10.4851 12.2254 11.2015 12.9417 12.0851 12.9417Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.51867 12.3282C7.29816 12.6011 8.16475 12.6514 9.02269 12.4216C9.57879 12.2726 10.0784 12.0185 10.5087 11.6888C10.7819 12.0555 11.1606 12.3304 11.5913 12.4805C10.9688 13.029 10.2149 13.4478 9.35911 13.6771C8.13946 14.0038 6.90632 13.8971 5.82126 13.4533C6.15821 13.1562 6.4021 12.7652 6.51867 12.3282ZM9.17629 2.89409C11.1101 3.34433 12.739 4.81872 13.2889 6.87043C13.4219 7.3665 13.4811 7.8649 13.4774 8.35466C13.0924 8.13213 12.6422 8.01837 12.1741 8.05276L12.1711 8.05257C12.1539 7.77199 12.109 7.48889 12.0334 7.20684C11.6363 5.72533 10.5048 4.6372 9.13549 4.22844C9.25559 3.87667 9.29214 3.49087 9.22309 3.09892C9.2108 3.02922 9.19451 2.96108 9.17629 2.89409ZM4.7311 3.89107L4.78302 4.11879C4.87648 4.4488 5.04146 4.74263 5.25579 4.98896C3.98078 6.01355 3.35848 7.72904 3.8089 9.41059C3.81828 9.44559 3.82866 9.48025 3.83885 9.51479C3.38217 9.61268 2.98548 9.84137 2.68107 10.1556C2.63414 10.022 2.5897 9.88632 2.55244 9.74726C1.93301 7.43489 2.86717 5.07173 4.71504 3.76697L4.7311 3.89107Z\" fill=\"currentColor\"></path><path d=\"M7.99136 5.28105C8.87501 5.28105 9.59136 4.56471 9.59136 3.68105C9.59136 2.7974 8.87501 2.08105 7.99136 2.08105C7.1077 2.08105 6.39136 2.7974 6.39136 3.68105C6.39136 4.56471 7.1077 5.28105 7.99136 5.28105Z\" stroke=\"currentColor\"></path><path d=\"M3.94009 12.9417C4.82374 12.9417 5.54009 12.2254 5.54009 11.3417C5.54009 10.458 4.82374 9.7417 3.94009 9.7417C3.05643 9.7417 2.34009 10.458 2.34009 11.3417C2.34009 12.2254 3.05643 12.9417 3.94009 12.9417Z\" stroke=\"currentColor\"></path><path d=\"M12.0851 12.9417C12.9688 12.9417 13.6851 12.2254 13.6851 11.3417C13.6851 10.458 12.9688 9.7417 12.0851 9.7417C11.2015 9.7417 10.4851 10.458 10.4851 11.3417C10.4851 12.2254 11.2015 12.9417 12.0851 12.9417Z\" stroke=\"currentColor\"></path></svg>"},"browse-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.9375 5.90295H11.0625\" stroke=\"currentColor\"></path><path d=\"M4.9375 9.02991H8.27841\" stroke=\"currentColor\"></path><path d=\"M12.5 1.32617C13.3039 1.32617 14 1.95171 14 2.77637V13.2246C13.9996 14.0489 13.3036 14.6738 12.5 14.6738H3.5C2.69637 14.6738 2.00042 14.0489 2 13.2246V2.77637C2 1.95171 2.69613 1.32617 3.5 1.32617H12.5ZM3.5 2.32617C3.1993 2.32617 3 2.55186 3 2.77637V13.2246C3.00044 13.4489 3.19963 13.6738 3.5 13.6738H12.5C12.8004 13.6738 12.9996 13.4489 13 13.2246V2.77637C13 2.55186 12.8007 2.32617 12.5 2.32617H3.5Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.9375 5.90295H11.0625\" stroke=\"currentColor\"></path><path d=\"M4.9375 9.02991H8.27841\" stroke=\"currentColor\"></path><path d=\"M12.5 1.32617C13.3039 1.32617 14 1.95171 14 2.77637V13.2246C13.9996 14.0489 13.3036 14.6738 12.5 14.6738H3.5C2.69637 14.6738 2.00042 14.0489 2 13.2246V2.77637C2 1.95171 2.69613 1.32617 3.5 1.32617H12.5ZM3.5 2.32617C3.1993 2.32617 3 2.55186 3 2.77637V13.2246C3.00044 13.4489 3.19963 13.6738 3.5 13.6738H12.5C12.8004 13.6738 12.9996 13.4489 13 13.2246V2.77637C13 2.55186 12.8007 2.32617 12.5 2.32617H3.5Z\" fill=\"currentColor\"></path></svg>"},"context-injection-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M5 2.5H3.5C3.23478 2.5 2.98043 2.60536 2.79289 2.79289C2.60536 2.98043 2.5 3.23478 2.5 3.5V13.5C2.5 13.7652 2.60536 14.0196 2.79289 14.2071C2.98043 14.3946 3.23478 14.5 3.5 14.5H12.5C12.7652 14.5 13.0196 14.3946 13.2071 14.2071C13.3946 14.0196 13.5 13.7652 13.5 13.5V3.5C13.5 3.23478 13.3946 2.98043 13.2071 2.79289C13.0196 2.60536 12.7652 2.5 12.5 2.5H11\" stroke=\"currentColor\"></path><path d=\"M8 0.5V7.5\" stroke=\"currentColor\"></path><path d=\"M5.5 5L8 7.5L10.5 5\" stroke=\"currentColor\"></path><path d=\"M5.5 11H10.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M5 2.5H3.5C3.23478 2.5 2.98043 2.60536 2.79289 2.79289C2.60536 2.98043 2.5 3.23478 2.5 3.5V13.5C2.5 13.7652 2.60536 14.0196 2.79289 14.2071C2.98043 14.3946 3.23478 14.5 3.5 14.5H12.5C12.7652 14.5 13.0196 14.3946 13.2071 14.2071C13.3946 14.0196 13.5 13.7652 13.5 13.5V3.5C13.5 3.23478 13.3946 2.98043 13.2071 2.79289C13.0196 2.60536 12.7652 2.5 12.5 2.5H11\" stroke=\"currentColor\"></path><path d=\"M8 0.5V7.5\" stroke=\"currentColor\"></path><path d=\"M5.5 5L8 7.5L10.5 5\" stroke=\"currentColor\"></path><path d=\"M5.5 11H10.5\" stroke=\"currentColor\"></path></svg>"},"link-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.59961 9.40051C6.82779 9.6334 7.10015 9.81842 7.40074 9.94472C7.70132 10.071 8.02409 10.1361 8.35013 10.1361C8.67618 10.1361 8.99894 10.071 9.29953 9.94472C9.60011 9.81842 9.87247 9.6334 10.1007 9.40051L12.9015 6.59967C13.3658 6.13541 13.6266 5.50572 13.6266 4.84915C13.6266 4.19258 13.3658 3.56289 12.9015 3.09863C12.4372 2.63436 11.8075 2.37354 11.151 2.37354C10.4944 2.37354 9.86472 2.63436 9.40045 3.09863L9.05034 3.44873\" stroke=\"currentColor\"></path><path d=\"M9.40051 6.59959C9.17233 6.3667 8.89997 6.18169 8.59939 6.05538C8.2988 5.92907 7.97603 5.86401 7.64999 5.86401C7.32395 5.86401 7.00118 5.92907 6.70059 6.05538C6.40001 6.18169 6.12765 6.3667 5.89946 6.59959L3.09863 9.40043C2.63436 9.8647 2.37354 10.4944 2.37354 11.151C2.37354 11.8075 2.63436 12.4372 3.09863 12.9015C3.56289 13.3657 4.19258 13.6266 4.84915 13.6266C5.50572 13.6266 6.13541 13.3657 6.59967 12.9015L6.94978 12.5514\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.59961 9.40051C6.82779 9.6334 7.10015 9.81842 7.40074 9.94472C7.70132 10.071 8.02409 10.1361 8.35013 10.1361C8.67618 10.1361 8.99894 10.071 9.29953 9.94472C9.60011 9.81842 9.87247 9.6334 10.1007 9.40051L12.9015 6.59967C13.3658 6.13541 13.6266 5.50572 13.6266 4.84915C13.6266 4.19258 13.3658 3.56289 12.9015 3.09863C12.4372 2.63436 11.8075 2.37354 11.151 2.37354C10.4944 2.37354 9.86472 2.63436 9.40045 3.09863L9.05034 3.44873\" stroke=\"currentColor\"></path><path d=\"M9.40051 6.59959C9.17233 6.3667 8.89997 6.18169 8.59939 6.05538C8.2988 5.92907 7.97603 5.86401 7.64999 5.86401C7.32395 5.86401 7.00118 5.92907 6.70059 6.05538C6.40001 6.18169 6.12765 6.3667 5.89946 6.59959L3.09863 9.40043C2.63436 9.8647 2.37354 10.4944 2.37354 11.151C2.37354 11.8075 2.63436 12.4372 3.09863 12.9015C3.56289 13.3657 4.19258 13.6266 4.84915 13.6266C5.50572 13.6266 6.13541 13.3657 6.59967 12.9015L6.94978 12.5514\" stroke=\"currentColor\"></path></svg>"},"right-up-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M11.7256 2.77441C12.5538 2.77469 13.2256 3.44616 13.2256 4.27441V10.1416H12.2256V4.27441C12.2256 3.99844 12.0015 3.77469 11.7256 3.77441H5.7207V2.77441H11.7256Z\" fill=\"currentColor\"></path><path d=\"M2.77441 13.2255L12.3756 3.62427\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M11.7256 2.77441C12.5538 2.77469 13.2256 3.44616 13.2256 4.27441V10.1416H12.2256V4.27441C12.2256 3.99844 12.0015 3.77469 11.7256 3.77441H5.7207V2.77441H11.7256Z\" fill=\"currentColor\"></path><path d=\"M2.77441 13.2255L12.3756 3.62427\" stroke=\"currentColor\"></path></svg>"},"enhance-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M1.98486 2.95374H14.0151\" stroke=\"currentColor\"></path><path d=\"M1.98486 6.31787H14.0151\" stroke=\"currentColor\"></path><path d=\"M1.98486 9.68213H14.0151\" stroke=\"currentColor\"></path><path d=\"M1.98486 13.0463H8.4627\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M1.98486 2.95374H14.0151\" stroke=\"currentColor\"></path><path d=\"M1.98486 6.31787H14.0151\" stroke=\"currentColor\"></path><path d=\"M1.98486 9.68213H14.0151\" stroke=\"currentColor\"></path><path d=\"M1.98486 13.0463H8.4627\" stroke=\"currentColor\"></path></svg>"},"trash-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M1.28149 3.88831H14.7187\" stroke=\"currentColor\"></path><path d=\"M5.41602 3.88833V2.47962C5.41602 2.29282 5.52492 2.11366 5.71876 1.98157C5.9126 1.84948 6.17551 1.77527 6.44964 1.77527H9.55053C9.82466 1.77527 10.0876 1.84948 10.2814 1.98157C10.4753 2.11366 10.5842 2.29282 10.5842 2.47962V3.88833\" stroke=\"currentColor\"></path><path d=\"M2.57349 3.88831L3.19366 13.2943C3.21937 13.5502 3.33952 13.7872 3.53065 13.9593C3.72178 14.1313 3.97016 14.2259 4.22729 14.2246H11.7728C12.0299 14.2259 12.2783 14.1313 12.4694 13.9593C12.6605 13.7872 12.7807 13.5502 12.8064 13.2943L13.4266 3.88831\" stroke=\"currentColor\"></path><path d=\"M6.44946 6.98926V11.1238\" stroke=\"currentColor\"></path><path d=\"M9.55054 6.98926V11.1238\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M1.28149 3.88831H14.7187\" stroke=\"currentColor\"></path><path d=\"M5.41602 3.88833V2.47962C5.41602 2.29282 5.52492 2.11366 5.71876 1.98157C5.9126 1.84948 6.17551 1.77527 6.44964 1.77527H9.55053C9.82466 1.77527 10.0876 1.84948 10.2814 1.98157C10.4753 2.11366 10.5842 2.29282 10.5842 2.47962V3.88833\" stroke=\"currentColor\"></path><path d=\"M2.57349 3.88831L3.19366 13.2943C3.21937 13.5502 3.33952 13.7872 3.53065 13.9593C3.72178 14.1313 3.97016 14.2259 4.22729 14.2246H11.7728C12.0299 14.2259 12.2783 14.1313 12.4694 13.9593C12.6605 13.7872 12.7807 13.5502 12.8064 13.2943L13.4266 3.88831\" stroke=\"currentColor\"></path><path d=\"M6.44946 6.98926V11.1238\" stroke=\"currentColor\"></path><path d=\"M9.55054 6.98926V11.1238\" stroke=\"currentColor\"></path></svg>"},"warning-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M8 4.29199V9.79199\" stroke=\"currentColor\"></path><path d=\"M8 10.708V11.708\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M8 4.29199V9.79199\" stroke=\"currentColor\"></path><path d=\"M8 10.708V11.708\" stroke=\"currentColor\"></path></svg>"},"check-circle-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 36 36\" fill=\"none\" stroke-width=\"1\"><path d=\"M28.1936 14.6936L19.8066 23.0806C19.2373 23.65 18.7159 24.1742 18.24 24.5571C17.7389 24.9602 17.1365 25.3359 16.3657 25.458C15.9581 25.5225 15.5428 25.5225 15.1353 25.458C14.3645 25.3359 13.7621 24.9602 13.261 24.5571C12.7851 24.1742 12.2637 23.65 11.6943 23.0806L7.80737 19.1936L10.1936 16.8074L14.0806 20.6943C14.7033 21.317 15.0763 21.6873 15.377 21.9292C15.6523 22.1507 15.7109 22.1325 15.6626 22.1248C15.7208 22.1339 15.7802 22.1339 15.8384 22.1248C15.7901 22.1325 15.8486 22.1507 16.124 21.9292C16.4247 21.6873 16.7977 21.317 17.4204 20.6943L25.8074 12.3074L28.1936 14.6936Z\" fill=\"currentColor\"></path><path d=\"M32.8496 18.0005C32.8496 9.79906 26.2019 3.15137 18.0005 3.15137C9.79906 3.15137 3.15137 9.79906 3.15137 18.0005C3.15137 26.2019 9.79906 32.8496 18.0005 32.8496C26.2019 32.8496 32.8496 26.2019 32.8496 18.0005ZM35.7764 18.0005C35.7764 27.8173 27.8173 35.7764 18.0005 35.7764C8.18363 35.7764 0.224609 27.8173 0.224609 18.0005C0.224609 8.18363 8.18363 0.224609 18.0005 0.224609C27.8173 0.224609 35.7764 8.18363 35.7764 18.0005Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 36 36\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M28.1936 14.6936L19.8066 23.0806C19.2373 23.65 18.7159 24.1742 18.24 24.5571C17.7389 24.9602 17.1365 25.3359 16.3657 25.458C15.9581 25.5225 15.5428 25.5225 15.1353 25.458C14.3645 25.3359 13.7621 24.9602 13.261 24.5571C12.7851 24.1742 12.2637 23.65 11.6943 23.0806L7.80737 19.1936L10.1936 16.8074L14.0806 20.6943C14.7033 21.317 15.0763 21.6873 15.377 21.9292C15.6523 22.1507 15.7109 22.1325 15.6626 22.1248C15.7208 22.1339 15.7802 22.1339 15.8384 22.1248C15.7901 22.1325 15.8486 22.1507 16.124 21.9292C16.4247 21.6873 16.7977 21.317 17.4204 20.6943L25.8074 12.3074L28.1936 14.6936Z\" fill=\"currentColor\"></path><path d=\"M32.8496 18.0005C32.8496 9.79906 26.2019 3.15137 18.0005 3.15137C9.79906 3.15137 3.15137 9.79906 3.15137 18.0005C3.15137 26.2019 9.79906 32.8496 18.0005 32.8496C26.2019 32.8496 32.8496 26.2019 32.8496 18.0005ZM35.7764 18.0005C35.7764 27.8173 27.8173 35.7764 18.0005 35.7764C8.18363 35.7764 0.224609 27.8173 0.224609 18.0005C0.224609 8.18363 8.18363 0.224609 18.0005 0.224609C27.8173 0.224609 35.7764 8.18363 35.7764 18.0005Z\" fill=\"currentColor\"></path></svg>"},"warning-triangle-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1\"><path d=\"M6.87 2.6a1.33 1.33 0 0 1 2.26 0l5.34 9.33A1.33 1.33 0 0 1 13.33 14H2.67a1.33 1.33 0 0 1-1.14-2.07Z\" stroke=\"currentColor\"></path><path d=\"M8 6v3m0 2.33h.01\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.3\"><path d=\"M6.87 2.6a1.33 1.33 0 0 1 2.26 0l5.34 9.33A1.33 1.33 0 0 1 13.33 14H2.67a1.33 1.33 0 0 1-1.14-2.07Z\" stroke=\"currentColor\"></path><path d=\"M8 6v3m0 2.33h.01\" stroke=\"currentColor\"></path></svg>"},"user-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 8.25C9.51878 8.25 10.75 7.01878 10.75 5.5C10.75 3.98122 9.51878 2.75 8 2.75C6.48122 2.75 5.25 3.98122 5.25 5.5C5.25 7.01878 6.48122 8.25 8 8.25Z\" stroke=\"currentColor\"></path><path d=\"M2.5 14.5C2.5 11.5 5.25 10.25 8 10.25C10.75 10.25 13.5 11.5 13.5 14.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 8.25C9.51878 8.25 10.75 7.01878 10.75 5.5C10.75 3.98122 9.51878 2.75 8 2.75C6.48122 2.75 5.25 3.98122 5.25 5.5C5.25 7.01878 6.48122 8.25 8 8.25Z\" stroke=\"currentColor\"></path><path d=\"M2.5 14.5C2.5 11.5 5.25 10.25 8 10.25C10.75 10.25 13.5 11.5 13.5 14.5\" stroke=\"currentColor\"></path></svg>"},"users-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.18 8.2275C7.5620898 8.2275 8.6825 7.1070898 8.6825 5.725C8.6825 4.3429102 7.5620898 3.2225 6.18 3.2225C4.7979102 3.2225 3.6775 4.3429102 3.6775 5.725C3.6775 7.1070898 4.7979102 8.2275 6.18 8.2275Z\" stroke=\"currentColor\"></path><path d=\"M1.63 13.915C1.63 11.185 3.905 10.0475 6.18 10.0475C8.455 10.0475 10.73 11.185 10.73 13.915\" stroke=\"currentColor\"></path><path d=\"M10.275 3.359C11.3215 3.7685 12.0495 4.6785 12.0495 5.725C12.0495 6.7715 11.3215 7.6815 10.275 8.091\" stroke=\"currentColor\"></path><path d=\"M12.004 10.366C13.369 11.003 14.37 12.186 14.37 13.915\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.18 8.2275C7.5620898 8.2275 8.6825 7.1070898 8.6825 5.725C8.6825 4.3429102 7.5620898 3.2225 6.18 3.2225C4.7979102 3.2225 3.6775 4.3429102 3.6775 5.725C3.6775 7.1070898 4.7979102 8.2275 6.18 8.2275Z\" stroke=\"currentColor\"></path><path d=\"M1.63 13.915C1.63 11.185 3.905 10.0475 6.18 10.0475C8.455 10.0475 10.73 11.185 10.73 13.915\" stroke=\"currentColor\"></path><path d=\"M10.275 3.359C11.3215 3.7685 12.0495 4.6785 12.0495 5.725C12.0495 6.7715 11.3215 7.6815 10.275 8.091\" stroke=\"currentColor\"></path><path d=\"M12.004 10.366C13.369 11.003 14.37 12.186 14.37 13.915\" stroke=\"currentColor\"></path></svg>"},"paper-plane-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.74024 9.11029L1.82882 7.79865C1.75022 7.76323 1.75026 7.65161 1.82889 7.61626L12.9665 2.60943C13.0354 2.57846 13.1125 2.63213 13.1073 2.70749L12.3914 13.1388C12.3864 13.2117 12.3073 13.2548 12.2433 13.2194L6.12677 9.83657\" stroke=\"currentColor\"></path><path d=\"M8.44336 11.0825L6.2832 13.2843C6.22048 13.3482 6.11182 13.3038 6.11182 13.2143V9.86772C6.11182 9.84165 6.122 9.8166 6.1402 9.79793L12.972 2.78748\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.74024 9.11029L1.82882 7.79865C1.75022 7.76323 1.75026 7.65161 1.82889 7.61626L12.9665 2.60943C13.0354 2.57846 13.1125 2.63213 13.1073 2.70749L12.3914 13.1388C12.3864 13.2117 12.3073 13.2548 12.2433 13.2194L6.12677 9.83657\" stroke=\"currentColor\"></path><path d=\"M8.44336 11.0825L6.2832 13.2843C6.22048 13.3482 6.11182 13.3038 6.11182 13.2143V9.86772C6.11182 9.84165 6.122 9.8166 6.1402 9.79793L12.972 2.78748\" stroke=\"currentColor\"></path></svg>"},"stop-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M12.5 2.5H3.5C2.94772 2.5 2.5 2.94772 2.5 3.5V12.5C2.5 13.0523 2.94772 13.5 3.5 13.5H12.5C13.0523 13.5 13.5 13.0523 13.5 12.5V3.5C13.5 2.94772 13.0523 2.5 12.5 2.5Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12.5 2.5H3.5C2.94772 2.5 2.5 2.94772 2.5 3.5V12.5C2.5 13.0523 2.94772 13.5 3.5 13.5H12.5C13.0523 13.5 13.5 13.0523 13.5 12.5V3.5C13.5 2.94772 13.0523 2.5 12.5 2.5Z\" fill=\"currentColor\"></path></svg>"},"paperclip-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M12.75 4.5V9.5C12.75 10.7598 12.2496 11.968 11.3588 12.8588C10.468 13.7496 9.25978 14.25 8 14.25C6.74022 14.25 5.53204 13.7496 4.64124 12.8588C3.75045 11.968 3.25 10.7598 3.25 9.5V5C3.25 4.13805 3.59241 3.3114 4.2019 2.7019C4.8114 2.09241 5.63805 1.75 6.5 1.75C7.36195 1.75 8.1886 2.09241 8.7981 2.7019C9.40759 3.3114 9.75 4.13805 9.75 5V9.5C9.75 9.96413 9.56563 10.4092 9.23744 10.7374C8.90925 11.0656 8.46413 11.25 8 11.25C7.53587 11.25 7.09075 11.0656 6.76256 10.7374C6.43437 10.4092 6.25 9.96413 6.25 9.5V5.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12.75 4.5V9.5C12.75 10.7598 12.2496 11.968 11.3588 12.8588C10.468 13.7496 9.25978 14.25 8 14.25C6.74022 14.25 5.53204 13.7496 4.64124 12.8588C3.75045 11.968 3.25 10.7598 3.25 9.5V5C3.25 4.13805 3.59241 3.3114 4.2019 2.7019C4.8114 2.09241 5.63805 1.75 6.5 1.75C7.36195 1.75 8.1886 2.09241 8.7981 2.7019C9.40759 3.3114 9.75 4.13805 9.75 5V9.5C9.75 9.96413 9.56563 10.4092 9.23744 10.7374C8.90925 11.0656 8.46413 11.25 8 11.25C7.53587 11.25 7.09075 11.0656 6.76256 10.7374C6.43437 10.4092 6.25 9.96413 6.25 9.5V5.5\" stroke=\"currentColor\"></path></svg>"},"loading-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M12.596 12.596C11.687 13.5049 10.5288 14.1239 9.26798 14.3747C8.00716 14.6255 6.70028 14.4968 5.51261 14.0048C4.32494 13.5129 3.30981 12.6798 2.59557 11.611C1.88134 10.5421 1.50008 9.2855 1.5 7.99998C1.50008 6.71446 1.88134 5.45783 2.59557 4.38898C3.30981 3.32013 4.32494 2.48707 5.51261 1.99513C6.70028 1.50319 8.00716 1.37447 9.26798 1.62524C10.5288 1.87602 11.687 2.49502 12.596 3.40398\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12.596 12.596C11.687 13.5049 10.5288 14.1239 9.26798 14.3747C8.00716 14.6255 6.70028 14.4968 5.51261 14.0048C4.32494 13.5129 3.30981 12.6798 2.59557 11.611C1.88134 10.5421 1.50008 9.2855 1.5 7.99998C1.50008 6.71446 1.88134 5.45783 2.59557 4.38898C3.30981 3.32013 4.32494 2.48707 5.51261 1.99513C6.70028 1.50319 8.00716 1.37447 9.26798 1.62524C10.5288 1.87602 11.687 2.49502 12.596 3.40398\" stroke=\"currentColor\"></path></svg>"},"download-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 1.95317V10.0469\" stroke=\"currentColor\"></path><path d=\"M4.25 6.29688L8 10.0469L11.75 6.29688\" stroke=\"currentColor\"></path><path d=\"M1.5 10.0469V13.158C1.5 13.3937 1.60536 13.6198 1.79289 13.7865C1.98043 13.9532 2.23478 14.0469 2.5 14.0469H13.5C13.7652 14.0469 14.0196 13.9532 14.2071 13.7865C14.3946 13.6198 14.5 13.3937 14.5 13.158V10.0469\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 1.95317V10.0469\" stroke=\"currentColor\"></path><path d=\"M4.25 6.29688L8 10.0469L11.75 6.29688\" stroke=\"currentColor\"></path><path d=\"M1.5 10.0469V13.158C1.5 13.3937 1.60536 13.6198 1.79289 13.7865C1.98043 13.9532 2.23478 14.0469 2.5 14.0469H13.5C13.7652 14.0469 14.0196 13.9532 14.2071 13.7865C14.3946 13.6198 14.5 13.3937 14.5 13.158V10.0469\" stroke=\"currentColor\"></path></svg>"},"upload-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 10.0469V1.95317\" stroke=\"currentColor\"></path><path d=\"M4.25 5.70317L8 1.95317L11.75 5.70317\" stroke=\"currentColor\"></path><path d=\"M1.5 10.0469V13.158C1.5 13.3937 1.60536 13.6198 1.79289 13.7865C1.98043 13.9532 2.23478 14.0469 2.5 14.0469H13.5C13.7652 14.0469 14.0196 13.9532 14.2071 13.7865C14.3946 13.6198 14.5 13.3937 14.5 13.158V10.0469\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 10.0469V1.95317\" stroke=\"currentColor\"></path><path d=\"M4.25 5.70317L8 1.95317L11.75 5.70317\" stroke=\"currentColor\"></path><path d=\"M1.5 10.0469V13.158C1.5 13.3937 1.60536 13.6198 1.79289 13.7865C1.98043 13.9532 2.23478 14.0469 2.5 14.0469H13.5C13.7652 14.0469 14.0196 13.9532 14.2071 13.7865C14.3946 13.6198 14.5 13.3937 14.5 13.158V10.0469\" stroke=\"currentColor\"></path></svg>"},"play-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M10.3329 7.91346C10.3996 7.95195 10.3996 8.04818 10.3329 8.08667L6.78304 10.1362C6.71638 10.1747 6.63304 10.1266 6.63304 10.0496L6.63304 5.95055C6.63304 5.87357 6.71638 5.82546 6.78304 5.86395L10.3329 7.91346Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M10.3329 7.91346C10.3996 7.95195 10.3996 8.04818 10.3329 8.08667L6.78304 10.1362C6.71638 10.1747 6.63304 10.1266 6.63304 10.0496L6.63304 5.95055C6.63304 5.87357 6.71638 5.82546 6.78304 5.86395L10.3329 7.91346Z\" stroke=\"currentColor\"></path></svg>"},"pause-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M6.5 5V11\" stroke=\"currentColor\"></path><path d=\"M9.5 5V11\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M6.5 5V11\" stroke=\"currentColor\"></path><path d=\"M9.5 5V11\" stroke=\"currentColor\"></path></svg>"},"fullscreen-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.33154 9.40576V13.1685C2.3318 13.4444 2.55556 13.6685 2.83154 13.6685H6.49463V14.6685H2.83154C2.00328 14.6685 1.3318 13.9967 1.33154 13.1685V9.40576H2.33154ZM13.1685 1.33154C13.9964 1.33199 14.6683 2.00352 14.6685 2.83154V6.40576H13.6685V2.83154C13.6683 2.5558 13.4441 2.33199 13.1685 2.33154H9.49463V1.33154H13.1685Z\" fill=\"currentColor\"></path><path d=\"M9.4292 6.57077L13.914 2.08594\" stroke=\"currentColor\"></path><path d=\"M6.57077 9.4292L2.08594 13.914\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.33154 9.40576V13.1685C2.3318 13.4444 2.55556 13.6685 2.83154 13.6685H6.49463V14.6685H2.83154C2.00328 14.6685 1.3318 13.9967 1.33154 13.1685V9.40576H2.33154ZM13.1685 1.33154C13.9964 1.33199 14.6683 2.00352 14.6685 2.83154V6.40576H13.6685V2.83154C13.6683 2.5558 13.4441 2.33199 13.1685 2.33154H9.49463V1.33154H13.1685Z\" fill=\"currentColor\"></path><path d=\"M9.4292 6.57077L13.914 2.08594\" stroke=\"currentColor\"></path><path d=\"M6.57077 9.4292L2.08594 13.914\" stroke=\"currentColor\"></path></svg>"},"code-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.27612 1.5L4.52612 14.5\" stroke=\"currentColor\"></path><path d=\"M11.4739 1.5L9.72388 14.5\" stroke=\"currentColor\"></path><path d=\"M2.39868 5.5H14.0681\" stroke=\"currentColor\"></path><path d=\"M1.93188 10.5H13.6013\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.27612 1.5L4.52612 14.5\" stroke=\"currentColor\"></path><path d=\"M11.4739 1.5L9.72388 14.5\" stroke=\"currentColor\"></path><path d=\"M2.39868 5.5H14.0681\" stroke=\"currentColor\"></path><path d=\"M1.93188 10.5H13.6013\" stroke=\"currentColor\"></path></svg>"},"cordis-plugin-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3.16143 6.59068L1.75205 8.00006L3.10619 9.35419L2.39908 10.0613L0.832948 8.49517C0.559581 8.2218 0.559582 7.77831 0.832948 7.50494L2.45432 5.88357L3.16143 6.59068ZM8.49511 15.1671C8.22176 15.4405 7.77826 15.4404 7.50489 15.1671L5.93461 13.5968L6.64172 12.8897L8 14.248L9.40938 12.8386L10.1165 13.5457L8.49511 15.1671ZM15.1671 7.50494C15.4403 7.7782 15.4401 8.22179 15.1671 8.49517L13.652 10.0102L12.9449 9.30309L14.248 8.00006L12.8897 6.64178L13.5968 5.93467L15.1671 7.50494ZM9.35414 3.10624L8 1.7521L6.69696 3.05514L5.98986 2.34803L7.50489 0.833003C7.77828 0.559981 8.22186 0.559752 8.49511 0.833003L10.0612 2.39913L9.35414 3.10624Z\" fill=\"currentColor\"></path><circle cx=\"8\" cy=\"8\" r=\"1.76221\" stroke=\"currentColor\"></circle></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3.16143 6.59068L1.75205 8.00006L3.10619 9.35419L2.39908 10.0613L0.832948 8.49517C0.559581 8.2218 0.559582 7.77831 0.832948 7.50494L2.45432 5.88357L3.16143 6.59068ZM8.49511 15.1671C8.22176 15.4405 7.77826 15.4404 7.50489 15.1671L5.93461 13.5968L6.64172 12.8897L8 14.248L9.40938 12.8386L10.1165 13.5457L8.49511 15.1671ZM15.1671 7.50494C15.4403 7.7782 15.4401 8.22179 15.1671 8.49517L13.652 10.0102L12.9449 9.30309L14.248 8.00006L12.8897 6.64178L13.5968 5.93467L15.1671 7.50494ZM9.35414 3.10624L8 1.7521L6.69696 3.05514L5.98986 2.34803L7.50489 0.833003C7.77828 0.559981 8.22186 0.559752 8.49511 0.833003L10.0612 2.39913L9.35414 3.10624Z\" fill=\"currentColor\"></path><circle cx=\"8\" cy=\"8\" r=\"1.76221\" stroke=\"currentColor\"></circle></svg>"},"api-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3 4L7 8L3 12\" stroke=\"currentColor\"></path><path d=\"M9 12H13\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3 4L7 8L3 12\" stroke=\"currentColor\"></path><path d=\"M9 12H13\" stroke=\"currentColor\"></path></svg>"},"personalization-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3.25 7.16357C3.20417 7.32247 3.17778 7.48993 3.17773 7.66357C3.17773 7.83698 3.20336 8.00486 3.24902 8.16357H1.85742V7.16357H3.25ZM14.1426 8.16357H6.71484C6.76052 8.00485 6.78613 7.837 6.78613 7.66357C6.78609 7.48991 6.75971 7.32249 6.71387 7.16357H14.1426V8.16357Z\" fill=\"currentColor\"></path><path d=\"M9.1377 11.9092C9.08596 12.0666 9.05668 12.2344 9.05664 12.4092C9.05664 12.5838 9.08606 12.7518 9.1377 12.9092H1.85742V11.9092H9.1377ZM14.1426 12.9092H12.1816C12.2332 12.7519 12.2617 12.5838 12.2617 12.4092C12.2617 12.2345 12.2333 12.0666 12.1816 11.9092H14.1426V12.9092Z\" fill=\"currentColor\"></path><path d=\"M9.1123 3.09106C9.06138 3.24865 9.03324 3.41653 9.0332 3.59106C9.0332 3.76549 9.06148 3.93355 9.1123 4.09106H1.85742V3.09106H9.1123ZM14.1426 4.09106H12.207C12.2578 3.93358 12.2861 3.76545 12.2861 3.59106C12.2861 3.41657 12.2579 3.24862 12.207 3.09106H14.1426V4.09106Z\" fill=\"currentColor\"></path><circle cx=\"4.97065\" cy=\"7.66401\" r=\"1.35151\" stroke=\"currentColor\"></circle><circle cx=\"10.6596\" cy=\"12.4091\" r=\"1.35151\" stroke=\"currentColor\"></circle><circle cx=\"10.6596\" cy=\"3.59101\" r=\"1.35151\" stroke=\"currentColor\"></circle></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3.25 7.16357C3.20417 7.32247 3.17778 7.48993 3.17773 7.66357C3.17773 7.83698 3.20336 8.00486 3.24902 8.16357H1.85742V7.16357H3.25ZM14.1426 8.16357H6.71484C6.76052 8.00485 6.78613 7.837 6.78613 7.66357C6.78609 7.48991 6.75971 7.32249 6.71387 7.16357H14.1426V8.16357Z\" fill=\"currentColor\"></path><path d=\"M9.1377 11.9092C9.08596 12.0666 9.05668 12.2344 9.05664 12.4092C9.05664 12.5838 9.08606 12.7518 9.1377 12.9092H1.85742V11.9092H9.1377ZM14.1426 12.9092H12.1816C12.2332 12.7519 12.2617 12.5838 12.2617 12.4092C12.2617 12.2345 12.2333 12.0666 12.1816 11.9092H14.1426V12.9092Z\" fill=\"currentColor\"></path><path d=\"M9.1123 3.09106C9.06138 3.24865 9.03324 3.41653 9.0332 3.59106C9.0332 3.76549 9.06148 3.93355 9.1123 4.09106H1.85742V3.09106H9.1123ZM14.1426 4.09106H12.207C12.2578 3.93358 12.2861 3.76545 12.2861 3.59106C12.2861 3.41657 12.2579 3.24862 12.207 3.09106H14.1426V4.09106Z\" fill=\"currentColor\"></path><circle cx=\"4.97065\" cy=\"7.66401\" r=\"1.35151\" stroke=\"currentColor\"></circle><circle cx=\"10.6596\" cy=\"12.4091\" r=\"1.35151\" stroke=\"currentColor\"></circle><circle cx=\"10.6596\" cy=\"3.59101\" r=\"1.35151\" stroke=\"currentColor\"></circle></svg>"},"project-add-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M5.54492 2.06738C5.91034 2.06754 6.26318 2.20149 6.53711 2.44336L7.94043 3.68164V4.7998C7.71462 4.74105 7.50367 4.63139 7.32617 4.47461L5.87598 3.19238C5.78477 3.11185 5.66658 3.06754 5.54492 3.06738H2.94922C2.67322 3.06738 2.44946 3.29145 2.44922 3.56738V12.4326C2.44927 12.7087 2.67311 12.9326 2.94922 12.9326H12.9326C13.2086 12.9325 13.4326 12.7086 13.4326 12.4326V8.53613H14.4326V12.4326C14.4326 13.2609 13.7609 13.9325 12.9326 13.9326H2.94922C2.12083 13.9326 1.44927 13.261 1.44922 12.4326V3.56738C1.44946 2.73916 2.12094 2.06738 2.94922 2.06738H5.54492Z\" fill=\"currentColor\"></path><path d=\"M9.75977 4.50208H14.5509\" stroke=\"currentColor\"></path><path d=\"M12.1492 6.89758L12.1492 2.10642\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M5.54492 2.06738C5.91034 2.06754 6.26318 2.20149 6.53711 2.44336L7.94043 3.68164V4.7998C7.71462 4.74105 7.50367 4.63139 7.32617 4.47461L5.87598 3.19238C5.78477 3.11185 5.66658 3.06754 5.54492 3.06738H2.94922C2.67322 3.06738 2.44946 3.29145 2.44922 3.56738V12.4326C2.44927 12.7087 2.67311 12.9326 2.94922 12.9326H12.9326C13.2086 12.9325 13.4326 12.7086 13.4326 12.4326V8.53613H14.4326V12.4326C14.4326 13.2609 13.7609 13.9325 12.9326 13.9326H2.94922C2.12083 13.9326 1.44927 13.261 1.44922 12.4326V3.56738C1.44946 2.73916 2.12094 2.06738 2.94922 2.06738H5.54492Z\" fill=\"currentColor\"></path><path d=\"M9.75977 4.50208H14.5509\" stroke=\"currentColor\"></path><path d=\"M12.1492 6.89758L12.1492 2.10642\" stroke=\"currentColor\"></path></svg>"},"folder-open-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M12.3994 13.5986H2.04956C1.49728 13.5986 1.04956 13.1509 1.04956 12.5986V3.40137C1.04956 2.84908 1.49728 2.40137 2.04956 2.40137H4.76632C5.01016 2.40137 5.24561 2.49046 5.42836 2.6519L6.94088 3.98799C7.12364 4.14943 7.35908 4.23852 7.60293 4.23852H12.3994C12.9517 4.23852 13.3994 4.68624 13.3994 5.23852V7.16991\" stroke=\"currentColor\"></path><path d=\"M2.55911 7.93683C2.67584 7.49906 3.07229 7.19446 3.52536 7.19446H13.6491C14.3061 7.19446 14.7846 7.81725 14.6153 8.45209L13.4411 12.856C13.3244 13.2938 12.9279 13.5984 12.4748 13.5984H2.35113C1.69411 13.5984 1.21562 12.9756 1.38489 12.3407L2.55911 7.93683Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12.3994 13.5986H2.04956C1.49728 13.5986 1.04956 13.1509 1.04956 12.5986V3.40137C1.04956 2.84908 1.49728 2.40137 2.04956 2.40137H4.76632C5.01016 2.40137 5.24561 2.49046 5.42836 2.6519L6.94088 3.98799C7.12364 4.14943 7.35908 4.23852 7.60293 4.23852H12.3994C12.9517 4.23852 13.3994 4.68624 13.3994 5.23852V7.16991\" stroke=\"currentColor\"></path><path d=\"M2.55911 7.93683C2.67584 7.49906 3.07229 7.19446 3.52536 7.19446H13.6491C14.3061 7.19446 14.7846 7.81725 14.6153 8.45209L13.4411 12.856C13.3244 13.2938 12.9279 13.5984 12.4748 13.5984H2.35113C1.69411 13.5984 1.21562 12.9756 1.38489 12.3407L2.55911 7.93683Z\" stroke=\"currentColor\"></path></svg>"},"folder-open":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.55912 7.93683C2.67584 7.49906 3.0723 7.19446 3.52536 7.19446H13.6491C14.3061 7.19446 14.7846 7.81725 14.6153 8.45209L13.4411 12.856C13.3244 13.2938 12.9279 13.5984 12.4748 13.5984H2.35113C1.69411 13.5984 1.21562 12.9756 1.38489 12.3407L2.55912 7.93683Z\" fill=\"currentColor\" opacity=\"0.16\"></path><path d=\"M13.6491 6.69446C14.6346 6.69453 15.3522 7.62895 15.0983 8.58118L13.9245 12.9845C13.7494 13.6412 13.1539 14.0988 12.4743 14.0988H2.35126C1.36574 14.0988 0.648153 13.1643 0.902044 12.212L2.07587 7.80774C2.25102 7.15128 2.84567 6.69455 3.52509 6.69446H13.6491ZM3.52509 7.69446C3.29865 7.69455 3.10004 7.84674 3.04169 8.06555L1.86786 12.4698C1.78345 12.7872 2.02285 13.0988 2.35126 13.0988H12.4743C12.7007 13.0988 12.8992 12.9463 12.9577 12.7277L14.1325 8.32336C14.2171 8.00598 13.9776 7.69453 13.6491 7.69446H3.52509Z\" fill=\"currentColor\"></path><path d=\"M4.7666 1.90137C5.13227 1.90144 5.48571 2.03525 5.75977 2.27734L7.27246 3.61328C7.36379 3.69382 7.48174 3.73828 7.60352 3.73828H12.3994C13.2276 3.73841 13.8993 4.41005 13.8994 5.23828V6.7168C13.8183 6.70327 13.735 6.69436 13.6494 6.69434H12.8994V5.23828C12.8993 4.96233 12.6754 4.73841 12.3994 4.73828H7.60352C7.23781 4.73828 6.88446 4.60438 6.61035 4.3623L5.09766 3.02637C5.00636 2.94576 4.88838 2.90144 4.7666 2.90137H2.0498C1.77366 2.90137 1.5498 3.12523 1.5498 3.40137V9.78223L0.902344 12.2119C0.648452 13.1642 1.36604 14.0986 2.35156 14.0986H2.0498C1.2214 14.0986 0.549838 13.427 0.549805 12.5986V3.40137C0.549805 2.57294 1.22138 1.90137 2.0498 1.90137H4.7666Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.55912 7.93683C2.67584 7.49906 3.0723 7.19446 3.52536 7.19446H13.6491C14.3061 7.19446 14.7846 7.81725 14.6153 8.45209L13.4411 12.856C13.3244 13.2938 12.9279 13.5984 12.4748 13.5984H2.35113C1.69411 13.5984 1.21562 12.9756 1.38489 12.3407L2.55912 7.93683Z\" fill=\"currentColor\" opacity=\"0.16\"></path><path d=\"M13.6491 6.69446C14.6346 6.69453 15.3522 7.62895 15.0983 8.58118L13.9245 12.9845C13.7494 13.6412 13.1539 14.0988 12.4743 14.0988H2.35126C1.36574 14.0988 0.648153 13.1643 0.902044 12.212L2.07587 7.80774C2.25102 7.15128 2.84567 6.69455 3.52509 6.69446H13.6491ZM3.52509 7.69446C3.29865 7.69455 3.10004 7.84674 3.04169 8.06555L1.86786 12.4698C1.78345 12.7872 2.02285 13.0988 2.35126 13.0988H12.4743C12.7007 13.0988 12.8992 12.9463 12.9577 12.7277L14.1325 8.32336C14.2171 8.00598 13.9776 7.69453 13.6491 7.69446H3.52509Z\" fill=\"currentColor\"></path><path d=\"M4.7666 1.90137C5.13227 1.90144 5.48571 2.03525 5.75977 2.27734L7.27246 3.61328C7.36379 3.69382 7.48174 3.73828 7.60352 3.73828H12.3994C13.2276 3.73841 13.8993 4.41005 13.8994 5.23828V6.7168C13.8183 6.70327 13.735 6.69436 13.6494 6.69434H12.8994V5.23828C12.8993 4.96233 12.6754 4.73841 12.3994 4.73828H7.60352C7.23781 4.73828 6.88446 4.60438 6.61035 4.3623L5.09766 3.02637C5.00636 2.94576 4.88838 2.90144 4.7666 2.90137H2.0498C1.77366 2.90137 1.5498 3.12523 1.5498 3.40137V9.78223L0.902344 12.2119C0.648452 13.1642 1.36604 14.0986 2.35156 14.0986H2.0498C1.2214 14.0986 0.549838 13.427 0.549805 12.5986V3.40137C0.549805 2.57294 1.22138 1.90137 2.0498 1.90137H4.7666Z\" fill=\"currentColor\"></path></svg>"},"folder-close":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M1.50439 3.11059C1.50439 2.55831 1.95211 2.1106 2.50439 2.1106H5.43389C5.67773 2.1106 5.91318 2.19969 6.09593 2.36113L7.71649 3.79265C7.89924 3.95409 8.1347 4.04319 8.3785 4.04319H13.4958C14.0481 4.04319 14.4958 4.4909 14.4958 5.04319V12.8894C14.4958 13.4417 14.0481 13.8894 13.4958 13.8894H2.50439C1.95211 13.8894 1.50439 13.4417 1.50439 12.8894V4.04319V3.11059Z\" stroke=\"currentColor\"></path><path d=\"M3.63501 7.66614H12.3647\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M1.50439 3.11059C1.50439 2.55831 1.95211 2.1106 2.50439 2.1106H5.43389C5.67773 2.1106 5.91318 2.19969 6.09593 2.36113L7.71649 3.79265C7.89924 3.95409 8.1347 4.04319 8.3785 4.04319H13.4958C14.0481 4.04319 14.4958 4.4909 14.4958 5.04319V12.8894C14.4958 13.4417 14.0481 13.8894 13.4958 13.8894H2.50439C1.95211 13.8894 1.50439 13.4417 1.50439 12.8894V4.04319V3.11059Z\" stroke=\"currentColor\"></path><path d=\"M3.63501 7.66614H12.3647\" stroke=\"currentColor\"></path></svg>"},"tree-corner":{"size":10,"aspect":[8,10],"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 9 11\" fill=\"none\" stroke-width=\"1\"><path d=\"M0.5 0V7C0.5 7.79565 0.81607 8.55871 1.37868 9.12132C1.94129 9.68393 2.70435 10 3.5 10H8.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 9 11\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M0.5 0V7C0.5 7.79565 0.81607 8.55871 1.37868 9.12132C1.94129 9.68393 2.70435 10 3.5 10H8.5\" stroke=\"currentColor\"></path></svg>"},"light-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8.00007 11.8117C10.1052 11.8117 11.8117 10.1052 11.8117 8.00007C11.8117 5.89499 10.1052 4.18848 8.00007 4.18848C5.89499 4.18848 4.18848 5.89499 4.18848 8.00007C4.18848 10.1052 5.89499 11.8117 8.00007 11.8117Z\" stroke=\"currentColor\"></path><path d=\"M13.3899 8H15.1499\" stroke=\"currentColor\"></path><path d=\"M11.8115 11.8115L13.0556 13.0556\" stroke=\"currentColor\"></path><path d=\"M8 13.3901V15.1501\" stroke=\"currentColor\"></path><path d=\"M4.18868 11.8115L2.94458 13.0556\" stroke=\"currentColor\"></path><path d=\"M2.6101 8H0.850098\" stroke=\"currentColor\"></path><path d=\"M4.18868 4.18856L2.94458 2.94446\" stroke=\"currentColor\"></path><path d=\"M8 2.6101V0.850098\" stroke=\"currentColor\"></path><path d=\"M11.8115 4.18856L13.0556 2.94446\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8.00007 11.8117C10.1052 11.8117 11.8117 10.1052 11.8117 8.00007C11.8117 5.89499 10.1052 4.18848 8.00007 4.18848C5.89499 4.18848 4.18848 5.89499 4.18848 8.00007C4.18848 10.1052 5.89499 11.8117 8.00007 11.8117Z\" stroke=\"currentColor\"></path><path d=\"M13.3899 8H15.1499\" stroke=\"currentColor\"></path><path d=\"M11.8115 11.8115L13.0556 13.0556\" stroke=\"currentColor\"></path><path d=\"M8 13.3901V15.1501\" stroke=\"currentColor\"></path><path d=\"M4.18868 11.8115L2.94458 13.0556\" stroke=\"currentColor\"></path><path d=\"M2.6101 8H0.850098\" stroke=\"currentColor\"></path><path d=\"M4.18868 4.18856L2.94458 2.94446\" stroke=\"currentColor\"></path><path d=\"M8 2.6101V0.850098\" stroke=\"currentColor\"></path><path d=\"M11.8115 4.18856L13.0556 2.94446\" stroke=\"currentColor\"></path></svg>"},"dark-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M14.1127 8.70663C14.2576 8.60602 14.4627 8.71355 14.4386 8.88834C14.2901 9.96567 13.8731 10.9912 13.2229 11.8692C12.479 12.8735 11.4613 13.6421 10.2917 14.0829C9.1222 14.5236 7.85038 14.6179 6.62865 14.3543C5.40692 14.0907 4.28709 13.4805 3.40332 12.5967C2.51955 11.7129 1.90931 10.5931 1.64572 9.37135C1.38212 8.14962 1.47635 6.87779 1.91711 5.70825C2.35787 4.5387 3.12647 3.52103 4.13083 2.77714C5.00878 2.12689 6.03433 1.70994 7.11166 1.5614C7.28645 1.5373 7.39397 1.74238 7.29337 1.88734C6.68703 2.76099 6.37885 3.81241 6.42313 4.88345C6.47392 6.11194 6.98471 7.27645 7.85413 8.14587C8.72355 9.01529 9.88805 9.52608 11.1166 9.57687C12.1876 9.62114 13.239 9.31296 14.1127 8.70663Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M14.1127 8.70663C14.2576 8.60602 14.4627 8.71355 14.4386 8.88834C14.2901 9.96567 13.8731 10.9912 13.2229 11.8692C12.479 12.8735 11.4613 13.6421 10.2917 14.0829C9.1222 14.5236 7.85038 14.6179 6.62865 14.3543C5.40692 14.0907 4.28709 13.4805 3.40332 12.5967C2.51955 11.7129 1.90931 10.5931 1.64572 9.37135C1.38212 8.14962 1.47635 6.87779 1.91711 5.70825C2.35787 4.5387 3.12647 3.52103 4.13083 2.77714C5.00878 2.12689 6.03433 1.70994 7.11166 1.5614C7.28645 1.5373 7.39397 1.74238 7.29337 1.88734C6.68703 2.76099 6.37885 3.81241 6.42313 4.88345C6.47392 6.11194 6.98471 7.27645 7.85413 8.14587C8.72355 9.01529 9.88805 9.52608 11.1166 9.57687C12.1876 9.62114 13.239 9.31296 14.1127 8.70663Z\" stroke=\"currentColor\"></path></svg>"},"followsystem-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M13.5 2.5H2.5C1.94772 2.5 1.5 2.94772 1.5 3.5V11.5C1.5 12.0523 1.94772 12.5 2.5 12.5H13.5C14.0523 12.5 14.5 12.0523 14.5 11.5V3.5C14.5 2.94772 14.0523 2.5 13.5 2.5Z\" stroke=\"currentColor\"></path><path d=\"M5 14.5H11\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M13.5 2.5H2.5C1.94772 2.5 1.5 2.94772 1.5 3.5V11.5C1.5 12.0523 1.94772 12.5 2.5 12.5H13.5C14.0523 12.5 14.5 12.0523 14.5 11.5V3.5C14.5 2.94772 14.0523 2.5 13.5 2.5Z\" stroke=\"currentColor\"></path><path d=\"M5 14.5H11\" stroke=\"currentColor\"></path></svg>"},"data-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M7.8667 0.349609C8.96906 0.349634 10.0601 0.481272 11.0317 0.735352C11.9973 0.987845 12.8453 1.362 13.4644 1.84766C14.0744 2.32629 14.507 2.95539 14.5161 3.69336H14.5171V8.53516C14.0843 8.32076 13.6108 8.17679 13.1108 8.11816C13.1831 7.96848 13.2162 7.82856 13.2163 7.70312V5.76758C12.6269 6.16618 11.8739 6.47995 11.0317 6.7002C10.0602 6.95423 8.96896 7.08494 7.8667 7.08496C6.76461 7.08493 5.67411 6.95415 4.70264 6.7002C3.85994 6.48006 3.10694 6.1662 2.51709 5.76758V7.70312L2.521 7.78418C2.56374 8.19554 2.93361 8.74414 3.91357 9.23145C4.9281 9.73585 6.35004 10.0371 7.8667 10.0371C8.26373 10.0371 8.6543 10.0141 9.03271 9.97461C8.75596 10.3799 8.54664 10.8349 8.42041 11.3232C8.23666 11.3313 8.0518 11.3369 7.8667 11.3369C6.20108 11.3369 4.57025 11.01 3.33447 10.3955C3.04163 10.2499 2.76658 10.0836 2.51709 9.90039V11.6738C2.51728 12.1379 2.88589 12.7556 3.92236 13.292C4.93457 13.8157 6.35342 14.1289 7.8667 14.1289C8.12318 14.1289 8.37694 14.1161 8.62646 14.0986C8.82021 14.5535 9.08999 14.9682 9.41943 15.3271C8.91285 15.3934 8.39149 15.4287 7.8667 15.4287C6.19761 15.4287 4.56379 15.0869 3.32568 14.4463C2.11244 13.8185 1.21649 12.8562 1.21631 11.6738V3.76367C1.21595 3.74853 1.21438 3.733 1.21436 3.71777C1.21436 2.96917 1.65103 2.33053 2.26807 1.84668C2.88747 1.36112 3.73675 0.987685 4.70264 0.735352C5.67413 0.481376 6.76457 0.349636 7.8667 0.349609ZM7.8667 1.65039C6.86269 1.65042 5.88326 1.77028 5.03076 1.99316C4.17183 2.2176 3.50421 2.52956 3.06982 2.87012C2.65043 3.19909 2.52622 3.48898 2.51709 3.69336V3.74414C2.52719 3.94845 2.65185 4.23772 3.06982 4.56543C3.50425 4.90601 4.17172 5.21795 5.03076 5.44238C5.88326 5.66527 6.8627 5.78513 7.8667 5.78516C8.8707 5.78513 9.85015 5.66525 10.7026 5.44238C11.5611 5.21787 12.2286 4.9049 12.6626 4.56445C13.0982 4.22252 13.2163 3.9231 13.2163 3.71777L13.2104 3.63574C13.1818 3.43623 13.044 3.16941 12.6626 2.87012C12.2286 2.52957 11.5614 2.21773 10.7026 1.99316C9.85009 1.77025 8.8708 1.65041 7.8667 1.65039Z\" fill=\"currentColor\"></path><path d=\"M12.8936 10.0361L13.2061 10.5566C13.2296 10.5959 13.2651 10.6562 13.3027 10.707C13.3469 10.7666 13.4148 10.8431 13.5195 10.9023C13.6244 10.9617 13.725 10.9801 13.7988 10.9873C13.8619 10.9934 13.9318 10.9932 13.9775 10.9932H14.6162L14.8896 11.4502L14.5947 11.9443C14.5698 11.9859 14.5312 12.0483 14.5029 12.1084C14.4781 12.1611 14.4514 12.2312 14.4395 12.3164L14.4326 12.4072L14.4395 12.4971C14.4514 12.5825 14.4781 12.6532 14.5029 12.7061C14.5312 12.7661 14.5689 12.8287 14.5938 12.8701L14.8896 13.3633L14.6162 13.8213H13.9775C13.9318 13.8213 13.8619 13.821 13.7988 13.8271C13.7433 13.8326 13.6728 13.8442 13.5967 13.875L13.5195 13.9121C13.4148 13.9714 13.3469 14.0478 13.3027 14.1074C13.265 14.1583 13.2296 14.2186 13.2061 14.2578L12.8936 14.7783H12.3115L11.999 14.2578C11.9755 14.2186 11.9401 14.1583 11.9023 14.1074C11.8693 14.0628 11.823 14.0083 11.7578 13.959L11.6855 13.9121L11.6074 13.875C11.5316 13.8445 11.4615 13.8325 11.4062 13.8271C11.3432 13.821 11.2733 13.8213 11.2275 13.8213H10.5889L10.3135 13.3633L10.6104 12.8701C10.6352 12.8287 10.6739 12.7661 10.7021 12.7061C10.7352 12.6357 10.7724 12.534 10.7725 12.4072C10.7724 12.2804 10.7352 12.1788 10.7021 12.1084C10.6739 12.0483 10.6353 11.9859 10.6104 11.9443L10.3135 11.4502L10.5889 10.9932H11.2275C11.2733 10.9932 11.3432 10.9934 11.4062 10.9873C11.4801 10.9801 11.5808 10.9616 11.6855 10.9023C11.7903 10.843 11.8582 10.7666 11.9023 10.707C11.94 10.6562 11.9755 10.5959 11.999 10.5566L12.3115 10.0361H12.8936Z\" stroke=\"currentColor\" stroke-miterlimit=\"10\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M7.8667 0.349609C8.96906 0.349634 10.0601 0.481272 11.0317 0.735352C11.9973 0.987845 12.8453 1.362 13.4644 1.84766C14.0744 2.32629 14.507 2.95539 14.5161 3.69336H14.5171V8.53516C14.0843 8.32076 13.6108 8.17679 13.1108 8.11816C13.1831 7.96848 13.2162 7.82856 13.2163 7.70312V5.76758C12.6269 6.16618 11.8739 6.47995 11.0317 6.7002C10.0602 6.95423 8.96896 7.08494 7.8667 7.08496C6.76461 7.08493 5.67411 6.95415 4.70264 6.7002C3.85994 6.48006 3.10694 6.1662 2.51709 5.76758V7.70312L2.521 7.78418C2.56374 8.19554 2.93361 8.74414 3.91357 9.23145C4.9281 9.73585 6.35004 10.0371 7.8667 10.0371C8.26373 10.0371 8.6543 10.0141 9.03271 9.97461C8.75596 10.3799 8.54664 10.8349 8.42041 11.3232C8.23666 11.3313 8.0518 11.3369 7.8667 11.3369C6.20108 11.3369 4.57025 11.01 3.33447 10.3955C3.04163 10.2499 2.76658 10.0836 2.51709 9.90039V11.6738C2.51728 12.1379 2.88589 12.7556 3.92236 13.292C4.93457 13.8157 6.35342 14.1289 7.8667 14.1289C8.12318 14.1289 8.37694 14.1161 8.62646 14.0986C8.82021 14.5535 9.08999 14.9682 9.41943 15.3271C8.91285 15.3934 8.39149 15.4287 7.8667 15.4287C6.19761 15.4287 4.56379 15.0869 3.32568 14.4463C2.11244 13.8185 1.21649 12.8562 1.21631 11.6738V3.76367C1.21595 3.74853 1.21438 3.733 1.21436 3.71777C1.21436 2.96917 1.65103 2.33053 2.26807 1.84668C2.88747 1.36112 3.73675 0.987685 4.70264 0.735352C5.67413 0.481376 6.76457 0.349636 7.8667 0.349609ZM7.8667 1.65039C6.86269 1.65042 5.88326 1.77028 5.03076 1.99316C4.17183 2.2176 3.50421 2.52956 3.06982 2.87012C2.65043 3.19909 2.52622 3.48898 2.51709 3.69336V3.74414C2.52719 3.94845 2.65185 4.23772 3.06982 4.56543C3.50425 4.90601 4.17172 5.21795 5.03076 5.44238C5.88326 5.66527 6.8627 5.78513 7.8667 5.78516C8.8707 5.78513 9.85015 5.66525 10.7026 5.44238C11.5611 5.21787 12.2286 4.9049 12.6626 4.56445C13.0982 4.22252 13.2163 3.9231 13.2163 3.71777L13.2104 3.63574C13.1818 3.43623 13.044 3.16941 12.6626 2.87012C12.2286 2.52957 11.5614 2.21773 10.7026 1.99316C9.85009 1.77025 8.8708 1.65041 7.8667 1.65039Z\" fill=\"currentColor\"></path><path d=\"M12.8936 10.0361L13.2061 10.5566C13.2296 10.5959 13.2651 10.6562 13.3027 10.707C13.3469 10.7666 13.4148 10.8431 13.5195 10.9023C13.6244 10.9617 13.725 10.9801 13.7988 10.9873C13.8619 10.9934 13.9318 10.9932 13.9775 10.9932H14.6162L14.8896 11.4502L14.5947 11.9443C14.5698 11.9859 14.5312 12.0483 14.5029 12.1084C14.4781 12.1611 14.4514 12.2312 14.4395 12.3164L14.4326 12.4072L14.4395 12.4971C14.4514 12.5825 14.4781 12.6532 14.5029 12.7061C14.5312 12.7661 14.5689 12.8287 14.5938 12.8701L14.8896 13.3633L14.6162 13.8213H13.9775C13.9318 13.8213 13.8619 13.821 13.7988 13.8271C13.7433 13.8326 13.6728 13.8442 13.5967 13.875L13.5195 13.9121C13.4148 13.9714 13.3469 14.0478 13.3027 14.1074C13.265 14.1583 13.2296 14.2186 13.2061 14.2578L12.8936 14.7783H12.3115L11.999 14.2578C11.9755 14.2186 11.9401 14.1583 11.9023 14.1074C11.8693 14.0628 11.823 14.0083 11.7578 13.959L11.6855 13.9121L11.6074 13.875C11.5316 13.8445 11.4615 13.8325 11.4062 13.8271C11.3432 13.821 11.2733 13.8213 11.2275 13.8213H10.5889L10.3135 13.3633L10.6104 12.8701C10.6352 12.8287 10.6739 12.7661 10.7021 12.7061C10.7352 12.6357 10.7724 12.534 10.7725 12.4072C10.7724 12.2804 10.7352 12.1788 10.7021 12.1084C10.6739 12.0483 10.6353 11.9859 10.6104 11.9443L10.3135 11.4502L10.5889 10.9932H11.2275C11.2733 10.9932 11.3432 10.9934 11.4062 10.9873C11.4801 10.9801 11.5808 10.9616 11.6855 10.9023C11.7903 10.843 11.8582 10.7666 11.9023 10.707C11.94 10.6562 11.9755 10.5959 11.999 10.5566L12.3115 10.0361H12.8936Z\" stroke=\"currentColor\" stroke-miterlimit=\"10\"></path></svg>"},"database-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M13.1967 5.1869C13.7232 4.77378 14.0003 4.30517 14.0001 3.82819C14.0003 3.3512 13.7232 2.88259 13.1967 2.46947C12.6702 2.05635 11.9128 1.71328 11.0006 1.47475C10.0885 1.23621 9.05371 1.11062 8.00039 1.1106C6.94707 1.11057 5.9123 1.23612 5.00009 1.47461C4.08742 1.71301 3.32948 2.05604 2.80249 2.46919C2.2755 2.88235 1.99805 3.35106 1.99805 3.82819C1.99805 4.30531 2.2755 4.77402 2.80249 5.18718C3.32948 5.60033 4.08742 5.94336 5.00009 6.18176C5.9123 6.42025 6.94707 6.5458 8.00039 6.54578C9.05371 6.54575 10.0885 6.42016 11.0006 6.18163C11.9128 5.94309 12.6702 5.60002 13.1967 5.1869Z\" stroke=\"currentColor\"></path><path d=\"M2 3.80371V11.7848\" stroke=\"currentColor\"></path><path d=\"M14 3.80371V11.7848\" stroke=\"currentColor\"></path><path d=\"M2 7.81396C2 8.60524 2.63214 9.36411 3.75736 9.92363C4.88258 10.4832 6.4087 10.7975 8 10.7975C9.5913 10.7975 11.1174 10.4832 12.2426 9.92363C13.3679 9.36411 14 8.60524 14 7.81396\" stroke=\"currentColor\"></path><path d=\"M2 11.7847C2 12.6081 2.63214 13.3977 3.75736 13.98C4.88258 14.5622 6.4087 14.8893 8 14.8893C9.5913 14.8893 11.1174 14.5622 12.2426 13.98C13.3679 13.3977 14 12.6081 14 11.7847\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M13.1967 5.1869C13.7232 4.77378 14.0003 4.30517 14.0001 3.82819C14.0003 3.3512 13.7232 2.88259 13.1967 2.46947C12.6702 2.05635 11.9128 1.71328 11.0006 1.47475C10.0885 1.23621 9.05371 1.11062 8.00039 1.1106C6.94707 1.11057 5.9123 1.23612 5.00009 1.47461C4.08742 1.71301 3.32948 2.05604 2.80249 2.46919C2.2755 2.88235 1.99805 3.35106 1.99805 3.82819C1.99805 4.30531 2.2755 4.77402 2.80249 5.18718C3.32948 5.60033 4.08742 5.94336 5.00009 6.18176C5.9123 6.42025 6.94707 6.5458 8.00039 6.54578C9.05371 6.54575 10.0885 6.42016 11.0006 6.18163C11.9128 5.94309 12.6702 5.60002 13.1967 5.1869Z\" stroke=\"currentColor\"></path><path d=\"M2 3.80371V11.7848\" stroke=\"currentColor\"></path><path d=\"M14 3.80371V11.7848\" stroke=\"currentColor\"></path><path d=\"M2 7.81396C2 8.60524 2.63214 9.36411 3.75736 9.92363C4.88258 10.4832 6.4087 10.7975 8 10.7975C9.5913 10.7975 11.1174 10.4832 12.2426 9.92363C13.3679 9.36411 14 8.60524 14 7.81396\" stroke=\"currentColor\"></path><path d=\"M2 11.7847C2 12.6081 2.63214 13.3977 3.75736 13.98C4.88258 14.5622 6.4087 14.8893 8 14.8893C9.5913 14.8893 11.1174 14.5622 12.2426 13.98C13.3679 13.3977 14 12.6081 14 11.7847\" stroke=\"currentColor\"></path></svg>"},"clock-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 14C11.3137 14 14 11.3137 14 8C14 4.68629 11.3137 2 8 2C4.68629 2 2 4.68629 2 8C2 11.3137 4.68629 14 8 14Z\" stroke=\"currentColor\"></path><path d=\"M8 4.31V8.46L11 10.08\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 14C11.3137 14 14 11.3137 14 8C14 4.68629 11.3137 2 8 2C4.68629 2 2 4.68629 2 8C2 11.3137 4.68629 14 8 14Z\" stroke=\"currentColor\"></path><path d=\"M8 4.31V8.46L11 10.08\" stroke=\"currentColor\"></path></svg>"},"gauge-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3.4041 13.096C2.49514 12.187 1.87614 11.0288 1.62537 9.76798C1.37459 8.50716 1.50331 7.20028 1.99525 6.01261C2.48719 4.82494 3.32025 3.80981 4.3891 3.09557C5.45795 2.38134 6.71458 2.00008 8.0001 2C9.28563 2.00008 10.5423 2.38134 11.6111 3.09557C12.68 3.80981 13.513 4.82494 14.005 6.01261C14.4969 7.20028 14.6256 8.50716 14.3748 9.76798C14.1241 11.0288 13.5051 12.187 12.5961 13.096\" stroke=\"currentColor\"></path><path d=\"M8 8.49994L11.6114 4.88855\" stroke=\"currentColor\"></path><path d=\"M8 9.75C8.69036 9.75 9.25 9.19036 9.25 8.5C9.25 7.80964 8.69036 7.25 8 7.25C7.30964 7.25 6.75 7.80964 6.75 8.5C6.75 9.19036 7.30964 9.75 8 9.75Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3.4041 13.096C2.49514 12.187 1.87614 11.0288 1.62537 9.76798C1.37459 8.50716 1.50331 7.20028 1.99525 6.01261C2.48719 4.82494 3.32025 3.80981 4.3891 3.09557C5.45795 2.38134 6.71458 2.00008 8.0001 2C9.28563 2.00008 10.5423 2.38134 11.6111 3.09557C12.68 3.80981 13.513 4.82494 14.005 6.01261C14.4969 7.20028 14.6256 8.50716 14.3748 9.76798C14.1241 11.0288 13.5051 12.187 12.5961 13.096\" stroke=\"currentColor\"></path><path d=\"M8 8.49994L11.6114 4.88855\" stroke=\"currentColor\"></path><path d=\"M8 9.75C8.69036 9.75 9.25 9.19036 9.25 8.5C9.25 7.80964 8.69036 7.25 8 7.25C7.30964 7.25 6.75 7.80964 6.75 8.5C6.75 9.19036 7.30964 9.75 8 9.75Z\" fill=\"currentColor\"></path></svg>"},"send-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.97211 1.94476C7.55785 1.35914 8.50767 1.35919 9.09343 1.94476L13.921 6.77228L13.2138 7.47939L8.38632 2.65187C8.19108 2.45682 7.87443 2.45677 7.67922 2.65187L2.74397 7.58711L2.03687 6.88L6.97211 1.94476Z\" fill=\"currentColor\"></path><path d=\"M7.97571 14.5732L8.02421 2.34139\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.97211 1.94476C7.55785 1.35914 8.50767 1.35919 9.09343 1.94476L13.921 6.77228L13.2138 7.47939L8.38632 2.65187C8.19108 2.45682 7.87443 2.45677 7.67922 2.65187L2.74397 7.58711L2.03687 6.88L6.97211 1.94476Z\" fill=\"currentColor\"></path><path d=\"M7.97571 14.5732L8.02421 2.34139\" stroke=\"currentColor\"></path></svg>"},"queue-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M5 6.75H11\" stroke=\"currentColor\"></path><path d=\"M5 9H8\" stroke=\"currentColor\"></path><path d=\"M2.37067 11.2497C1.5872 9.89252 1.32042 8.29798 1.61945 6.7597C1.91847 5.22141 2.76317 3.84293 3.99801 2.87809C5.23285 1.91325 6.7747 1.427 8.33964 1.50888C9.90458 1.59076 11.3873 2.23526 12.5147 3.32369C13.6422 4.41232 14.3384 5.8717 14.4751 7.43304C14.6118 8.99438 14.1797 10.5525 13.2585 11.8205C12.3372 13.0885 10.9889 13.9809 9.4617 14.3334C8.18666 14.6277 6.8587 14.529 5.64964 14.0601C5.17095 13.8745 4.76937 13.4929 4.26509 13.3963C3.67389 13.2832 2.95232 13.5595 2.0377 14.3334\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M5 6.75H11\" stroke=\"currentColor\"></path><path d=\"M5 9H8\" stroke=\"currentColor\"></path><path d=\"M2.37067 11.2497C1.5872 9.89252 1.32042 8.29798 1.61945 6.7597C1.91847 5.22141 2.76317 3.84293 3.99801 2.87809C5.23285 1.91325 6.7747 1.427 8.33964 1.50888C9.90458 1.59076 11.3873 2.23526 12.5147 3.32369C13.6422 4.41232 14.3384 5.8717 14.4751 7.43304C14.6118 8.99438 14.1797 10.5525 13.2585 11.8205C12.3372 13.0885 10.9889 13.9809 9.4617 14.3334C8.18666 14.6277 6.8587 14.529 5.64964 14.0601C5.17095 13.8745 4.76937 13.4929 4.26509 13.3963C3.67389 13.2832 2.95232 13.5595 2.0377 14.3334\" stroke=\"currentColor\"></path></svg>"},"checklist-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M3.75 6.25C4.7165 6.25 5.5 5.4665 5.5 4.5C5.5 3.5335 4.7165 2.75 3.75 2.75C2.7835 2.75 2 3.5335 2 4.5C2 5.4665 2.7835 6.25 3.75 6.25Z\" stroke=\"currentColor\"></path><path d=\"M7.5 4.5H13.5\" stroke=\"currentColor\"></path><path d=\"M3.75 13.25C4.7165 13.25 5.5 12.4665 5.5 11.5C5.5 10.5335 4.7165 9.75 3.75 9.75C2.7835 9.75 2 10.5335 2 11.5C2 12.4665 2.7835 13.25 3.75 13.25Z\" stroke=\"currentColor\"></path><path d=\"M7.5 11.5H13.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M3.75 6.25C4.7165 6.25 5.5 5.4665 5.5 4.5C5.5 3.5335 4.7165 2.75 3.75 2.75C2.7835 2.75 2 3.5335 2 4.5C2 5.4665 2.7835 6.25 3.75 6.25Z\" stroke=\"currentColor\"></path><path d=\"M7.5 4.5H13.5\" stroke=\"currentColor\"></path><path d=\"M3.75 13.25C4.7165 13.25 5.5 12.4665 5.5 11.5C5.5 10.5335 4.7165 9.75 3.75 9.75C2.7835 9.75 2 10.5335 2 11.5C2 12.4665 2.7835 13.25 3.75 13.25Z\" stroke=\"currentColor\"></path><path d=\"M7.5 11.5H13.5\" stroke=\"currentColor\"></path></svg>"},"list-pen-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.9375 5.90295H11.0625\" stroke=\"currentColor\"></path><path d=\"M4.9375 9.02991H8.27841\" stroke=\"currentColor\"></path><path d=\"M12.5 1.32617C13.3039 1.32617 14 1.95171 14 2.77637V7.61328L13 8.68164V2.77637C13 2.55186 12.8007 2.32617 12.5 2.32617H3.5C3.1993 2.32617 3 2.55186 3 2.77637V13.2246C3.00044 13.4489 3.19963 13.6738 3.5 13.6738H8.32812L7.39258 14.6738H3.5C2.69637 14.6738 2.00042 14.0489 2 13.2246V2.77637C2 1.95171 2.69613 1.32617 3.5 1.32617H12.5Z\" fill=\"currentColor\"></path><path d=\"M8.97212 14.3693C9.17511 14.5723 9.37811 14.7753 9.5811 14.9783C9.67012 14.8953 9.75914 14.8123 9.84815 14.7293C11.4505 13.2352 13.0528 11.7411 14.6551 10.247C14.7441 10.164 14.8331 10.081 14.9221 9.99803C14.5989 9.6748 14.2756 9.35157 13.9524 9.02834C13.8694 9.11736 13.7864 9.20637 13.7034 9.29539C12.2093 10.8977 10.7152 12.5 9.22113 14.1023C9.13813 14.1913 9.05513 14.2803 8.97212 14.3693Z\" fill=\"currentColor\"></path><path d=\"M11.6323 13.7841C11.6323 14.0395 11.6323 14.295 11.6323 14.5504C11.6812 14.5523 11.7301 14.5543 11.779 14.5562C12.659 14.5913 13.539 14.6263 14.419 14.6614C14.4679 14.6633 14.5168 14.6653 14.5657 14.6672C14.5657 14.3339 14.5657 14.0006 14.5657 13.6672C14.5168 13.6692 14.4679 13.6711 14.419 13.6731C13.539 13.7081 12.659 13.7432 11.779 13.7783C11.7301 13.7802 11.6812 13.7821 11.6323 13.7841Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.9375 5.90295H11.0625\" stroke=\"currentColor\"></path><path d=\"M4.9375 9.02991H8.27841\" stroke=\"currentColor\"></path><path d=\"M12.5 1.32617C13.3039 1.32617 14 1.95171 14 2.77637V7.61328L13 8.68164V2.77637C13 2.55186 12.8007 2.32617 12.5 2.32617H3.5C3.1993 2.32617 3 2.55186 3 2.77637V13.2246C3.00044 13.4489 3.19963 13.6738 3.5 13.6738H8.32812L7.39258 14.6738H3.5C2.69637 14.6738 2.00042 14.0489 2 13.2246V2.77637C2 1.95171 2.69613 1.32617 3.5 1.32617H12.5Z\" fill=\"currentColor\"></path><path d=\"M8.97212 14.3693C9.17511 14.5723 9.37811 14.7753 9.5811 14.9783C9.67012 14.8953 9.75914 14.8123 9.84815 14.7293C11.4505 13.2352 13.0528 11.7411 14.6551 10.247C14.7441 10.164 14.8331 10.081 14.9221 9.99803C14.5989 9.6748 14.2756 9.35157 13.9524 9.02834C13.8694 9.11736 13.7864 9.20637 13.7034 9.29539C12.2093 10.8977 10.7152 12.5 9.22113 14.1023C9.13813 14.1913 9.05513 14.2803 8.97212 14.3693Z\" fill=\"currentColor\"></path><path d=\"M11.6323 13.7841C11.6323 14.0395 11.6323 14.295 11.6323 14.5504C11.6812 14.5523 11.7301 14.5543 11.779 14.5562C12.659 14.5913 13.539 14.6263 14.419 14.6614C14.4679 14.6633 14.5168 14.6653 14.5657 14.6672C14.5657 14.3339 14.5657 14.0006 14.5657 13.6672C14.5168 13.6692 14.4679 13.6711 14.419 13.6731C13.539 13.7081 12.659 13.7432 11.779 13.7783C11.7301 13.7802 11.6812 13.7821 11.6323 13.7841Z\" fill=\"currentColor\"></path></svg>"},"goal-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M14.5001 8C14.5 9.28552 14.1188 10.5422 13.4045 11.611C12.6903 12.6799 11.6752 13.5129 10.4875 14.0049C9.29982 14.4968 7.99295 14.6255 6.73212 14.3747C5.4713 14.124 4.31314 13.505 3.4041 12.596C2.49514 11.687 1.87614 10.5288 1.62537 9.26798C1.37459 8.00716 1.50331 6.70028 1.99525 5.51261C2.48719 4.32494 3.32025 3.30981 4.3891 2.59557C5.45795 1.88134 6.71458 1.50008 8.0001 1.5\" stroke=\"currentColor\"></path><path d=\"M11.5 8C11.5001 8.69227 11.2948 9.36901 10.9102 9.94463C10.5257 10.5202 9.97901 10.9689 9.33944 11.2338C8.69986 11.4987 7.99609 11.5681 7.31712 11.433C6.63816 11.2979 6.01449 10.9645 5.52501 10.475C5.03548 9.98552 4.70209 9.36185 4.56702 8.68289C4.43195 8.00392 4.50127 7.30015 4.76619 6.66057C5.03112 6.021 5.47976 5.47436 6.05538 5.08978C6.631 4.70519 7.30774 4.49995 8.00001 4.5\" stroke=\"currentColor\"></path><path d=\"M8.00024 7.99976L11.2 4.80005\" stroke=\"currentColor\"></path><path d=\"M12.4719 5.62245C12.4246 5.66972 12.3569 5.69025 12.2913 5.67715L10.7814 5.37555C10.7022 5.35972 10.6402 5.29781 10.6244 5.2186L10.3228 3.70866C10.3097 3.6431 10.3302 3.57533 10.3775 3.52806L12.1826 1.723C12.2863 1.61929 12.4627 1.65879 12.5122 1.79684L12.9271 2.95225C12.9472 3.00847 12.9915 3.05272 13.0477 3.07291L14.2031 3.48774C14.3412 3.5373 14.3807 3.71368 14.277 3.81739L12.4719 5.62245Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M14.5001 8C14.5 9.28552 14.1188 10.5422 13.4045 11.611C12.6903 12.6799 11.6752 13.5129 10.4875 14.0049C9.29982 14.4968 7.99295 14.6255 6.73212 14.3747C5.4713 14.124 4.31314 13.505 3.4041 12.596C2.49514 11.687 1.87614 10.5288 1.62537 9.26798C1.37459 8.00716 1.50331 6.70028 1.99525 5.51261C2.48719 4.32494 3.32025 3.30981 4.3891 2.59557C5.45795 1.88134 6.71458 1.50008 8.0001 1.5\" stroke=\"currentColor\"></path><path d=\"M11.5 8C11.5001 8.69227 11.2948 9.36901 10.9102 9.94463C10.5257 10.5202 9.97901 10.9689 9.33944 11.2338C8.69986 11.4987 7.99609 11.5681 7.31712 11.433C6.63816 11.2979 6.01449 10.9645 5.52501 10.475C5.03548 9.98552 4.70209 9.36185 4.56702 8.68289C4.43195 8.00392 4.50127 7.30015 4.76619 6.66057C5.03112 6.021 5.47976 5.47436 6.05538 5.08978C6.631 4.70519 7.30774 4.49995 8.00001 4.5\" stroke=\"currentColor\"></path><path d=\"M8.00024 7.99976L11.2 4.80005\" stroke=\"currentColor\"></path><path d=\"M12.4719 5.62245C12.4246 5.66972 12.3569 5.69025 12.2913 5.67715L10.7814 5.37555C10.7022 5.35972 10.6402 5.29781 10.6244 5.2186L10.3228 3.70866C10.3097 3.6431 10.3302 3.57533 10.3775 3.52806L12.1826 1.723C12.2863 1.61929 12.4627 1.65879 12.5122 1.79684L12.9271 2.95225C12.9472 3.00847 12.9915 3.05272 13.0477 3.07291L14.2031 3.48774C14.3412 3.5373 14.3807 3.71368 14.277 3.81739L12.4719 5.62245Z\" stroke=\"currentColor\"></path></svg>"},"sparkle":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M5.875 3C5.875 6.33333 7.54167 8 10.875 8C7.54167 8 5.875 9.66667 5.875 13C5.875 9.66667 4.20833 8 0.875 8C4.20833 8 5.875 6.33333 5.875 3Z\" stroke=\"currentColor\"></path><path d=\"M12.375 1.55823C12.375 3.39156 13.2917 4.30823 15.125 4.30823C13.2917 4.30823 12.375 5.22489 12.375 7.05823C12.375 5.22489 11.4583 4.30823 9.625 4.30823C11.4583 4.30823 12.375 3.39156 12.375 1.55823Z\" stroke=\"currentColor\"></path><path d=\"M12.375 10.4418C12.375 11.7751 13.0417 12.4418 14.375 12.4418C13.0417 12.4418 12.375 13.1084 12.375 14.4418C12.375 13.1084 11.7083 12.4418 10.375 12.4418C11.7083 12.4418 12.375 11.7751 12.375 10.4418Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M5.875 3C5.875 6.33333 7.54167 8 10.875 8C7.54167 8 5.875 9.66667 5.875 13C5.875 9.66667 4.20833 8 0.875 8C4.20833 8 5.875 6.33333 5.875 3Z\" stroke=\"currentColor\"></path><path d=\"M12.375 1.55823C12.375 3.39156 13.2917 4.30823 15.125 4.30823C13.2917 4.30823 12.375 5.22489 12.375 7.05823C12.375 5.22489 11.4583 4.30823 9.625 4.30823C11.4583 4.30823 12.375 3.39156 12.375 1.55823Z\" stroke=\"currentColor\"></path><path d=\"M12.375 10.4418C12.375 11.7751 13.0417 12.4418 14.375 12.4418C13.0417 12.4418 12.375 13.1084 12.375 14.4418C12.375 13.1084 11.7083 12.4418 10.375 12.4418C11.7083 12.4418 12.375 11.7751 12.375 10.4418Z\" stroke=\"currentColor\"></path></svg>"},"inspect-outline":{"size":12,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.67398 4.25061L1.36094 7.86484C1.29085 7.9413 1.29085 8.05866 1.36094 8.13513L4.67398 11.7494\" stroke=\"currentColor\"></path><path d=\"M11.3262 4.25061L14.6392 7.86484C14.7093 7.9413 14.7093 8.05866 14.6392 8.13513L11.3262 11.7494\" stroke=\"currentColor\"></path><path d=\"M9.56222 3.62573L6.43774 12.3743\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.67398 4.25061L1.36094 7.86484C1.29085 7.9413 1.29085 8.05866 1.36094 8.13513L4.67398 11.7494\" stroke=\"currentColor\"></path><path d=\"M11.3262 4.25061L14.6392 7.86484C14.7093 7.9413 14.7093 8.05866 14.6392 8.13513L11.3262 11.7494\" stroke=\"currentColor\"></path><path d=\"M9.56222 3.62573L6.43774 12.3743\" stroke=\"currentColor\"></path></svg>"},"skill-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 17 17\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.57788 5.77124H10.7029\" stroke=\"currentColor\"></path><path d=\"M4.57788 8.89819H7.91879\" stroke=\"currentColor\"></path><path d=\"M12.1404 1.19446C12.9442 1.19446 13.6404 1.81999 13.6404 2.64465V8.89856H12.6404V2.64465C12.6404 2.42015 12.4411 2.19446 12.1404 2.19446H3.14038C2.83968 2.19446 2.64038 2.42015 2.64038 2.64465V13.0929C2.64082 13.3172 2.84001 13.5421 3.14038 13.5421H8.88159V14.5421H3.14038C2.33675 14.5421 1.6408 13.9172 1.64038 13.0929V2.64465C1.64038 1.81999 2.33651 1.19446 3.14038 1.19446H12.1404Z\" fill=\"currentColor\"></path><path d=\"M12.0051 15.1056C12.0051 13.6395 10.8166 12.451 9.35059 12.451C10.8166 12.451 12.0051 11.2626 12.0051 9.79651C12.0051 11.2626 13.1936 12.451 14.6597 12.451C13.1936 12.451 12.0051 13.6395 12.0051 15.1056Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 17 17\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.57788 5.77124H10.7029\" stroke=\"currentColor\"></path><path d=\"M4.57788 8.89819H7.91879\" stroke=\"currentColor\"></path><path d=\"M12.1404 1.19446C12.9442 1.19446 13.6404 1.81999 13.6404 2.64465V8.89856H12.6404V2.64465C12.6404 2.42015 12.4411 2.19446 12.1404 2.19446H3.14038C2.83968 2.19446 2.64038 2.42015 2.64038 2.64465V13.0929C2.64082 13.3172 2.84001 13.5421 3.14038 13.5421H8.88159V14.5421H3.14038C2.33675 14.5421 1.6408 13.9172 1.64038 13.0929V2.64465C1.64038 1.81999 2.33651 1.19446 3.14038 1.19446H12.1404Z\" fill=\"currentColor\"></path><path d=\"M12.0051 15.1056C12.0051 13.6395 10.8166 12.451 9.35059 12.451C10.8166 12.451 12.0051 11.2626 12.0051 9.79651C12.0051 11.2626 13.1936 12.451 14.6597 12.451C13.1936 12.451 12.0051 13.6395 12.0051 15.1056Z\" stroke=\"currentColor\"></path></svg>"},"question-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M5.75 6.69646C5.75 6.29865 5.88196 5.90976 6.12919 5.57899C6.37643 5.24821 6.72783 4.99041 7.13896 4.83817C7.5501 4.68593 8.0025 4.6461 8.43895 4.72371C8.87541 4.80132 9.27632 4.99289 9.59099 5.27419C9.90566 5.55549 10.12 5.91388 10.2068 6.30406C10.2936 6.69423 10.249 7.09866 10.0787 7.4662C9.90843 7.83373 9.62004 8.14787 9.25003 8.36889C9.19476 8.4019 9.13803 8.43262 9.08004 8.46099C8.52566 8.73217 8 9.20817 8 9.82532\" stroke=\"currentColor\"></path><path d=\"M8 10.7416V11.7416\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M5.75 6.69646C5.75 6.29865 5.88196 5.90976 6.12919 5.57899C6.37643 5.24821 6.72783 4.99041 7.13896 4.83817C7.5501 4.68593 8.0025 4.6461 8.43895 4.72371C8.87541 4.80132 9.27632 4.99289 9.59099 5.27419C9.90566 5.55549 10.12 5.91388 10.2068 6.30406C10.2936 6.69423 10.249 7.09866 10.0787 7.4662C9.90843 7.83373 9.62004 8.14787 9.25003 8.36889C9.19476 8.4019 9.13803 8.43262 9.08004 8.46099C8.52566 8.73217 8 9.20817 8 9.82532\" stroke=\"currentColor\"></path><path d=\"M8 10.7416V11.7416\" stroke=\"currentColor\"></path></svg>"},"info-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 14 14\" fill=\"none\" stroke-width=\"1\"><path d=\"M12.5757 7.00012C12.5757 3.92085 10.0794 1.42463 7.00012 1.42456C3.9208 1.42456 1.42456 3.9208 1.42456 7.00012C1.42463 10.0794 3.92085 12.5757 7.00012 12.5757C10.0793 12.5756 12.5756 10.0793 12.5757 7.00012ZM13.8002 7.00012C13.8001 10.7559 10.7559 13.8001 7.00012 13.8002C3.2443 13.8002 0.199291 10.7559 0.199219 7.00012C0.199219 3.24426 3.24426 0.199219 7.00012 0.199219C10.7559 0.199291 13.8002 3.2443 13.8002 7.00012Z\" fill=\"currentColor\"></path><path d=\"M7.6127 3.18921V4.55986H6.38735V3.18921H7.6127Z\" fill=\"currentColor\"></path><path d=\"M7.6127 5.68921V10.8109H6.38735V5.68921H7.6127Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 14 14\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12.5757 7.00012C12.5757 3.92085 10.0794 1.42463 7.00012 1.42456C3.9208 1.42456 1.42456 3.9208 1.42456 7.00012C1.42463 10.0794 3.92085 12.5757 7.00012 12.5757C10.0793 12.5756 12.5756 10.0793 12.5757 7.00012ZM13.8002 7.00012C13.8001 10.7559 10.7559 13.8001 7.00012 13.8002C3.2443 13.8002 0.199291 10.7559 0.199219 7.00012C0.199219 3.24426 3.24426 0.199219 7.00012 0.199219C10.7559 0.199291 13.8002 3.2443 13.8002 7.00012Z\" fill=\"currentColor\"></path><path d=\"M7.6127 3.18921V4.55986H6.38735V3.18921H7.6127Z\" fill=\"currentColor\"></path><path d=\"M7.6127 5.68921V10.8109H6.38735V5.68921H7.6127Z\" fill=\"currentColor\"></path></svg>"},"plugin-pinwheel-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M7.84457 5.06199C11.6605 4.93876 14.7962 6.14848 14.8484 7.76397C14.8875 8.97461 13.1838 10.0696 10.7215 10.5942\" stroke=\"currentColor\"></path><path d=\"M5.12742 8.07731C5.00419 4.26138 6.21391 1.12568 7.8294 1.07351C9.04004 1.03441 10.135 2.73808 10.6596 5.20037\" stroke=\"currentColor\"></path><path d=\"M8.02457 10.6802C4.20865 10.8034 1.07294 9.5937 1.02077 7.97821C0.981678 6.76758 2.68535 5.67262 5.14763 5.14798\" stroke=\"currentColor\"></path><path d=\"M10.7476 7.89535C10.8708 11.7113 9.66109 14.847 8.0456 14.8991C6.83496 14.9382 5.74 13.2346 5.21536 10.7723\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M7.84457 5.06199C11.6605 4.93876 14.7962 6.14848 14.8484 7.76397C14.8875 8.97461 13.1838 10.0696 10.7215 10.5942\" stroke=\"currentColor\"></path><path d=\"M5.12742 8.07731C5.00419 4.26138 6.21391 1.12568 7.8294 1.07351C9.04004 1.03441 10.135 2.73808 10.6596 5.20037\" stroke=\"currentColor\"></path><path d=\"M8.02457 10.6802C4.20865 10.8034 1.07294 9.5937 1.02077 7.97821C0.981678 6.76758 2.68535 5.67262 5.14763 5.14798\" stroke=\"currentColor\"></path><path d=\"M10.7476 7.89535C10.8708 11.7113 9.66109 14.847 8.0456 14.8991C6.83496 14.9382 5.74 13.2346 5.21536 10.7723\" stroke=\"currentColor\"></path></svg>"},"alarm-clock-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 17 17\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.09372 11.9895L3.11865 14.0387\" stroke=\"currentColor\"></path><path d=\"M12.1392 11.9895L13.1143 14.0387\" stroke=\"currentColor\"></path><path d=\"M8.11646 4.78442V8.03442L10.6165 9.53442\" stroke=\"currentColor\"></path><path d=\"M8.11646 13.4094C11.154 13.4094 13.6165 10.947 13.6165 7.90942C13.6165 4.87186 11.154 2.40942 8.11646 2.40942C5.07889 2.40942 2.61646 4.87186 2.61646 7.90942C2.61646 10.947 5.07889 13.4094 8.11646 13.4094Z\" stroke=\"currentColor\"></path><path d=\"M1.75952 4.74323C2.30657 3.65639 3.12646 2.73047 4.12926 2.05542\" stroke=\"currentColor\"></path><path d=\"M14.3345 4.74323C13.7874 3.65639 12.9675 2.73047 11.9647 2.05542\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 17 17\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.09372 11.9895L3.11865 14.0387\" stroke=\"currentColor\"></path><path d=\"M12.1392 11.9895L13.1143 14.0387\" stroke=\"currentColor\"></path><path d=\"M8.11646 4.78442V8.03442L10.6165 9.53442\" stroke=\"currentColor\"></path><path d=\"M8.11646 13.4094C11.154 13.4094 13.6165 10.947 13.6165 7.90942C13.6165 4.87186 11.154 2.40942 8.11646 2.40942C5.07889 2.40942 2.61646 4.87186 2.61646 7.90942C2.61646 10.947 5.07889 13.4094 8.11646 13.4094Z\" stroke=\"currentColor\"></path><path d=\"M1.75952 4.74323C2.30657 3.65639 3.12646 2.73047 4.12926 2.05542\" stroke=\"currentColor\"></path><path d=\"M14.3345 4.74323C13.7874 3.65639 12.9675 2.73047 11.9647 2.05542\" stroke=\"currentColor\"></path></svg>"},"archive-outline":{"size":20,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M13.5 2.5H2.5C1.94772 2.5 1.5 2.94772 1.5 3.5V4.5C1.5 5.05228 1.94772 5.5 2.5 5.5H13.5C14.0523 5.5 14.5 5.05228 14.5 4.5V3.5C14.5 2.94772 14.0523 2.5 13.5 2.5Z\" stroke=\"currentColor\"></path><path d=\"M2.5 5.5V13.5C2.5 13.7652 2.60536 14.0196 2.79289 14.2071C2.98043 14.3946 3.23478 14.5 3.5 14.5H12.5C12.7652 14.5 13.0196 14.3946 13.2071 14.2071C13.3946 14.0196 13.5 13.7652 13.5 13.5V5.5\" stroke=\"currentColor\"></path><path d=\"M6.5 9.5H9.5\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M13.5 2.5H2.5C1.94772 2.5 1.5 2.94772 1.5 3.5V4.5C1.5 5.05228 1.94772 5.5 2.5 5.5H13.5C14.0523 5.5 14.5 5.05228 14.5 4.5V3.5C14.5 2.94772 14.0523 2.5 13.5 2.5Z\" stroke=\"currentColor\"></path><path d=\"M2.5 5.5V13.5C2.5 13.7652 2.60536 14.0196 2.79289 14.2071C2.98043 14.3946 3.23478 14.5 3.5 14.5H12.5C12.7652 14.5 13.0196 14.3946 13.2071 14.2071C13.3946 14.0196 13.5 13.7652 13.5 13.5V5.5\" stroke=\"currentColor\"></path><path d=\"M6.5 9.5H9.5\" stroke=\"currentColor\"></path></svg>"},"wrap-lines-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M2.3457 3.19299H13.6541\" stroke=\"currentColor\"></path><path d=\"M2.3457 7.46497H9.19332\" stroke=\"currentColor\"></path><path d=\"M2.3457 11.7369H6.4849\" stroke=\"currentColor\"></path><path d=\"M9.1936 7.46497H11.5183C12.6981 7.46497 13.6544 8.42132 13.6544 9.60103C13.6544 10.7808 12.6981 11.7371 11.5183 11.7371H9.1936\" stroke=\"currentColor\"></path><path d=\"M10.9505 9.7677L9.12262 11.5956C9.04452 11.6737 9.04452 11.8003 9.12262 11.8784L10.9505 13.7063\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M2.3457 3.19299H13.6541\" stroke=\"currentColor\"></path><path d=\"M2.3457 7.46497H9.19332\" stroke=\"currentColor\"></path><path d=\"M2.3457 11.7369H6.4849\" stroke=\"currentColor\"></path><path d=\"M9.1936 7.46497H11.5183C12.6981 7.46497 13.6544 8.42132 13.6544 9.60103C13.6544 10.7808 12.6981 11.7371 11.5183 11.7371H9.1936\" stroke=\"currentColor\"></path><path d=\"M10.9505 9.7677L9.12262 11.5956C9.04452 11.6737 9.04452 11.8003 9.12262 11.8784L10.9505 13.7063\" stroke=\"currentColor\"></path></svg>"},"nowrap-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\"><path d=\"M2 15H1V1H2V15Z\" fill=\"currentColor\"></path><path d=\"M12.3535 7.64645C12.5487 7.84171 12.5487 8.15829 12.3535 8.35355L9.85352 10.8535L9.14648 10.1465L10.793 8.5H3.5V7.5H10.793L9.14648 5.85352L9.85352 5.14648L12.3535 7.64645Z\" fill=\"currentColor\"></path><path d=\"M15 15H14V1H15V15Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\"><path d=\"M2 15H1V1H2V15Z\" fill=\"currentColor\"></path><path d=\"M12.3535 7.64645C12.5487 7.84171 12.5487 8.15829 12.3535 8.35355L9.85352 10.8535L9.14648 10.1465L10.793 8.5H3.5V7.5H10.793L9.14648 5.85352L9.85352 5.14648L12.3535 7.64645Z\" fill=\"currentColor\"></path><path d=\"M15 15H14V1H15V15Z\" fill=\"currentColor\"></path></svg>"},"wrap-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\"><path d=\"M10.9999 8C10.9999 6.89543 10.1046 6 9 6H4.5V5H9C10.6568 5 11.9999 6.34315 11.9999 8C11.9999 9.65685 10.6568 11 9 11H6.20703L6.85351 11.6465L6.14648 12.3535L4.64652 10.8536C4.45126 10.6583 4.45126 10.3417 4.64652 10.1464L6.14648 8.64648L6.85351 9.35352L6.20703 10H9C10.1046 10 10.9999 9.10457 10.9999 8Z\" fill=\"currentColor\"></path><path d=\"M2 15H1V1H2V15Z\" fill=\"currentColor\"></path><path d=\"M15 15H14V1H15V15Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\"><path d=\"M10.9999 8C10.9999 6.89543 10.1046 6 9 6H4.5V5H9C10.6568 5 11.9999 6.34315 11.9999 8C11.9999 9.65685 10.6568 11 9 11H6.20703L6.85351 11.6465L6.14648 12.3535L4.64652 10.8536C4.45126 10.6583 4.45126 10.3417 4.64652 10.1464L6.14648 8.64648L6.85351 9.35352L6.20703 10H9C10.1046 10 10.9999 9.10457 10.9999 8Z\" fill=\"currentColor\"></path><path d=\"M2 15H1V1H2V15Z\" fill=\"currentColor\"></path><path d=\"M15 15H14V1H15V15Z\" fill=\"currentColor\"></path></svg>"},"compare-split-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6 1.5H2.5C1.94772 1.5 1.5 1.94772 1.5 2.5V13.5C1.5 14.0523 1.94772 14.5 2.5 14.5H6C6.55228 14.5 7 14.0523 7 13.5V2.5C7 1.94772 6.55228 1.5 6 1.5Z\" stroke=\"currentColor\"></path><path d=\"M13.5 1.5H10C9.44772 1.5 9 1.94772 9 2.5V13.5C9 14.0523 9.44772 14.5 10 14.5H13.5C14.0523 14.5 14.5 14.0523 14.5 13.5V2.5C14.5 1.94772 14.0523 1.5 13.5 1.5Z\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6 1.5H2.5C1.94772 1.5 1.5 1.94772 1.5 2.5V13.5C1.5 14.0523 1.94772 14.5 2.5 14.5H6C6.55228 14.5 7 14.0523 7 13.5V2.5C7 1.94772 6.55228 1.5 6 1.5Z\" stroke=\"currentColor\"></path><path d=\"M13.5 1.5H10C9.44772 1.5 9 1.94772 9 2.5V13.5C9 14.0523 9.44772 14.5 10 14.5H13.5C14.0523 14.5 14.5 14.0523 14.5 13.5V2.5C14.5 1.94772 14.0523 1.5 13.5 1.5Z\" stroke=\"currentColor\"></path></svg>"},"plan-outline":{"size":14,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M4.9375 5.90295H11.0625\" stroke=\"currentColor\"></path><path d=\"M4.9375 9.02991H8.27841\" stroke=\"currentColor\"></path><path d=\"M12.5 1.32617C13.3039 1.32617 14 1.95171 14 2.77637V7.61328L13 8.68164V2.77637C13 2.55186 12.8007 2.32617 12.5 2.32617H3.5C3.1993 2.32617 3 2.55186 3 2.77637V13.2246C3.00044 13.4489 3.19963 13.6738 3.5 13.6738H8.32812L7.39258 14.6738H3.5C2.69637 14.6738 2.00042 14.0489 2 13.2246V2.77637C2 1.95171 2.69613 1.32617 3.5 1.32617H12.5Z\" fill=\"currentColor\"></path><path d=\"M8.97212 14.3693C9.17511 14.5723 9.37811 14.7753 9.5811 14.9783C9.67012 14.8953 9.75914 14.8123 9.84815 14.7293C11.4505 13.2352 13.0528 11.7411 14.6551 10.247C14.7441 10.164 14.8331 10.081 14.9221 9.99803C14.5989 9.6748 14.2756 9.35157 13.9524 9.02834C13.8694 9.11736 13.7864 9.20637 13.7034 9.29539C12.2093 10.8977 10.7152 12.5 9.22113 14.1023C9.13813 14.1913 9.05513 14.2803 8.97212 14.3693Z\" fill=\"currentColor\"></path><path d=\"M11.6323 13.7841C11.6323 14.0395 11.6323 14.295 11.6323 14.5504C11.6812 14.5523 11.7301 14.5543 11.779 14.5562C12.659 14.5913 13.539 14.6263 14.419 14.6614C14.4679 14.6633 14.5168 14.6653 14.5657 14.6672C14.5657 14.3339 14.5657 14.0006 14.5657 13.6672C14.5168 13.6692 14.4679 13.6711 14.419 13.6731C13.539 13.7081 12.659 13.7432 11.779 13.7783C11.7301 13.7802 11.6812 13.7821 11.6323 13.7841Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M4.9375 5.90295H11.0625\" stroke=\"currentColor\"></path><path d=\"M4.9375 9.02991H8.27841\" stroke=\"currentColor\"></path><path d=\"M12.5 1.32617C13.3039 1.32617 14 1.95171 14 2.77637V7.61328L13 8.68164V2.77637C13 2.55186 12.8007 2.32617 12.5 2.32617H3.5C3.1993 2.32617 3 2.55186 3 2.77637V13.2246C3.00044 13.4489 3.19963 13.6738 3.5 13.6738H8.32812L7.39258 14.6738H3.5C2.69637 14.6738 2.00042 14.0489 2 13.2246V2.77637C2 1.95171 2.69613 1.32617 3.5 1.32617H12.5Z\" fill=\"currentColor\"></path><path d=\"M8.97212 14.3693C9.17511 14.5723 9.37811 14.7753 9.5811 14.9783C9.67012 14.8953 9.75914 14.8123 9.84815 14.7293C11.4505 13.2352 13.0528 11.7411 14.6551 10.247C14.7441 10.164 14.8331 10.081 14.9221 9.99803C14.5989 9.6748 14.2756 9.35157 13.9524 9.02834C13.8694 9.11736 13.7864 9.20637 13.7034 9.29539C12.2093 10.8977 10.7152 12.5 9.22113 14.1023C9.13813 14.1913 9.05513 14.2803 8.97212 14.3693Z\" fill=\"currentColor\"></path><path d=\"M11.6323 13.7841C11.6323 14.0395 11.6323 14.295 11.6323 14.5504C11.6812 14.5523 11.7301 14.5543 11.779 14.5562C12.659 14.5913 13.539 14.6263 14.419 14.6614C14.4679 14.6633 14.5168 14.6653 14.5657 14.6672C14.5657 14.3339 14.5657 14.0006 14.5657 13.6672C14.5168 13.6692 14.4679 13.6711 14.419 13.6731C13.539 13.7081 12.659 13.7432 11.779 13.7783C11.7301 13.7802 11.6812 13.7821 11.6323 13.7841Z\" fill=\"currentColor\"></path></svg>"},"compact-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path opacity=\"0.35\" d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M8 1.5C8.85359 1.5 9.69883 1.66813 10.4874 1.99478C11.2761 2.32144 11.9926 2.80022 12.5962 3.40381C13.1998 4.00739 13.6786 4.72394 14.0052 5.51256C14.3319 6.30117 14.5 7.14641 14.5 8\" stroke=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path opacity=\"0.35\" d=\"M8 14.5C11.5899 14.5 14.5 11.5899 14.5 8C14.5 4.41015 11.5899 1.5 8 1.5C4.41015 1.5 1.5 4.41015 1.5 8C1.5 11.5899 4.41015 14.5 8 14.5Z\" stroke=\"currentColor\"></path><path d=\"M8 1.5C8.85359 1.5 9.69883 1.66813 10.4874 1.99478C11.2761 2.32144 11.9926 2.80022 12.5962 3.40381C13.1998 4.00739 13.6786 4.72394 14.0052 5.51256C14.3319 6.30117 14.5 7.14641 14.5 8\" stroke=\"currentColor\"></path></svg>"},"shield-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M6.80132 2.14853C7.70663 1.80917 8.70422 1.80919 9.60952 2.14859L14.1296 3.84317V7.11961C14.1296 11.6089 10.7615 13.5975 8.20543 14.5779C5.64931 13.5975 2.28052 11.6089 2.28052 7.11961V3.84317L6.80132 2.14853Z\" stroke=\"currentColor\" stroke-linejoin=\"round\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M6.80132 2.14853C7.70663 1.80917 8.70422 1.80919 9.60952 2.14859L14.1296 3.84317V7.11961C14.1296 11.6089 10.7615 13.5975 8.20543 14.5779C5.64931 13.5975 2.28052 11.6089 2.28052 7.11961V3.84317L6.80132 2.14853Z\" stroke=\"currentColor\" stroke-linejoin=\"round\"></path></svg>"},"check-circle-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M12.5303 6.53027L8.80273 10.2578C8.54967 10.5109 8.31796 10.7439 8.10645 10.9141C7.88375 11.0932 7.616 11.2602 7.27344 11.3145C7.09229 11.3431 6.90771 11.3431 6.72656 11.3145C6.384 11.2602 6.11625 11.0932 5.89355 10.9141C5.68204 10.7439 5.45033 10.5109 5.19727 10.2578L3.46973 8.53027L4.53027 7.46973L6.25781 9.19727C6.53457 9.47402 6.70036 9.63859 6.83398 9.74609C6.95637 9.84453 6.98241 9.83644 6.96094 9.83301C6.98679 9.83709 7.01321 9.83709 7.03906 9.83301C7.01759 9.83644 7.04363 9.84453 7.16602 9.74609C7.29964 9.63859 7.46543 9.47402 7.74219 9.19727L11.4697 5.46973L12.5303 6.53027Z\" fill=\"currentColor\"></path><path d=\"M14.5996 8C14.5996 4.35492 11.6451 1.40039 8 1.40039C4.35492 1.40039 1.40039 4.35492 1.40039 8C1.40039 11.6451 4.35492 14.5996 8 14.5996C11.6451 14.5996 14.5996 11.6451 14.5996 8ZM15.9004 8C15.9004 12.363 12.363 15.9004 8 15.9004C3.63695 15.9004 0.0996094 12.363 0.0996094 8C0.0996094 3.63695 3.63695 0.0996094 8 0.0996094C12.363 0.0996094 15.9004 3.63695 15.9004 8Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M12.5303 6.53027L8.80273 10.2578C8.54967 10.5109 8.31796 10.7439 8.10645 10.9141C7.88375 11.0932 7.616 11.2602 7.27344 11.3145C7.09229 11.3431 6.90771 11.3431 6.72656 11.3145C6.384 11.2602 6.11625 11.0932 5.89355 10.9141C5.68204 10.7439 5.45033 10.5109 5.19727 10.2578L3.46973 8.53027L4.53027 7.46973L6.25781 9.19727C6.53457 9.47402 6.70036 9.63859 6.83398 9.74609C6.95637 9.84453 6.98241 9.83644 6.96094 9.83301C6.98679 9.83709 7.01321 9.83709 7.03906 9.83301C7.01759 9.83644 7.04363 9.84453 7.16602 9.74609C7.29964 9.63859 7.46543 9.47402 7.74219 9.19727L11.4697 5.46973L12.5303 6.53027Z\" fill=\"currentColor\"></path><path d=\"M14.5996 8C14.5996 4.35492 11.6451 1.40039 8 1.40039C4.35492 1.40039 1.40039 4.35492 1.40039 8C1.40039 11.6451 4.35492 14.5996 8 14.5996C11.6451 14.5996 14.5996 11.6451 14.5996 8ZM15.9004 8C15.9004 12.363 12.363 15.9004 8 15.9004C3.63695 15.9004 0.0996094 12.363 0.0996094 8C0.0996094 3.63695 3.63695 0.0996094 8 0.0996094C12.363 0.0996094 15.9004 3.63695 15.9004 8Z\" fill=\"currentColor\"></path></svg>"},"unarchive-outline":{"size":20,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 20 20\" fill=\"none\" stroke-width=\"1\"><path fill-rule=\"evenodd\" clip-rule=\"evenodd\" d=\"M15.8659 2.05975C17.2603 2.05995 18.3913 3.19096 18.3914 4.58527V5.4874C18.3914 6.02747 18.2192 6.52672 17.9303 6.93735C17.9336 6.96524 17.9388 6.99318 17.9388 7.02195V12.8884C17.9388 13.6345 17.9395 14.2379 17.8996 14.7254C17.8642 15.1593 17.7936 15.5499 17.6373 15.9141L17.5654 16.0685C17.278 16.6328 16.8405 17.1046 16.3038 17.434L16.0679 17.5661C15.66 17.7739 15.2196 17.8598 14.7237 17.9003C14.2362 17.9401 13.6327 17.9405 12.8867 17.9405H7.11122C6.36511 17.9405 5.76171 17.9401 5.27418 17.9003C4.84051 17.8649 4.44949 17.7952 4.08545 17.6391L3.93104 17.5661C3.36673 17.2785 2.89392 16.8414 2.56465 16.3044L2.43245 16.0685C2.22473 15.6608 2.13878 15.2211 2.09825 14.7254C2.05841 14.2379 2.05912 13.6345 2.05912 12.8884V7.02195C2.05912 6.99284 2.06422 6.96449 2.06758 6.93629C1.77931 6.52592 1.60858 6.02687 1.60858 5.4874V4.58527C1.60876 3.19084 2.73962 2.05975 4.1341 2.05975H15.8659ZM16.4984 7.92936C16.296 7.98169 16.0847 8.01288 15.8659 8.01291H4.1341C3.91478 8.01291 3.70246 7.98194 3.49955 7.92936V12.8884C3.49955 13.6582 3.50053 14.1927 3.53445 14.608C3.56769 15.0146 3.62923 15.244 3.71635 15.415L3.7925 15.5514C3.98339 15.8627 4.25749 16.1165 4.58464 16.2833L4.72529 16.3435C4.88095 16.3993 5.08638 16.4402 5.39158 16.4651C5.80685 16.4991 6.34138 16.5001 7.11122 16.5001H12.8867C13.6564 16.5001 14.1911 16.499 14.6063 16.4651C15.0128 16.432 15.2423 16.3703 15.4133 16.2833L15.5508 16.2061C15.8618 16.0152 16.116 15.7419 16.2827 15.415L16.3429 15.2732C16.3985 15.1177 16.4396 14.9128 16.4645 14.608C16.4985 14.1927 16.4984 13.6583 16.4984 12.8884V7.92936ZM4.1341 3.50019C3.53511 3.50019 3.0492 3.98631 3.04902 4.58527V5.4874C3.04902 6.08649 3.535 6.57248 4.1341 6.57248H15.8659C16.4648 6.57228 16.951 6.08638 16.951 5.4874V4.58527C16.9509 3.98644 16.4647 3.50038 15.8659 3.50019H4.1341Z\" fill=\"currentColor\"></path><path d=\"M10 14.1V10.1M7.85 12.05L10 9.9L12.15 12.05\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 20 20\" fill=\"none\" stroke-width=\"1.3\"><path fill-rule=\"evenodd\" clip-rule=\"evenodd\" d=\"M15.8659 2.05975C17.2603 2.05995 18.3913 3.19096 18.3914 4.58527V5.4874C18.3914 6.02747 18.2192 6.52672 17.9303 6.93735C17.9336 6.96524 17.9388 6.99318 17.9388 7.02195V12.8884C17.9388 13.6345 17.9395 14.2379 17.8996 14.7254C17.8642 15.1593 17.7936 15.5499 17.6373 15.9141L17.5654 16.0685C17.278 16.6328 16.8405 17.1046 16.3038 17.434L16.0679 17.5661C15.66 17.7739 15.2196 17.8598 14.7237 17.9003C14.2362 17.9401 13.6327 17.9405 12.8867 17.9405H7.11122C6.36511 17.9405 5.76171 17.9401 5.27418 17.9003C4.84051 17.8649 4.44949 17.7952 4.08545 17.6391L3.93104 17.5661C3.36673 17.2785 2.89392 16.8414 2.56465 16.3044L2.43245 16.0685C2.22473 15.6608 2.13878 15.2211 2.09825 14.7254C2.05841 14.2379 2.05912 13.6345 2.05912 12.8884V7.02195C2.05912 6.99284 2.06422 6.96449 2.06758 6.93629C1.77931 6.52592 1.60858 6.02687 1.60858 5.4874V4.58527C1.60876 3.19084 2.73962 2.05975 4.1341 2.05975H15.8659ZM16.4984 7.92936C16.296 7.98169 16.0847 8.01288 15.8659 8.01291H4.1341C3.91478 8.01291 3.70246 7.98194 3.49955 7.92936V12.8884C3.49955 13.6582 3.50053 14.1927 3.53445 14.608C3.56769 15.0146 3.62923 15.244 3.71635 15.415L3.7925 15.5514C3.98339 15.8627 4.25749 16.1165 4.58464 16.2833L4.72529 16.3435C4.88095 16.3993 5.08638 16.4402 5.39158 16.4651C5.80685 16.4991 6.34138 16.5001 7.11122 16.5001H12.8867C13.6564 16.5001 14.1911 16.499 14.6063 16.4651C15.0128 16.432 15.2423 16.3703 15.4133 16.2833L15.5508 16.2061C15.8618 16.0152 16.116 15.7419 16.2827 15.415L16.3429 15.2732C16.3985 15.1177 16.4396 14.9128 16.4645 14.608C16.4985 14.1927 16.4984 13.6583 16.4984 12.8884V7.92936ZM4.1341 3.50019C3.53511 3.50019 3.0492 3.98631 3.04902 4.58527V5.4874C3.04902 6.08649 3.535 6.57248 4.1341 6.57248H15.8659C16.4648 6.57228 16.951 6.08638 16.951 5.4874V4.58527C16.9509 3.98644 16.4647 3.50038 15.8659 3.50019H4.1341Z\" fill=\"currentColor\"></path><path d=\"M10 14.1V10.1M7.85 12.05L10 9.9L12.15 12.05\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\"></path></svg>"},"pin-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M9.96976 1.70572L13.1554 3.93629L10.9019 8.12317L11.5158 11.605L10.7192 12.7427L2.52767 7.00693L3.3243 5.86922L6.80612 5.25528L9.96976 1.70572Z\" stroke=\"currentColor\" stroke-linejoin=\"round\"></path><path d=\"M6.05285 9.47511C6.27284 9.16094 6.70586 9.08458 7.02003 9.30457C7.3342 9.52455 7.41055 9.95757 7.19057 10.2717L3.98587 14.4708L3.21223 13.9291L6.05285 9.47511Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M9.96976 1.70572L13.1554 3.93629L10.9019 8.12317L11.5158 11.605L10.7192 12.7427L2.52767 7.00693L3.3243 5.86922L6.80612 5.25528L9.96976 1.70572Z\" stroke=\"currentColor\" stroke-linejoin=\"round\"></path><path d=\"M6.05285 9.47511C6.27284 9.16094 6.70586 9.08458 7.02003 9.30457C7.3342 9.52455 7.41055 9.95757 7.19057 10.2717L3.98587 14.4708L3.21223 13.9291L6.05285 9.47511Z\" fill=\"currentColor\"></path></svg>"},"pin-fill":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1\"><path d=\"M9.96976 1.70572L13.1554 3.93629L10.9019 8.12317L11.5158 11.605L10.7192 12.7427L2.52767 7.00693L3.3243 5.86922L6.80612 5.25528L9.96976 1.70572Z\" fill=\"currentColor\" stroke=\"currentColor\" stroke-linejoin=\"round\"></path><path d=\"M6.05285 9.47511C6.27284 9.16094 6.70586 9.08458 7.02003 9.30457C7.3342 9.52455 7.41055 9.95757 7.19057 10.2717L3.98587 14.4708L3.21223 13.9291L6.05285 9.47511Z\" fill=\"currentColor\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke-width=\"1.3\"><path d=\"M9.96976 1.70572L13.1554 3.93629L10.9019 8.12317L11.5158 11.605L10.7192 12.7427L2.52767 7.00693L3.3243 5.86922L6.80612 5.25528L9.96976 1.70572Z\" fill=\"currentColor\" stroke=\"currentColor\" stroke-linejoin=\"round\"></path><path d=\"M6.05285 9.47511C6.27284 9.16094 6.70586 9.08458 7.02003 9.30457C7.3342 9.52455 7.41055 9.95757 7.19057 10.2717L3.98587 14.4708L3.21223 13.9291L6.05285 9.47511Z\" fill=\"currentColor\"></path></svg>"},"flat-list-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1\"><path d=\"M6 3.5h7.5M6 8h7.5M6 12.5h7.5\"></path><path d=\"M2.6 3.5h.01M2.6 8h.01M2.6 12.5h.01\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1.3\"><path d=\"M6 3.5h7.5M6 8h7.5M6 12.5h7.5\"></path><path d=\"M2.6 3.5h.01M2.6 8h.01M2.6 12.5h.01\"></path></svg>"},"workspace-tree-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1\"><path d=\"M14 12.05c0 .8-.65 1.45-1.46 1.45H3.46C2.65 13.5 2 12.85 2 12.05v-8.1c0-.8.65-1.45 1.46-1.45h2.4c.49 0 .94.24 1.21.65l.5.73c.27.4.73.65 1.21.65h3.76c.8 0 1.46.65 1.46 1.45v6.02Z\"></path><path d=\"M8.7 8.1v3M11.2 8.1v3\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.3\"><path d=\"M14 12.05c0 .8-.65 1.45-1.46 1.45H3.46C2.65 13.5 2 12.85 2 12.05v-8.1c0-.8.65-1.45 1.46-1.45h2.4c.49 0 .94.24 1.21.65l.5.73c.27.4.73.65 1.21.65h3.76c.8 0 1.46.65 1.46 1.45v6.02Z\"></path><path d=\"M8.7 8.1v3M11.2 8.1v3\"></path></svg>"},"chevrons-up-down-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1\"><path d=\"m5.1 6 2.9-2.9L10.9 6\"></path><path d=\"m5.1 10 2.9 2.9 2.9-2.9\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.3\"><path d=\"m5.1 6 2.9-2.9L10.9 6\"></path><path d=\"m5.1 10 2.9 2.9 2.9-2.9\"></path></svg>"},"drag-handle-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1\"><path d=\"M4 5h8M4 8h8M4 11h8\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1.3\"><path d=\"M4 5h8M4 8h8M4 11h8\"></path></svg>"},"archive-off-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1\"><mask id=\"dsw-icon-archive-off-outline-regular-mask\" maskUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"16\" height=\"16\"><rect x=\"0\" y=\"0\" width=\"16\" height=\"16\" fill=\"white\" stroke=\"none\"></rect><path d=\"m2.2 1.3 11.6 12.8\" stroke=\"black\" stroke-width=\"4\"></path></mask><g mask=\"url(#dsw-icon-archive-off-outline-regular-mask)\"><rect x=\"1.9\" y=\"2.1\" width=\"12.2\" height=\"3.4\" rx=\"1.1\"></rect><path d=\"M2.95 5.7v4.8a2.9 2.9 0 0 0 2.9 2.9h4.3a2.9 2.9 0 0 0 2.9-2.9V5.7\"></path></g><path d=\"m2.2 1.3 11.6 12.8\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.3\"><mask id=\"dsw-icon-archive-off-outline-medium-mask\" maskUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"16\" height=\"16\"><rect x=\"0\" y=\"0\" width=\"16\" height=\"16\" fill=\"white\" stroke=\"none\"></rect><path d=\"m2.2 1.3 11.6 12.8\" stroke=\"black\" stroke-width=\"4.3\"></path></mask><g mask=\"url(#dsw-icon-archive-off-outline-medium-mask)\"><rect x=\"1.9\" y=\"2.1\" width=\"12.2\" height=\"3.4\" rx=\"1.1\"></rect><path d=\"M2.95 5.7v4.8a2.9 2.9 0 0 0 2.9 2.9h4.3a2.9 2.9 0 0 0 2.9-2.9V5.7\"></path></g><path d=\"m2.2 1.3 11.6 12.8\"></path></svg>"},"archive-check-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1\"><rect x=\"1.9\" y=\"2.1\" width=\"12.2\" height=\"3.4\" rx=\"1.1\"></rect><path d=\"M2.95 5.7v4.8a2.9 2.9 0 0 0 2.9 2.9h4.3a2.9 2.9 0 0 0 2.9-2.9V5.7\"></path><path d=\"m6 9.35 1.4 1.4 2.6-2.6\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"1.3\"><rect x=\"1.9\" y=\"2.1\" width=\"12.2\" height=\"3.4\" rx=\"1.1\"></rect><path d=\"M2.95 5.7v4.8a2.9 2.9 0 0 0 2.9 2.9h4.3a2.9 2.9 0 0 0 2.9-2.9V5.7\"></path><path d=\"m6 9.35 1.4 1.4 2.6-2.6\"></path></svg>"},"sliders-two-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1\"><path d=\"M2.3 5h5.85M12.05 5h1.65\"></path><circle cx=\"9.95\" cy=\"5\" r=\"1.45\"></circle><path d=\"M2.3 11h1.65M7.85 11h5.85\"></path><circle cx=\"5.75\" cy=\"11\" r=\"1.45\"></circle></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-width=\"1.3\"><path d=\"M2.3 5h5.85M12.05 5h1.65\"></path><circle cx=\"9.95\" cy=\"5\" r=\"1.45\"></circle><path d=\"M2.3 11h1.65M7.85 11h5.85\"></path><circle cx=\"5.75\" cy=\"11\" r=\"1.45\"></circle></svg>"},"microphone-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1\"><rect x=\"5\" y=\"1.5\" width=\"6\" height=\"9\" rx=\"3\"></rect><path d=\"M2.35 8.675C3.075 11.3 5.2 13.125 8 13.125C10.8 13.125 12.925 11.3 13.65 8.675M8 13.125V15\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.3\"><rect x=\"5.15\" y=\"1.65\" width=\"5.7\" height=\"8.7\" rx=\"2.85\"></rect><path d=\"M2.35 8.675C3.075 11.3 5.2 13.125 8 13.125C10.8 13.125 12.925 11.3 13.65 8.675M8 13.125V15\"></path></svg>"},"save-outline":{"size":16,"aspect":null,"regular":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1\" stroke-linejoin=\"round\"><path d=\"M2.5 1.5h8l3 3v9a1 1 0 0 1-1 1h-10a1 1 0 0 1-1-1v-11a1 1 0 0 1 1-1Z\"></path><path d=\"M4.5 1.5v4h6v-4M4 14.5v-6h7v6\"></path></svg>","medium":"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.3\" stroke-linejoin=\"round\"><path d=\"M2.5 1.5h8l3 3v9a1 1 0 0 1-1 1h-10a1 1 0 0 1-1-1v-11a1 1 0 0 1 1-1Z\"></path><path d=\"M4.5 1.5v4h6v-4M4 14.5v-6h7v6\"></path></svg>"}};
  function makeProductIcon(glyph, weight) {
    return function ProductIcon(props) {
      var art = ICON_ART[glyph];
      var size = (props && props.size) || (art && art.size) || 16;
      var aspect = (art && art.aspect) || [1, 1];
      var width = (size * aspect[0]) / aspect[1];
      var svg = art ? (art[weight] || art.regular) : "";
      svg = svg.replace("<svg ", "<svg width=\"" + width + "\" height=\"" + size + "\" ");
      return React.createElement("span", {
        className: classNames("dshr-icon", props && props.className),
        "aria-hidden": "true",
        style: { display: "inline-flex", width: width, height: size },
        dangerouslySetInnerHTML: { __html: svg },
      });
    };
  }
  var IconLayersOutlineRegular = makeProductIcon("layers-outline", "regular");
  var IconLayersOutlineMedium = makeProductIcon("layers-outline", "medium");
  var IconNewChatOutlineRegular = makeProductIcon("new-chat-outline", "regular");
  var IconNewChatOutlineMedium = makeProductIcon("new-chat-outline", "medium");
  var IconSearchOutlineRegular = makeProductIcon("search-outline", "regular");
  var IconSearchOutlineMedium = makeProductIcon("search-outline", "medium");
  var IconEyeOutlineRegular = makeProductIcon("eye-outline", "regular");
  var IconEyeOutlineMedium = makeProductIcon("eye-outline", "medium");
  var IconEyeOffOutlineRegular = makeProductIcon("eye-off-outline", "regular");
  var IconEyeOffOutlineMedium = makeProductIcon("eye-off-outline", "medium");
  var IconGlobeOutlineRegular = makeProductIcon("globe-outline", "regular");
  var IconGlobeOutlineMedium = makeProductIcon("globe-outline", "medium");
  var IconSettingsOutlineRegular = makeProductIcon("settings-outline", "regular");
  var IconSettingsOutlineMedium = makeProductIcon("settings-outline", "medium");
  var IconPanelLeftOutlineRegular = makeProductIcon("panel-left-outline", "regular");
  var IconPanelLeftOutlineMedium = makeProductIcon("panel-left-outline", "medium");
  var IconEllipsisOutlineRegular = makeProductIcon("ellipsis-outline", "regular");
  var IconEllipsisOutlineMedium = makeProductIcon("ellipsis-outline", "medium");
  var IconPlusOutlineRegular = makeProductIcon("plus-outline", "regular");
  var IconPlusOutlineMedium = makeProductIcon("plus-outline", "medium");
  var IconCheckOutlineRegular = makeProductIcon("check-outline", "regular");
  var IconCheckOutlineMedium = makeProductIcon("check-outline", "medium");
  var IconMailUnreadOutlineRegular = makeProductIcon("mail-unread-outline", "regular");
  var IconMailUnreadOutlineMedium = makeProductIcon("mail-unread-outline", "medium");
  var IconMailReadOutlineRegular = makeProductIcon("mail-read-outline", "regular");
  var IconMailReadOutlineMedium = makeProductIcon("mail-read-outline", "medium");
  var IconBranchOutlineRegular = makeProductIcon("branch-outline", "regular");
  var IconBranchOutlineMedium = makeProductIcon("branch-outline", "medium");
  var IconChevronDownOutlineRegular = makeProductIcon("chevron-down-outline", "regular");
  var IconChevronDownOutlineMedium = makeProductIcon("chevron-down-outline", "medium");
  var IconChevronLeftOutlineRegular = makeProductIcon("chevron-left-outline", "regular");
  var IconChevronLeftOutlineMedium = makeProductIcon("chevron-left-outline", "medium");
  var IconChevronRightOutlineRegular = makeProductIcon("chevron-right-outline", "regular");
  var IconChevronRightOutlineMedium = makeProductIcon("chevron-right-outline", "medium");
  var IconTriangleRightFillRegular = makeProductIcon("triangle-right-fill", "regular");
  var IconTriangleRightFillMedium = makeProductIcon("triangle-right-fill", "medium");
  var IconChevronUpOutlineRegular = makeProductIcon("chevron-up-outline", "regular");
  var IconChevronUpOutlineMedium = makeProductIcon("chevron-up-outline", "medium");
  var IconCloseOutlineRegular = makeProductIcon("close-outline", "regular");
  var IconCloseOutlineMedium = makeProductIcon("close-outline", "medium");
  var IconCloseFillRegular = makeProductIcon("close-fill", "regular");
  var IconCloseFillMedium = makeProductIcon("close-fill", "medium");
  var IconCloseCircleFillRegular = makeProductIcon("close-circle-fill", "regular");
  var IconCloseCircleFillMedium = makeProductIcon("close-circle-fill", "medium");
  var IconCopyOutlineRegular = makeProductIcon("copy-outline", "regular");
  var IconCopyOutlineMedium = makeProductIcon("copy-outline", "medium");
  var IconRefreshOutlineRegular = makeProductIcon("refresh-outline", "regular");
  var IconRefreshOutlineMedium = makeProductIcon("refresh-outline", "medium");
  var IconLikeOutlineRegular = makeProductIcon("like-outline", "regular");
  var IconLikeOutlineMedium = makeProductIcon("like-outline", "medium");
  var IconLikeFillRegular = makeProductIcon("like-fill", "regular");
  var IconLikeFillMedium = makeProductIcon("like-fill", "medium");
  var IconDislikeOutlineRegular = makeProductIcon("dislike-outline", "regular");
  var IconDislikeOutlineMedium = makeProductIcon("dislike-outline", "medium");
  var IconDislikeFillRegular = makeProductIcon("dislike-fill", "regular");
  var IconDislikeFillMedium = makeProductIcon("dislike-fill", "medium");
  var IconShareOutlineRegular = makeProductIcon("share-outline", "regular");
  var IconShareOutlineMedium = makeProductIcon("share-outline", "medium");
  var IconDeliverDocRegular = makeProductIcon("deliver-doc", "regular");
  var IconDeliverDocMedium = makeProductIcon("deliver-doc", "medium");
  var IconEditOutlineRegular = makeProductIcon("edit-outline", "regular");
  var IconEditOutlineMedium = makeProductIcon("edit-outline", "medium");
  var IconThinkOutlineRegular = makeProductIcon("think-outline", "regular");
  var IconThinkOutlineMedium = makeProductIcon("think-outline", "medium");
  var IconAgentPresetOutlineRegular = makeProductIcon("agent-preset-outline", "regular");
  var IconAgentPresetOutlineMedium = makeProductIcon("agent-preset-outline", "medium");
  var IconBrowseOutlineRegular = makeProductIcon("browse-outline", "regular");
  var IconBrowseOutlineMedium = makeProductIcon("browse-outline", "medium");
  var IconContextInjectionOutlineRegular = makeProductIcon("context-injection-outline", "regular");
  var IconContextInjectionOutlineMedium = makeProductIcon("context-injection-outline", "medium");
  var IconLinkOutlineRegular = makeProductIcon("link-outline", "regular");
  var IconLinkOutlineMedium = makeProductIcon("link-outline", "medium");
  var IconRightUpOutlineRegular = makeProductIcon("right-up-outline", "regular");
  var IconRightUpOutlineMedium = makeProductIcon("right-up-outline", "medium");
  var IconEnhanceOutlineRegular = makeProductIcon("enhance-outline", "regular");
  var IconEnhanceOutlineMedium = makeProductIcon("enhance-outline", "medium");
  var IconTrashOutlineRegular = makeProductIcon("trash-outline", "regular");
  var IconTrashOutlineMedium = makeProductIcon("trash-outline", "medium");
  var IconWarningOutlineRegular = makeProductIcon("warning-outline", "regular");
  var IconWarningOutlineMedium = makeProductIcon("warning-outline", "medium");
  var IconCheckCircleFillRegular = makeProductIcon("check-circle-fill", "regular");
  var IconCheckCircleFillMedium = makeProductIcon("check-circle-fill", "medium");
  var IconWarningTriangleOutlineRegular = makeProductIcon("warning-triangle-outline", "regular");
  var IconWarningTriangleOutlineMedium = makeProductIcon("warning-triangle-outline", "medium");
  var IconUserOutlineRegular = makeProductIcon("user-outline", "regular");
  var IconUserOutlineMedium = makeProductIcon("user-outline", "medium");
  var IconUsersOutlineRegular = makeProductIcon("users-outline", "regular");
  var IconUsersOutlineMedium = makeProductIcon("users-outline", "medium");
  var IconPaperPlaneOutlineRegular = makeProductIcon("paper-plane-outline", "regular");
  var IconPaperPlaneOutlineMedium = makeProductIcon("paper-plane-outline", "medium");
  var IconStopFillRegular = makeProductIcon("stop-fill", "regular");
  var IconStopFillMedium = makeProductIcon("stop-fill", "medium");
  var IconPaperclipOutlineRegular = makeProductIcon("paperclip-outline", "regular");
  var IconPaperclipOutlineMedium = makeProductIcon("paperclip-outline", "medium");
  var IconLoadingOutlineRegular = makeProductIcon("loading-outline", "regular");
  var IconLoadingOutlineMedium = makeProductIcon("loading-outline", "medium");
  var IconDownloadOutlineRegular = makeProductIcon("download-outline", "regular");
  var IconDownloadOutlineMedium = makeProductIcon("download-outline", "medium");
  var IconUploadOutlineRegular = makeProductIcon("upload-outline", "regular");
  var IconUploadOutlineMedium = makeProductIcon("upload-outline", "medium");
  var IconPlayOutlineRegular = makeProductIcon("play-outline", "regular");
  var IconPlayOutlineMedium = makeProductIcon("play-outline", "medium");
  var IconPauseOutlineRegular = makeProductIcon("pause-outline", "regular");
  var IconPauseOutlineMedium = makeProductIcon("pause-outline", "medium");
  var IconFullscreenOutlineRegular = makeProductIcon("fullscreen-outline", "regular");
  var IconFullscreenOutlineMedium = makeProductIcon("fullscreen-outline", "medium");
  var IconCodeOutlineRegular = makeProductIcon("code-outline", "regular");
  var IconCodeOutlineMedium = makeProductIcon("code-outline", "medium");
  var IconCordisPluginOutlineRegular = makeProductIcon("cordis-plugin-outline", "regular");
  var IconCordisPluginOutlineMedium = makeProductIcon("cordis-plugin-outline", "medium");
  var IconApiOutlineRegular = makeProductIcon("api-outline", "regular");
  var IconApiOutlineMedium = makeProductIcon("api-outline", "medium");
  var IconPersonalizationOutlineRegular = makeProductIcon("personalization-outline", "regular");
  var IconPersonalizationOutlineMedium = makeProductIcon("personalization-outline", "medium");
  var IconProjectAddOutlineRegular = makeProductIcon("project-add-outline", "regular");
  var IconProjectAddOutlineMedium = makeProductIcon("project-add-outline", "medium");
  var IconFolderOpenOutlineRegular = makeProductIcon("folder-open-outline", "regular");
  var IconFolderOpenOutlineMedium = makeProductIcon("folder-open-outline", "medium");
  var IconFolderOpenRegular = makeProductIcon("folder-open", "regular");
  var IconFolderOpenMedium = makeProductIcon("folder-open", "medium");
  var IconFolderCloseRegular = makeProductIcon("folder-close", "regular");
  var IconFolderCloseMedium = makeProductIcon("folder-close", "medium");
  var IconTreeCornerRegular = makeProductIcon("tree-corner", "regular");
  var IconTreeCornerMedium = makeProductIcon("tree-corner", "medium");
  var IconLightOutlineRegular = makeProductIcon("light-outline", "regular");
  var IconLightOutlineMedium = makeProductIcon("light-outline", "medium");
  var IconDarkOutlineRegular = makeProductIcon("dark-outline", "regular");
  var IconDarkOutlineMedium = makeProductIcon("dark-outline", "medium");
  var IconFollowsystemOutlineRegular = makeProductIcon("followsystem-outline", "regular");
  var IconFollowsystemOutlineMedium = makeProductIcon("followsystem-outline", "medium");
  var IconDataOutlineRegular = makeProductIcon("data-outline", "regular");
  var IconDataOutlineMedium = makeProductIcon("data-outline", "medium");
  var IconDatabaseOutlineRegular = makeProductIcon("database-outline", "regular");
  var IconDatabaseOutlineMedium = makeProductIcon("database-outline", "medium");
  var IconClockOutlineRegular = makeProductIcon("clock-outline", "regular");
  var IconClockOutlineMedium = makeProductIcon("clock-outline", "medium");
  var IconGaugeOutlineRegular = makeProductIcon("gauge-outline", "regular");
  var IconGaugeOutlineMedium = makeProductIcon("gauge-outline", "medium");
  var IconSendOutlineRegular = makeProductIcon("send-outline", "regular");
  var IconSendOutlineMedium = makeProductIcon("send-outline", "medium");
  var IconQueueOutlineRegular = makeProductIcon("queue-outline", "regular");
  var IconQueueOutlineMedium = makeProductIcon("queue-outline", "medium");
  var IconChecklistOutlineRegular = makeProductIcon("checklist-outline", "regular");
  var IconChecklistOutlineMedium = makeProductIcon("checklist-outline", "medium");
  var IconListPenOutlineRegular = makeProductIcon("list-pen-outline", "regular");
  var IconListPenOutlineMedium = makeProductIcon("list-pen-outline", "medium");
  var IconGoalOutlineRegular = makeProductIcon("goal-outline", "regular");
  var IconGoalOutlineMedium = makeProductIcon("goal-outline", "medium");
  var IconSparkleRegular = makeProductIcon("sparkle", "regular");
  var IconSparkleMedium = makeProductIcon("sparkle", "medium");
  var IconInspectOutlineRegular = makeProductIcon("inspect-outline", "regular");
  var IconInspectOutlineMedium = makeProductIcon("inspect-outline", "medium");
  var IconSkillOutlineRegular = makeProductIcon("skill-outline", "regular");
  var IconSkillOutlineMedium = makeProductIcon("skill-outline", "medium");
  var IconQuestionOutlineRegular = makeProductIcon("question-outline", "regular");
  var IconQuestionOutlineMedium = makeProductIcon("question-outline", "medium");
  var IconInfoOutlineRegular = makeProductIcon("info-outline", "regular");
  var IconInfoOutlineMedium = makeProductIcon("info-outline", "medium");
  var IconPluginPinwheelOutlineRegular = makeProductIcon("plugin-pinwheel-outline", "regular");
  var IconPluginPinwheelOutlineMedium = makeProductIcon("plugin-pinwheel-outline", "medium");
  var IconAlarmClockOutlineRegular = makeProductIcon("alarm-clock-outline", "regular");
  var IconAlarmClockOutlineMedium = makeProductIcon("alarm-clock-outline", "medium");
  var IconArchiveOutlineRegular = makeProductIcon("archive-outline", "regular");
  var IconArchiveOutlineMedium = makeProductIcon("archive-outline", "medium");
  var IconWrapLinesOutlineRegular = makeProductIcon("wrap-lines-outline", "regular");
  var IconWrapLinesOutlineMedium = makeProductIcon("wrap-lines-outline", "medium");
  var IconNowrapFillRegular = makeProductIcon("nowrap-fill", "regular");
  var IconNowrapFillMedium = makeProductIcon("nowrap-fill", "medium");
  var IconWrapFillRegular = makeProductIcon("wrap-fill", "regular");
  var IconWrapFillMedium = makeProductIcon("wrap-fill", "medium");
  var IconCompareSplitOutlineRegular = makeProductIcon("compare-split-outline", "regular");
  var IconCompareSplitOutlineMedium = makeProductIcon("compare-split-outline", "medium");
  var IconPlanOutlineRegular = makeProductIcon("plan-outline", "regular");
  var IconPlanOutlineMedium = makeProductIcon("plan-outline", "medium");
  var IconCompactOutlineRegular = makeProductIcon("compact-outline", "regular");
  var IconCompactOutlineMedium = makeProductIcon("compact-outline", "medium");
  var IconShieldOutlineRegular = makeProductIcon("shield-outline", "regular");
  var IconShieldOutlineMedium = makeProductIcon("shield-outline", "medium");
  var IconCheckCircleOutlineRegular = makeProductIcon("check-circle-outline", "regular");
  var IconCheckCircleOutlineMedium = makeProductIcon("check-circle-outline", "medium");
  var IconUnarchiveOutlineRegular = makeProductIcon("unarchive-outline", "regular");
  var IconUnarchiveOutlineMedium = makeProductIcon("unarchive-outline", "medium");
  var IconPinOutlineRegular = makeProductIcon("pin-outline", "regular");
  var IconPinOutlineMedium = makeProductIcon("pin-outline", "medium");
  var IconPinFillRegular = makeProductIcon("pin-fill", "regular");
  var IconPinFillMedium = makeProductIcon("pin-fill", "medium");
  var IconFlatListOutlineRegular = makeProductIcon("flat-list-outline", "regular");
  var IconFlatListOutlineMedium = makeProductIcon("flat-list-outline", "medium");
  var IconWorkspaceTreeOutlineRegular = makeProductIcon("workspace-tree-outline", "regular");
  var IconWorkspaceTreeOutlineMedium = makeProductIcon("workspace-tree-outline", "medium");
  var IconChevronsUpDownOutlineRegular = makeProductIcon("chevrons-up-down-outline", "regular");
  var IconChevronsUpDownOutlineMedium = makeProductIcon("chevrons-up-down-outline", "medium");
  var IconDragHandleOutlineRegular = makeProductIcon("drag-handle-outline", "regular");
  var IconDragHandleOutlineMedium = makeProductIcon("drag-handle-outline", "medium");
  var IconArchiveOffOutlineRegular = makeProductIcon("archive-off-outline", "regular");
  var IconArchiveOffOutlineMedium = makeProductIcon("archive-off-outline", "medium");
  var IconArchiveCheckOutlineRegular = makeProductIcon("archive-check-outline", "regular");
  var IconArchiveCheckOutlineMedium = makeProductIcon("archive-check-outline", "medium");
  var IconSlidersTwoOutlineRegular = makeProductIcon("sliders-two-outline", "regular");
  var IconSlidersTwoOutlineMedium = makeProductIcon("sliders-two-outline", "medium");
  var IconMicrophoneOutlineRegular = makeProductIcon("microphone-outline", "regular");
  var IconMicrophoneOutlineMedium = makeProductIcon("microphone-outline", "medium");
  var IconSaveOutlineRegular = makeProductIcon("save-outline", "regular");
  var IconSaveOutlineMedium = makeProductIcon("save-outline", "medium");


  var surface = {
    Button: Button,
    Input: Input,
    Switch: Switch,
    Checkbox: Checkbox,
    Tag: Tag,
    Pill: Pill,
    SegmentedControl: SegmentedControl,
    SegmentedTabs: SegmentedTabs,
    DisclosureRow: DisclosureRow,
    TextShimmer: TextShimmer,
    StateDot: StateDot,
    PathLabel: PathLabel,
    Toast: Toast,
    ShortcutKeys: ShortcutKeys,
    Modal: Modal,
    Menu: Menu,
    MenuGroup: MenuGroup,
    observeStickyMenuGroups: observeStickyMenuGroups,
    MenuItemButton: MenuItemButton,
    MenuSurface: MenuSurface,
    MENU_TRANSITION_MS: MENU_TRANSITION_MS,
    usePointerGrace: usePointerGrace,
    focusWithoutRing: focusWithoutRing,
    isMobileDevice: isMobileDevice,
    Tooltip: Tooltip,
    Spinner: Spinner,
    JsonBlock: JsonBlock,
    MarkdownText: MarkdownText,
    writeClipboard: writeClipboard,
    useAnchoredPosition: useAnchoredPosition,
    useDismissOnOutsidePointer: useDismissOnOutsidePointer,
    IconLayersOutlineRegular: IconLayersOutlineRegular,
    IconLayersOutlineMedium: IconLayersOutlineMedium,
    IconNewChatOutlineRegular: IconNewChatOutlineRegular,
    IconNewChatOutlineMedium: IconNewChatOutlineMedium,
    IconSearchOutlineRegular: IconSearchOutlineRegular,
    IconSearchOutlineMedium: IconSearchOutlineMedium,
    IconEyeOutlineRegular: IconEyeOutlineRegular,
    IconEyeOutlineMedium: IconEyeOutlineMedium,
    IconEyeOffOutlineRegular: IconEyeOffOutlineRegular,
    IconEyeOffOutlineMedium: IconEyeOffOutlineMedium,
    IconGlobeOutlineRegular: IconGlobeOutlineRegular,
    IconGlobeOutlineMedium: IconGlobeOutlineMedium,
    IconSettingsOutlineRegular: IconSettingsOutlineRegular,
    IconSettingsOutlineMedium: IconSettingsOutlineMedium,
    IconPanelLeftOutlineRegular: IconPanelLeftOutlineRegular,
    IconPanelLeftOutlineMedium: IconPanelLeftOutlineMedium,
    IconEllipsisOutlineRegular: IconEllipsisOutlineRegular,
    IconEllipsisOutlineMedium: IconEllipsisOutlineMedium,
    IconPlusOutlineRegular: IconPlusOutlineRegular,
    IconPlusOutlineMedium: IconPlusOutlineMedium,
    IconCheckOutlineRegular: IconCheckOutlineRegular,
    IconCheckOutlineMedium: IconCheckOutlineMedium,
    IconMailUnreadOutlineRegular: IconMailUnreadOutlineRegular,
    IconMailUnreadOutlineMedium: IconMailUnreadOutlineMedium,
    IconMailReadOutlineRegular: IconMailReadOutlineRegular,
    IconMailReadOutlineMedium: IconMailReadOutlineMedium,
    IconBranchOutlineRegular: IconBranchOutlineRegular,
    IconBranchOutlineMedium: IconBranchOutlineMedium,
    IconChevronDownOutlineRegular: IconChevronDownOutlineRegular,
    IconChevronDownOutlineMedium: IconChevronDownOutlineMedium,
    IconChevronLeftOutlineRegular: IconChevronLeftOutlineRegular,
    IconChevronLeftOutlineMedium: IconChevronLeftOutlineMedium,
    IconChevronRightOutlineRegular: IconChevronRightOutlineRegular,
    IconChevronRightOutlineMedium: IconChevronRightOutlineMedium,
    IconTriangleRightFillRegular: IconTriangleRightFillRegular,
    IconTriangleRightFillMedium: IconTriangleRightFillMedium,
    IconChevronUpOutlineRegular: IconChevronUpOutlineRegular,
    IconChevronUpOutlineMedium: IconChevronUpOutlineMedium,
    IconCloseOutlineRegular: IconCloseOutlineRegular,
    IconCloseOutlineMedium: IconCloseOutlineMedium,
    IconCloseFillRegular: IconCloseFillRegular,
    IconCloseFillMedium: IconCloseFillMedium,
    IconCloseCircleFillRegular: IconCloseCircleFillRegular,
    IconCloseCircleFillMedium: IconCloseCircleFillMedium,
    IconCopyOutlineRegular: IconCopyOutlineRegular,
    IconCopyOutlineMedium: IconCopyOutlineMedium,
    IconRefreshOutlineRegular: IconRefreshOutlineRegular,
    IconRefreshOutlineMedium: IconRefreshOutlineMedium,
    IconLikeOutlineRegular: IconLikeOutlineRegular,
    IconLikeOutlineMedium: IconLikeOutlineMedium,
    IconLikeFillRegular: IconLikeFillRegular,
    IconLikeFillMedium: IconLikeFillMedium,
    IconDislikeOutlineRegular: IconDislikeOutlineRegular,
    IconDislikeOutlineMedium: IconDislikeOutlineMedium,
    IconDislikeFillRegular: IconDislikeFillRegular,
    IconDislikeFillMedium: IconDislikeFillMedium,
    IconShareOutlineRegular: IconShareOutlineRegular,
    IconShareOutlineMedium: IconShareOutlineMedium,
    IconDeliverDocRegular: IconDeliverDocRegular,
    IconDeliverDocMedium: IconDeliverDocMedium,
    IconEditOutlineRegular: IconEditOutlineRegular,
    IconEditOutlineMedium: IconEditOutlineMedium,
    IconThinkOutlineRegular: IconThinkOutlineRegular,
    IconThinkOutlineMedium: IconThinkOutlineMedium,
    IconAgentPresetOutlineRegular: IconAgentPresetOutlineRegular,
    IconAgentPresetOutlineMedium: IconAgentPresetOutlineMedium,
    IconBrowseOutlineRegular: IconBrowseOutlineRegular,
    IconBrowseOutlineMedium: IconBrowseOutlineMedium,
    IconContextInjectionOutlineRegular: IconContextInjectionOutlineRegular,
    IconContextInjectionOutlineMedium: IconContextInjectionOutlineMedium,
    IconLinkOutlineRegular: IconLinkOutlineRegular,
    IconLinkOutlineMedium: IconLinkOutlineMedium,
    IconRightUpOutlineRegular: IconRightUpOutlineRegular,
    IconRightUpOutlineMedium: IconRightUpOutlineMedium,
    IconEnhanceOutlineRegular: IconEnhanceOutlineRegular,
    IconEnhanceOutlineMedium: IconEnhanceOutlineMedium,
    IconTrashOutlineRegular: IconTrashOutlineRegular,
    IconTrashOutlineMedium: IconTrashOutlineMedium,
    IconWarningOutlineRegular: IconWarningOutlineRegular,
    IconWarningOutlineMedium: IconWarningOutlineMedium,
    IconCheckCircleFillRegular: IconCheckCircleFillRegular,
    IconCheckCircleFillMedium: IconCheckCircleFillMedium,
    IconWarningTriangleOutlineRegular: IconWarningTriangleOutlineRegular,
    IconWarningTriangleOutlineMedium: IconWarningTriangleOutlineMedium,
    IconUserOutlineRegular: IconUserOutlineRegular,
    IconUserOutlineMedium: IconUserOutlineMedium,
    IconUsersOutlineRegular: IconUsersOutlineRegular,
    IconUsersOutlineMedium: IconUsersOutlineMedium,
    IconPaperPlaneOutlineRegular: IconPaperPlaneOutlineRegular,
    IconPaperPlaneOutlineMedium: IconPaperPlaneOutlineMedium,
    IconStopFillRegular: IconStopFillRegular,
    IconStopFillMedium: IconStopFillMedium,
    IconPaperclipOutlineRegular: IconPaperclipOutlineRegular,
    IconPaperclipOutlineMedium: IconPaperclipOutlineMedium,
    IconLoadingOutlineRegular: IconLoadingOutlineRegular,
    IconLoadingOutlineMedium: IconLoadingOutlineMedium,
    IconDownloadOutlineRegular: IconDownloadOutlineRegular,
    IconDownloadOutlineMedium: IconDownloadOutlineMedium,
    IconUploadOutlineRegular: IconUploadOutlineRegular,
    IconUploadOutlineMedium: IconUploadOutlineMedium,
    IconPlayOutlineRegular: IconPlayOutlineRegular,
    IconPlayOutlineMedium: IconPlayOutlineMedium,
    IconPauseOutlineRegular: IconPauseOutlineRegular,
    IconPauseOutlineMedium: IconPauseOutlineMedium,
    IconFullscreenOutlineRegular: IconFullscreenOutlineRegular,
    IconFullscreenOutlineMedium: IconFullscreenOutlineMedium,
    IconCodeOutlineRegular: IconCodeOutlineRegular,
    IconCodeOutlineMedium: IconCodeOutlineMedium,
    IconCordisPluginOutlineRegular: IconCordisPluginOutlineRegular,
    IconCordisPluginOutlineMedium: IconCordisPluginOutlineMedium,
    IconApiOutlineRegular: IconApiOutlineRegular,
    IconApiOutlineMedium: IconApiOutlineMedium,
    IconPersonalizationOutlineRegular: IconPersonalizationOutlineRegular,
    IconPersonalizationOutlineMedium: IconPersonalizationOutlineMedium,
    IconProjectAddOutlineRegular: IconProjectAddOutlineRegular,
    IconProjectAddOutlineMedium: IconProjectAddOutlineMedium,
    IconFolderOpenOutlineRegular: IconFolderOpenOutlineRegular,
    IconFolderOpenOutlineMedium: IconFolderOpenOutlineMedium,
    IconFolderOpenRegular: IconFolderOpenRegular,
    IconFolderOpenMedium: IconFolderOpenMedium,
    IconFolderCloseRegular: IconFolderCloseRegular,
    IconFolderCloseMedium: IconFolderCloseMedium,
    IconTreeCornerRegular: IconTreeCornerRegular,
    IconTreeCornerMedium: IconTreeCornerMedium,
    IconLightOutlineRegular: IconLightOutlineRegular,
    IconLightOutlineMedium: IconLightOutlineMedium,
    IconDarkOutlineRegular: IconDarkOutlineRegular,
    IconDarkOutlineMedium: IconDarkOutlineMedium,
    IconFollowsystemOutlineRegular: IconFollowsystemOutlineRegular,
    IconFollowsystemOutlineMedium: IconFollowsystemOutlineMedium,
    IconDataOutlineRegular: IconDataOutlineRegular,
    IconDataOutlineMedium: IconDataOutlineMedium,
    IconDatabaseOutlineRegular: IconDatabaseOutlineRegular,
    IconDatabaseOutlineMedium: IconDatabaseOutlineMedium,
    IconClockOutlineRegular: IconClockOutlineRegular,
    IconClockOutlineMedium: IconClockOutlineMedium,
    IconGaugeOutlineRegular: IconGaugeOutlineRegular,
    IconGaugeOutlineMedium: IconGaugeOutlineMedium,
    IconSendOutlineRegular: IconSendOutlineRegular,
    IconSendOutlineMedium: IconSendOutlineMedium,
    IconQueueOutlineRegular: IconQueueOutlineRegular,
    IconQueueOutlineMedium: IconQueueOutlineMedium,
    IconChecklistOutlineRegular: IconChecklistOutlineRegular,
    IconChecklistOutlineMedium: IconChecklistOutlineMedium,
    IconListPenOutlineRegular: IconListPenOutlineRegular,
    IconListPenOutlineMedium: IconListPenOutlineMedium,
    IconGoalOutlineRegular: IconGoalOutlineRegular,
    IconGoalOutlineMedium: IconGoalOutlineMedium,
    IconSparkleRegular: IconSparkleRegular,
    IconSparkleMedium: IconSparkleMedium,
    IconInspectOutlineRegular: IconInspectOutlineRegular,
    IconInspectOutlineMedium: IconInspectOutlineMedium,
    IconSkillOutlineRegular: IconSkillOutlineRegular,
    IconSkillOutlineMedium: IconSkillOutlineMedium,
    IconQuestionOutlineRegular: IconQuestionOutlineRegular,
    IconQuestionOutlineMedium: IconQuestionOutlineMedium,
    IconInfoOutlineRegular: IconInfoOutlineRegular,
    IconInfoOutlineMedium: IconInfoOutlineMedium,
    IconPluginPinwheelOutlineRegular: IconPluginPinwheelOutlineRegular,
    IconPluginPinwheelOutlineMedium: IconPluginPinwheelOutlineMedium,
    IconAlarmClockOutlineRegular: IconAlarmClockOutlineRegular,
    IconAlarmClockOutlineMedium: IconAlarmClockOutlineMedium,
    IconArchiveOutlineRegular: IconArchiveOutlineRegular,
    IconArchiveOutlineMedium: IconArchiveOutlineMedium,
    IconWrapLinesOutlineRegular: IconWrapLinesOutlineRegular,
    IconWrapLinesOutlineMedium: IconWrapLinesOutlineMedium,
    IconNowrapFillRegular: IconNowrapFillRegular,
    IconNowrapFillMedium: IconNowrapFillMedium,
    IconWrapFillRegular: IconWrapFillRegular,
    IconWrapFillMedium: IconWrapFillMedium,
    IconCompareSplitOutlineRegular: IconCompareSplitOutlineRegular,
    IconCompareSplitOutlineMedium: IconCompareSplitOutlineMedium,
    IconPlanOutlineRegular: IconPlanOutlineRegular,
    IconPlanOutlineMedium: IconPlanOutlineMedium,
    IconCompactOutlineRegular: IconCompactOutlineRegular,
    IconCompactOutlineMedium: IconCompactOutlineMedium,
    IconShieldOutlineRegular: IconShieldOutlineRegular,
    IconShieldOutlineMedium: IconShieldOutlineMedium,
    IconCheckCircleOutlineRegular: IconCheckCircleOutlineRegular,
    IconCheckCircleOutlineMedium: IconCheckCircleOutlineMedium,
    IconUnarchiveOutlineRegular: IconUnarchiveOutlineRegular,
    IconUnarchiveOutlineMedium: IconUnarchiveOutlineMedium,
    IconPinOutlineRegular: IconPinOutlineRegular,
    IconPinOutlineMedium: IconPinOutlineMedium,
    IconPinFillRegular: IconPinFillRegular,
    IconPinFillMedium: IconPinFillMedium,
    IconFlatListOutlineRegular: IconFlatListOutlineRegular,
    IconFlatListOutlineMedium: IconFlatListOutlineMedium,
    IconWorkspaceTreeOutlineRegular: IconWorkspaceTreeOutlineRegular,
    IconWorkspaceTreeOutlineMedium: IconWorkspaceTreeOutlineMedium,
    IconChevronsUpDownOutlineRegular: IconChevronsUpDownOutlineRegular,
    IconChevronsUpDownOutlineMedium: IconChevronsUpDownOutlineMedium,
    IconDragHandleOutlineRegular: IconDragHandleOutlineRegular,
    IconDragHandleOutlineMedium: IconDragHandleOutlineMedium,
    IconArchiveOffOutlineRegular: IconArchiveOffOutlineRegular,
    IconArchiveOffOutlineMedium: IconArchiveOffOutlineMedium,
    IconArchiveCheckOutlineRegular: IconArchiveCheckOutlineRegular,
    IconArchiveCheckOutlineMedium: IconArchiveCheckOutlineMedium,
    IconSlidersTwoOutlineRegular: IconSlidersTwoOutlineRegular,
    IconSlidersTwoOutlineMedium: IconSlidersTwoOutlineMedium,
    IconMicrophoneOutlineRegular: IconMicrophoneOutlineRegular,
    IconMicrophoneOutlineMedium: IconMicrophoneOutlineMedium,
    IconSaveOutlineRegular: IconSaveOutlineRegular,
    IconSaveOutlineMedium: IconSaveOutlineMedium,
  };
  // Any icon name the reference adds later still resolves to a generic glyph
  // instead of undefined, so an unmodelled import never breaks a bundle.
  if (typeof Proxy === "function") {
    var iconFallbacks = {};
    surface = new Proxy(surface, {
      get: function (target, property) {
        if (property in target) return target[property];
        if (typeof property === "string" && /^Icon[A-Z]/.test(property)) {
          if (!iconFallbacks[property]) iconFallbacks[property] = makeProductIcon("info-outline", "regular");
          return iconFallbacks[property];
        }
        return undefined;
      },
    });
  }
  window.__DSHR_MODULES__ = window.__DSHR_MODULES__ || {};
  window.__DSHR_MODULES__["@deepseek-ai/dsh-client-ui-primitives"] = surface;
})();
