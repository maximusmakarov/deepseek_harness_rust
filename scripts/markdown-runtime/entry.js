/* dshr shared host runtime: upstream markdown stack (round 103, C2.7).
 *
 * Bundled by scripts/build-markdown-runtime.mjs into
 * crates/server/assets/runtime/markdown.js and concatenated after
 * primitives.js by the server's shared-runtime artifact, so it overrides
 * `MarkdownText` in the `@deepseek-ai/dsh-client-ui-primitives` module-table
 * entry while every other export of the design-system port stays untouched.
 *
 * The renderer itself is the upstream component (mdast/micromark + GFM +
 * math/KaTeX + shiki + footnotes + incremental streaming); this file only
 * re-publishes it behind the dshr prop surface and installs the two style
 * sheets the standalone artifact has no other way to load (KaTeX layout,
 * shiki css-variable fallbacks).
 *
 * React identity: `react`, `react-dom` and `react/jsx-runtime` are aliased at
 * build time to the shims in ./shims, which re-export the shared host
 * instances from window.React/window.ReactDOM and
 * window.__DSHR_MODULES__["react/jsx-runtime"]. No second React is bundled.
 */
import { createElement as h, useMemo } from "react";
import {
  MarkdownText as UpstreamMarkdownText,
  extractMarkdownPlainText,
} from "@deepseek-ai/dsh-client-ui-primitives";
import katexCss from "katex/dist/katex.min.css";

/* Injected by scripts/build-markdown-runtime.mjs (pinned versions). */
const META = __DSHR_MARKDOWN_META__;

const table = (window.__DSHR_MODULES__ = window.__DSHR_MODULES__ || {});
const primitives = table["@deepseek-ai/dsh-client-ui-primitives"];

/* Reference CodeBlock defaults; the port had no copy labels at all, so an
 * English pair beats the reference's hard-coded Chinese one for dshr frames. */
const DEFAULT_CODE_LABELS = Object.freeze({ copyLabel: "Copy", copiedLabel: "Copied" });

/**
 * Structural CSS the upstream CSS modules cannot bring: their class objects are
 * stubbed to `{}` in the published ESM, so only the literal classes
 * (`md-code-block`, `.footnotes`, `.shiki`) and semantic tags are available.
 * `:where(html)` gives the shiki fallback palette zero specificity, so a real
 * theme sheet (which defines the same `--shiki-*` variables) always wins.
 */
const STRUCTURE_CSS = [
  ":where(html){--shiki-foreground:#24292f;--shiki-background:transparent;--shiki-token-keyword:#cf222e;--shiki-token-constant:#0550ae;--shiki-token-string:#0a3069;--shiki-token-comment:#6e7781;--shiki-token-function:#8250df;--shiki-token-parameter:#953800;--shiki-token-punctuation:#57606a;--shiki-token-link:#0969da;--shiki-token-string-expression:#0a3069}",
  ".dshr-markdown .md-code-block{margin:8px 0;border-radius:var(--dshr-radius-sm,6px);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.10));overflow:auto}",
  ".dshr-markdown .md-code-block>div:first-child{display:flex;align-items:center;justify-content:space-between;gap:8px;padding:4px 10px;font:11px/1.5 ui-monospace,SFMono-Regular,monospace;color:var(--dsw-alias-label-tertiary,#9b9b9b);border-bottom:1px solid var(--dsh-hairline-width,rgba(127,127,127,.18))}",
  ".dshr-markdown .md-code-block pre{margin:0;padding:8px 10px;background:transparent;overflow:auto}",
  ".dshr-markdown .md-code-block pre code{background:transparent}",
  ".dshr-markdown .md-code-block button{background:transparent;border:0;padding:0;font:inherit;color:inherit;cursor:pointer}",
  ".dshr-markdown .md-code-block button:hover{color:var(--dsw-alias-label-primary,#e6e6e6)}",
  ".dshr-markdown .shiki{background:var(--shiki-background,transparent)!important}",
  ".dshr-markdown table{border-collapse:collapse;margin:8px 0;font-size:13px}",
  ".dshr-markdown th,.dshr-markdown td{border:1px solid var(--dsw-alias-border-secondary,rgba(127,127,127,.28));padding:4px 8px;text-align:left}",
  ".dshr-markdown th{font-weight:600}",
  ".dshr-markdown .footnotes{margin-top:16px;font-size:12px;color:var(--dsw-alias-label-secondary,#b0b0b0)}",
  ".dshr-markdown .footnotes h2{font-size:12px}",
  ".dshr-markdown .katex-display{margin:8px 0;overflow-x:auto;overflow-y:hidden}",
].join("\n");

