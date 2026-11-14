/* dshr design-system compatibility shim (C-lite, round 79).
 *
 * A PARTIAL, independently implemented subset of
 * `@deepseek-ai/dsh-client-ui-primitives` for simple plugins. It is published in
 * the shared host module table under the reference id, so a DSH-style bundle
 * can `require("@deepseek-ai/dsh-client-ui-primitives")` and get these
 * components. This is not the upstream package: components, props and styling
 * are a small compatible surface (Button, StateDot, Tooltip, Icon, Spinner)
 * built with host React and CSS variables from the dshr theme. */
(function () {
  var React = window.React;
  if (!React) {
    throw new Error("dshr ui primitives: React is missing from the host runtime");
  }

  var styleId = "dshr-ui-primitives";
  if (!document.querySelector('style[data-dshr-primitives="' + styleId + '"]')) {
    var style = document.createElement("style");
    style.setAttribute("data-dshr-primitives", styleId);
    style.textContent = [
      ".dshr-button{font:inherit;color:var(--dsw-alias-label-primary,inherit);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:6px;padding:4px 10px;cursor:pointer}",
      ".dshr-button:hover{border-color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-button-primary{background:var(--dsw-alias-brand-primary,#4da3ff);color:#fff;border-color:transparent}",
      ".dshr-button-ghost{background:transparent;border-color:transparent}",
      ".dshr-state-dot{display:inline-block;width:8px;height:8px;border-radius:50%;background:var(--dsw-alias-state-idle-primary,#8a8a8a);vertical-align:middle}",
      ".dshr-state-dot-success{background:var(--dsw-alias-state-success-primary,#22c55e)}",
      ".dshr-state-dot-warning{background:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-state-dot-error{background:var(--dsw-alias-state-error-primary,#ef4444)}",
      ".dshr-state-dot-ongoing{background:var(--dsw-alias-state-business-primary,#4da3ff)}",
      ".dshr-tooltip{display:inline-flex;align-items:center;gap:4px}",
      ".dshr-icon{display:inline-flex;align-items:center;justify-content:center;line-height:1}",
      ".dshr-spinner{display:inline-block;width:12px;height:12px;border:2px solid var(--dsw-alias-border-secondary,#3a3a3a);border-top-color:var(--dsw-alias-brand-primary,#4da3ff);border-radius:50%;animation:dshr-spin .8s linear infinite}",
      "@keyframes dshr-spin{to{transform:rotate(360deg)}}",
    ].join("\n");
    document.head.appendChild(style);
  }

  function classNames(base, extra) {
    return extra ? base + " " + extra : base;
  }

  function Button(props) {
    var rest = {};
    Object.keys(props || {}).forEach(function (key) {
      if (key !== "variant" && key !== "className" && key !== "children") rest[key] = props[key];
    });
    if (!rest.type) rest.type = "button";
    rest.className = classNames(
      "dshr-button dshr-button-" + (props.variant || "secondary"),
      props.className,
    );
    return React.createElement("button", rest, props.children);
  }

  function StateDot(props) {
    return React.createElement("span", {
      className: classNames("dshr-state-dot dshr-state-dot-" + (props.state || "idle"), props.className),
      title: props.label || props.title || "",
      "aria-label": props.label || props.state || "idle",
    });
  }

  function Tooltip(props) {
    return React.createElement(
      "span",
      { className: classNames("dshr-tooltip", props.className), title: props.label || props.content || "" },
      props.children,
    );
  }

  function Icon(props) {
    return React.createElement(
      "span",
      {
        className: classNames("dshr-icon" + (props.name ? " dshr-icon-" + props.name : ""), props.className),
        "aria-hidden": "true",
      },
      props.children || null,
    );
  }

  function Spinner(props) {
    return React.createElement("span", {
      className: classNames("dshr-spinner", props.className),
      "aria-hidden": "true",
    });
  }

  window.__DSHR_MODULES__ = window.__DSHR_MODULES__ || {};
  window.__DSHR_MODULES__["@deepseek-ai/dsh-client-ui-primitives"] = {
    Button: Button,
    StateDot: StateDot,
    Tooltip: Tooltip,
    Icon: Icon,
    Spinner: Spinner,
  };
})();