function injectStyle(id, css) {
  if (typeof document === "undefined") return;
  var selector = 'style[data-dshr-runtime="' + id + '"]';
  if (document.querySelector(selector)) return;
  var style = document.createElement("style");
  style.setAttribute("data-dshr-runtime", id);
  style.textContent = css;
  (document.head || document.documentElement).appendChild(style);
}

/**
 * Drop KaTeX's @font-face rules: the artifact carries no font files, so a
 * relative url(fonts/...) would 404 against the runtime route. The layout
 * classes stay, so formulas keep their structure on the document's fallback
 * glyphs (the full KaTeX font set belongs to the host theme package).
 */
function katexSheet(css) {
  return css.replace(/@font-face\{[^}]*\}/g, "");
}

/** Map the dshr `labels.code` extension onto the reference `codeLabels`. */
function labelCode(labels) {
  if (!labels || typeof labels !== "object") return DEFAULT_CODE_LABELS;
  if (labels.copyLabel !== undefined || labels.copiedLabel !== undefined) return labels;
  if (labels.copy !== undefined || labels.copied !== undefined) {
    return { copyLabel: labels.copy, copiedLabel: labels.copied };
  }
  return DEFAULT_CODE_LABELS;
}

/**
 * Upstream MarkdownText behind the prop surface dshr plugins already use:
 * reference `text`/`streaming`/`codeLabels`/`fileMentions` plus the port's
 * `labels`/`variant`/`className`/`textInteractionSource` extensions. The outer
 * div re-establishes the port's `.dshr-markdown` styling hooks; the upstream
 * renderer keeps its own (class-less, CSS-module-stubbed) root inside.
 */
function MarkdownText(props) {
  var codeLabelsProp = props.codeLabels;
  var labelsProp = props.labels;
  /* Reference-stable code labels: the streaming renderer drops its frozen-block
   * cache when this identity changes, so derive it from the caller's own
   * objects instead of building a fresh pair per render. */
  var codeLabels = useMemo(function () {
    return codeLabelsProp || labelCode(labelsProp && labelsProp.code);
  }, [codeLabelsProp, labelsProp]);
  var variant = props.variant === "compact" ? "compact" : "body";
  var className = ["dshr-markdown", variant === "compact" ? "dshr-markdown-compact" : "", props.className]
    .filter(Boolean)
    .join(" ");
  var wrapperProps = {
    className: className,
    "data-markdown-engine": META.upstream,
    "data-markdown-variant": variant === "compact" ? "compact" : undefined,
    "data-text-interaction-source": props.textInteractionSource ? "" : undefined,
  };
  var upstreamProps = {
    text: props.text === undefined || props.text === null ? "" : String(props.text),
    streaming: props.streaming === true,
    codeLabels: codeLabels,
    fileMentions: props.fileMentions,
  };
  return h("div", wrapperProps, h(UpstreamMarkdownText, upstreamProps));
}

function install() {
  injectStyle("markdown-structure", STRUCTURE_CSS);
  injectStyle("katex", katexSheet(katexCss));
  if (!primitives) {
    return { status: "skipped", reason: "the primitives module is not published" };
  }
  primitives.MarkdownText = MarkdownText;
  /* Additive reference export: the plain-text projection of the same grammar.
   * The port never published it, so this can only widen compatibility. */
  if (typeof extractMarkdownPlainText === "function") {
    primitives.extractMarkdownPlainText = extractMarkdownPlainText;
  }
  return { status: "installed" };
}

var result = { ok: false, error: null };
try {
  Object.assign(result, install(), { meta: META });
  result.ok = result.status === "installed";
} catch (error) {
  result.error = String((error && error.stack) || error);
  if (typeof console !== "undefined" && console.error) {
    console.error("[dshr-markdown] upstream markdown stack failed to install:", error);
  }
}
window.__DSHR_MARKDOWN_RUNTIME__ = result;
