/* dshr design-system compatibility shim (C-lite/C2, rounds 79-85).
 *
 * A PARTIAL, independently implemented subset of
 * `@deepseek-ai/dsh-client-ui-primitives` for simple plugins. It is published in
 * the shared host module table under the reference id, so a DSH-style bundle
 * can `require("@deepseek-ai/dsh-client-ui-primitives")` and get these
 * components. This is not the upstream package: components, props and styling
 * are a small compatible surface built with host React and CSS variables from
 * the dshr theme, covering the primitives real conformance plugins ask for
 * (Button, Input, Switch, Checkbox, Tag, Pill, Menu, MenuSurface, Modal, Toast,
 * Tooltip, SegmentedControl, SegmentedTabs, DisclosureRow, PathLabel,
 * TextShimmer, JsonBlock, MarkdownText, StateDot, Spinner, icons, clipboard,
 * anchored-position/outside-dismiss hooks). */
(function () {
  var React = window.React;
  var ReactDOM = window.ReactDOM;
  if (!React) {
    throw new Error("dshr ui primitives: React is missing from the host runtime");
  }

  var styleId = "dshr-ui-primitives";
  if (!document.querySelector('style[data-dshr-primitives="' + styleId + '"]')) {
    var style = document.createElement("style");
    style.setAttribute("data-dshr-primitives", styleId);
    style.textContent = [
      ".dshr-button{font:inherit;color:var(--dsw-alias-label-primary,inherit);background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:6px;padding:4px 10px;cursor:pointer;display:inline-flex;align-items:center;gap:6px}",
      ".dshr-button:hover{border-color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-button:disabled{opacity:.5;cursor:default}",
      ".dshr-button-primary{background:var(--dsw-alias-brand-primary,#4da3ff);color:#fff;border-color:transparent}",
      ".dshr-button-ghost{background:transparent;border-color:transparent}",
      ".dshr-button-sm{font-size:12px;padding:3px 8px}",
      ".dshr-state-dot{display:inline-block;width:8px;height:8px;border-radius:50%;background:var(--dsw-alias-state-idle-primary,#8a8a8a);vertical-align:middle}",
      ".dshr-state-dot-success{background:var(--dsw-alias-state-success-primary,#22c55e)}",
      ".dshr-state-dot-warning{background:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-state-dot-error{background:var(--dsw-alias-state-error-primary,#ef4444)}",
      ".dshr-state-dot-ongoing{background:var(--dsw-alias-state-business-primary,#4da3ff)}",
      ".dshr-tooltip{display:inline-flex;align-items:center;gap:4px}",
      ".dshr-icon{display:inline-flex;align-items:center;justify-content:center;line-height:1;flex:none}",
      ".dshr-spinner{display:inline-block;width:12px;height:12px;border:2px solid var(--dsw-alias-border-secondary,#3a3a3a);border-top-color:var(--dsw-alias-brand-primary,#4da3ff);border-radius:50%;animation:dshr-spin .8s linear infinite}",
      ".dshr-json-block{margin:6px 0;padding:8px;border-radius:6px;background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));overflow-x:auto;font:12px/1.4 ui-monospace,monospace}",
      ".dshr-markdown-text{white-space:pre-wrap;overflow-wrap:anywhere}",
      ".dshr-input{display:inline-flex;align-items:center;gap:6px;padding:3px 8px;border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:6px;background:var(--dsw-alias-bg-layer-1,transparent);min-width:0}",
      ".dshr-input input{font:inherit;color:inherit;background:transparent;border:0;outline:0;min-width:0;flex:1}",
      ".dshr-switch{display:inline-flex;align-items:center;gap:8px;background:transparent;border:0;color:inherit;font:inherit;cursor:pointer;padding:0}",
      ".dshr-switch-track{width:28px;height:16px;border-radius:999px;background:var(--dsw-alias-border-secondary,#3a3a3a);position:relative;transition:background .15s}",
      ".dshr-switch-track.dshr-on{background:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-switch-knob{position:absolute;top:2px;left:2px;width:12px;height:12px;border-radius:50%;background:#fff;transition:left .15s}",
      ".dshr-switch-track.dshr-on .dshr-switch-knob{left:14px}",
      ".dshr-checkbox{display:inline-flex;align-items:center;gap:6px;cursor:pointer;font:inherit;color:inherit}",
      ".dshr-tag{display:inline-flex;align-items:center;border-radius:4px;padding:1px 6px;font-size:11px;line-height:1.5;border:1px solid var(--dsw-alias-border-secondary,#3a3a3a)}",
      ".dshr-tag-neutral{background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12))}",
      ".dshr-tag-info{border-color:var(--dsw-alias-brand-primary,#4da3ff);color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-tag-solid{background:var(--dsw-alias-brand-primary,#4da3ff);color:#fff;border-color:transparent}",
      ".dshr-pill{display:inline-flex;align-items:center;gap:4px;border-radius:999px;padding:2px 9px;font-size:12px;border:1px solid transparent;background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.12));color:inherit;cursor:pointer;font:inherit}",
      ".dshr-pill-static{cursor:default}",
      ".dshr-pill-active{outline:1px solid var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-menu{position:fixed;margin:0;padding:4px;list-style:none;min-width:140px;max-height:320px;overflow:auto;background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,.35);z-index:80}",
      ".dshr-menu-item{display:block;width:100%;text-align:left;background:transparent;border:0;color:inherit;font:inherit;padding:6px 8px;border-radius:6px;cursor:pointer}",
      ".dshr-menu-item:hover{background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.16))}",
      ".dshr-menu-item-selected{color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-menu-item:disabled{opacity:.5;cursor:default}",
      ".dshr-menu-surface{background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:10px;box-shadow:0 8px 24px rgba(0,0,0,.35);padding:8px;outline:none}",
      ".dshr-modal-backdrop{position:fixed;inset:0;background:rgba(0,0,0,.45);display:flex;align-items:center;justify-content:center;z-index:90;padding:16px}",
      ".dshr-modal{width:min(560px,100%);max-height:80vh;overflow:auto;background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:12px;padding:16px}",
      ".dshr-modal-title{margin:0 0 4px;font-size:15px}",
      ".dshr-modal-description{margin:0 0 10px;opacity:.75;font-size:12px}",
      ".dshr-modal-close{position:absolute;top:10px;right:10px}",
      ".dshr-modal-wrap{position:relative}",
      ".dshr-modal-footer{display:flex;justify-content:flex-end;gap:8px;margin-top:12px}",
      ".dshr-toast{position:fixed;left:50%;bottom:24px;transform:translateX(-50%);z-index:100;display:inline-flex;align-items:center;gap:8px;padding:8px 12px;border-radius:8px;background:var(--dsw-alias-bg-layer-1,#1f1f1f);border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);box-shadow:0 8px 24px rgba(0,0,0,.35);font-size:12px}",
      ".dshr-toast-success{border-color:var(--dsw-alias-state-success-primary,#22c55e)}",
      ".dshr-toast-error{border-color:var(--dsw-alias-state-error-primary,#ef4444)}",
      ".dshr-toast-warning{border-color:var(--dsw-alias-state-warn-primary,#f59e0b)}",
      ".dshr-segmented{display:inline-flex;border:1px solid var(--dsw-alias-border-secondary,#3a3a3a);border-radius:6px;overflow:hidden}",
      ".dshr-segmented button{font:inherit;background:transparent;border:0;border-right:1px solid var(--dsw-alias-border-secondary,#3a3a3a);color:inherit;padding:4px 10px;cursor:pointer;flex:1}",
      ".dshr-segmented button:last-child{border-right:0}",
      ".dshr-segmented button.dshr-selected{background:var(--dsw-alias-bg-layer-2,rgba(127,127,127,.16));color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-segmented button:disabled{opacity:.5;cursor:default}",
      ".dshr-tabs{display:flex;gap:4px;border-bottom:1px solid var(--dsw-alias-border-secondary,#3a3a3a)}",
      ".dshr-tabs button{font:inherit;background:transparent;border:0;border-bottom:2px solid transparent;color:inherit;padding:5px 10px;cursor:pointer}",
      ".dshr-tabs button.dshr-selected{border-bottom-color:var(--dsw-alias-brand-primary,#4da3ff);color:var(--dsw-alias-brand-primary,#4da3ff)}",
      ".dshr-disclosure{display:block}",
      ".dshr-disclosure-header{display:flex;align-items:center;gap:8px;width:100%;background:transparent;border:0;color:inherit;font:inherit;padding:4px 0;cursor:pointer;text-align:left}",
      ".dshr-disclosure-title{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}",
      ".dshr-disclosure-content{padding:4px 0 4px 16px}",
      ".dshr-path-label{font:12px/1.4 ui-monospace,monospace;overflow-wrap:anywhere}",
      ".dshr-text-shimmer{background:linear-gradient(90deg,currentColor 25%,transparent 50%,currentColor 75%);background-size:200% 100%;-webkit-background-clip:text;background-clip:text;color:transparent;animation:dshr-shimmer 1.4s linear infinite}",
      "@keyframes dshr-spin{to{transform:rotate(360deg)}}",
      "@keyframes dshr-shimmer{to{background-position:-200% 0}}",
    ].join("\n");
    document.head.appendChild(style);
  }

  function classNames(base, extra) {
    if (!extra) return base;
    return base ? base + " " + extra : extra;
  }

  function passThrough(props, excluded) {
    var rest = {};
    Object.keys(props || {}).forEach(function (key) {
      if (excluded.indexOf(key) === -1) rest[key] = props[key];
    });
    return rest;
  }

  function Button(props) {
    var rest = passThrough(props, ["variant", "size", "className", "children"]);
    if (!rest.type) rest.type = "button";
    rest.className = classNames(
      "dshr-button dshr-button-" + (props.variant || "secondary") +
        (props.size === "sm" ? " dshr-button-sm" : ""),
      props.className,
    );
    return React.createElement("button", rest, props.children);
  }

  var Input = React.forwardRef(function Input(props, ref) {
    var rest = passThrough(props, ["icon", "className"]);
    return React.createElement(
      "span",
      { className: classNames("dshr-input", props.className) },
      props.icon || null,
      React.createElement("input", Object.assign({ ref: ref }, rest)),
    );
  });

  function StateDot(props) {
    return React.createElement("span", {
      className: classNames("dshr-state-dot dshr-state-dot-" + (props.state || "idle"), props.className),
      title: props.label || props.title || "",
    });
  }

  function Tag(props) {
    var rest = passThrough(props, ["tone", "className", "children"]);
    return React.createElement(
      "span",
      Object.assign(rest, {
        className: classNames("dshr-tag dshr-tag-" + (props.tone || "outline"), props.className),
      }),
      props.children,
    );
  }

  function Pill(props) {
    var rest = passThrough(props, ["active", "className", "children"]);
    var interactive = props.onClick !== undefined;
    rest.className = classNames(
      "dshr-pill" +
        (interactive ? "" : " dshr-pill-static") +
        (props.active ? " dshr-pill-active" : ""),
      props.className,
    );
    if (interactive) {
      rest.type = "button";
      return React.createElement("button", rest, props.children);
    }
    return React.createElement("span", rest, props.children);
  }

  function Switch(props) {
    var checked = props.checked === true;
    return React.createElement(
      "button",
      {
        type: "button",
        role: "switch",
        "aria-checked": checked,
        className: classNames("dshr-switch", props.className),
        onClick: function () {
          if (!props.disabled && typeof props.onChange === "function") props.onChange(!checked);
        },
        disabled: props.disabled === true,
        title: props.title || "",
      },
      React.createElement(
        "span",
        { className: "dshr-switch-track" + (checked ? " dshr-on" : "") },
        React.createElement("span", { className: "dshr-switch-knob" }),
      ),
      props.label !== undefined ? React.createElement("span", null, props.label) : null,
    );
  }

  function Checkbox(props) {
    var rest = passThrough(props, ["checked", "disabled", "label", "onChange", "className"]);
    return React.createElement(
      "label",
      { className: classNames("dshr-checkbox", props.className) },
      React.createElement("input", Object.assign(rest, {
        type: "checkbox",
        checked: props.checked === true,
        disabled: props.disabled === true,
        onChange: function (event) {
          if (typeof props.onChange === "function") props.onChange(event.target.checked);
        },
      })),
      props.label !== undefined ? React.createElement("span", null, props.label) : null,
    );
  }

  function tooltipRender(props) {
    var rest = passThrough(props, ["content", "children", "className", "title"]);
    return React.createElement(
      "span",
      Object.assign(rest, {
        className: classNames("dshr-tooltip", props.className),
        title: props.title || (typeof props.content === "string" ? props.content : ""),
      }),
      props.children,
    );
  }
  var Tooltip = React.forwardRef(function TooltipForward(props, ref) {
    return React.cloneElement(tooltipRender(props), { ref: ref });
  });

  function Spinner(props) {
    return React.createElement("span", {
      className: classNames("dshr-spinner", props.className),
      role: "status",
      "aria-label": props.label || "loading",
    });
  }

  var iconPaths = {
    close: ["M3 3l10 10", "M13 3L3 13"],
    "chevron-down": ["M3 6l5 5 5-5"],
    "chevron-up": ["M3 10l5-5 5 5"],
    "chevron-left": ["M10 3L5 8l5 5"],
    "chevron-right": ["M6 3l5 5-5 5"],
    search: ["M7 2a5 5 0 100 10A5 5 0 007 2z", "M10.5 10.5L14 14"],
    settings: ["M8 5.5A2.5 2.5 0 108 10.5 2.5 2.5 0 008 5.5z", "M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1 1M11.6 11.6l1 1M12.6 3.4l-1 1M4.4 11.6l-1 1"],
    link: ["M6 9a3 3 0 004.2 0l2.3-2.3a3 3 0 00-4.2-4.2L7 3.8", "M10 7a3 3 0 00-4.2 0L3.5 9.3a3 3 0 004.2 4.2l1.3-1.3"],
    refresh: ["M13 8a5 5 0 11-1.5-3.5", "M13 1.5V5h-3.5"],
    warning: ["M8 2L1.5 13h13L8 2z", "M8 6v3", "M8 11v.5"],
    skill: ["M8 2l1.6 4.4L14 8l-4.4 1.6L8 14l-1.6-4.4L2 8l4.4-1.6L8 2z"],
    check: ["M2.5 8l3.5 3.5L13.5 4"],
    info: ["M8 2a6 6 0 100 12A6 6 0 008 2z", "M8 7v4", "M8 5v.5"],
    plus: ["M8 2.5v11", "M2.5 8h11"],
    trash: ["M2.5 4.5h11", "M6 4.5V3h4v1.5", "M4 4.5L4.7 13h6.6L12 4.5"],
    copy: ["M5 5h8v8H5z", "M3 3h8v2", "M3 3v8h2"],
  };

  function iconComponent(kind) {
    var paths = iconPaths[kind] || iconPaths.info;
    return function Icon(props) {
      var size = (props && props.size) || 16;
      var children = paths.map(function (path, index) {
        return React.createElement("path", {
          key: index,
          d: path,
          fill: "none",
          stroke: "currentColor",
          strokeWidth: 1.5,
          strokeLinecap: "round",
          strokeLinejoin: "round",
        });
      });
      return React.createElement(
        "svg",
        {
          width: size,
          height: size,
          viewBox: "0 0 16 16",
          className: classNames("dshr-icon", props && props.className),
          "aria-hidden": "true",
          focusable: "false",
        },
        children,
      );
    };
  }

  var Icon = iconComponent("info");
  var IconCloseOutlineRegular = iconComponent("close");
  var IconChevronDownOutlineRegular = iconComponent("chevron-down");
  var IconChevronUpOutlineRegular = iconComponent("chevron-up");
  var IconChevronLeftOutlineRegular = iconComponent("chevron-left");
  var IconChevronRightOutlineRegular = iconComponent("chevron-right");
  var IconChevronDownOutline14 = IconChevronDownOutlineRegular;
  var IconSearchOutlineRegular = iconComponent("search");
  var IconSettingsOutlineRegular = iconComponent("settings");
  var IconLinkOutlineRegular = iconComponent("link");
  var IconRefreshOutlineRegular = iconComponent("refresh");
  var IconWarningOutlineRegular = iconComponent("warning");
  var IconWarningOutlineMedium = IconWarningOutlineRegular;
  var IconSkillOutlineRegular = iconComponent("skill");
  var IconCheckOutlineRegular = iconComponent("check");
  var IconInfoOutlineRegular = iconComponent("info");
  var IconPlusOutlineRegular = iconComponent("plus");
  var IconTrashOutlineRegular = iconComponent("trash");
  var IconCopyOutlineRegular = iconComponent("copy");

  function useAnchoredPosition(options) {
    var opts = options || {};
    var state = React.useState(null);
    var setPosition = state[1];
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
        var panelWidth = panel ? panel.offsetWidth : 0;
        var panelHeight = panel ? panel.offsetHeight : 0;
        var left = opts.align === "end" ? rect.right - panelWidth : rect.left;
        var top = opts.side === "top" ? rect.top - panelHeight - gap : rect.bottom + gap;
        left = Math.max(margin, Math.min(left, window.innerWidth - panelWidth - margin));
        top = Math.max(margin, Math.min(top, window.innerHeight - panelHeight - margin));
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
    return state[0];
  }

  function useDismissOnOutsidePointer(panelRef, open, onDismiss, anchorRef) {
    var callbackRef = React.useRef(onDismiss);
    callbackRef.current = onDismiss;
    React.useEffect(function () {
      if (!open) return undefined;
      function handle(event) {
        var target = event.target;
        var panel = panelRef && panelRef.current;
        var anchor = anchorRef && anchorRef.current;
        if (panel && panel.contains(target)) return;
        if (anchor && anchor.contains(target)) return;
        if (typeof callbackRef.current === "function") callbackRef.current(event);
      }
      document.addEventListener("pointerdown", handle, true);
      return function () {
        document.removeEventListener("pointerdown", handle, true);
      };
    }, [open, panelRef, anchorRef]);
  }

  var Menu = function Menu(props) {
    var anchorWrapRef = React.useRef(null);
    var panelRef = React.useRef(null);
    var open = props.open === true;
    useDismissOnOutsidePointer(panelRef, open && !props.anchor, function () {
      if (typeof props.onClose === "function") props.onClose();
    }, anchorWrapRef);
    React.useEffect(function () {
      if (!open) return undefined;
      function onKey(event) {
        if (event.key === "Escape" && typeof props.onClose === "function") props.onClose();
      }
      document.addEventListener("keydown", onKey);
      return function () {
        document.removeEventListener("keydown", onKey);
      };
    }, [open, props.onClose]);
    var anchorElement = props.anchor
      ? React.createElement("span", { ref: anchorWrapRef, className: props.className }, props.anchor)
      : null;
    if (!open) return anchorElement;
    var rect = anchorWrapRef.current
      ? anchorWrapRef.current.getBoundingClientRect()
      : { left: 8, right: 8, top: 8, bottom: 8 };
    var style = { position: "fixed", zIndex: 80 };
    if (props.side === "top") style.bottom = Math.max(8, window.innerHeight - rect.top + 4);
    else style.top = rect.bottom + 4;
    if (props.align === "end") style.left = Math.max(8, rect.right - 180);
    else style.left = rect.left;
    var list = React.createElement(
      "ul",
      { ref: panelRef, role: "menu", className: classNames("dshr-menu", props.listClassName), style: style },
      (props.items || []).map(function (item) {
        return React.createElement(
          "li",
          { key: item.id, role: "none" },
          React.createElement(
            "button",
            {
              type: "button",
              role: "menuitem",
              disabled: item.disabled === true,
              className: classNames(
                "dshr-menu-item",
                item.id === props.selectedId ? "dshr-menu-item-selected" : "",
              ),
              onClick: function () {
                if (item.disabled) return;
                if (typeof props.onSelect === "function") props.onSelect(item.id);
                if (typeof props.onClose === "function") props.onClose();
              },
            },
            item.label,
          ),
        );
      }),
    );
    var panel = props.portal && ReactDOM
      ? ReactDOM.createPortal(list, document.body)
      : list;
    return React.createElement(React.Fragment, null, anchorElement, panel);
  };
  Menu.displayName = "Menu";

  var MenuSurface = React.forwardRef(function MenuSurface(props, ref) {
    var rest = passThrough(props, ["className", "children"]);
    rest.className = classNames("dshr-menu-surface", props.className);
    return React.createElement("div", Object.assign({ ref: ref }, rest), props.children);
  });

  function Modal(props) {
    if (!props.open) return null;
    var body = React.createElement(
      "div",
      {
        className: "dshr-modal-backdrop",
        onPointerDown: function (event) {
          if (event.target === event.currentTarget && typeof props.onClose === "function") props.onClose();
        },
      },
      React.createElement(
        "div",
        {
          className: classNames("dshr-modal-wrap", props.className),
          role: "dialog",
          "aria-modal": "true",
          "aria-label": typeof props.title === "string" ? props.title : undefined,
        },
        React.createElement(
          "div",
          { className: "dshr-modal" },
          typeof props.onClose === "function"
            ? React.createElement(
                "button",
                {
                  type: "button",
                  className: "dshr-button dshr-button-ghost dshr-modal-close",
                  onClick: props.onClose,
                  "aria-label": props.closeLabel || "close",
                },
                React.createElement(IconCloseOutlineRegular, { size: 14 }),
              )
            : null,
          props.title !== undefined
            ? React.createElement("h2", { className: "dshr-modal-title" }, props.title)
            : null,
          props.description !== undefined
            ? React.createElement("p", { className: "dshr-modal-description" }, props.description)
            : null,
          props.children,
          props.footer !== undefined
            ? React.createElement("div", { className: "dshr-modal-footer" }, props.footer)
            : null,
        ),
      ),
    );
    return ReactDOM ? ReactDOM.createPortal(body, document.body) : body;
  }

  function Toast(props) {
    React.useEffect(function () {
      if (typeof props.onDone !== "function") return undefined;
      var hold = typeof props.holdMs === "number" ? props.holdMs : 3000;
      var timer = setTimeout(function () {
        props.onDone();
      }, hold);
      return function () {
        clearTimeout(timer);
      };
    }, [props.onDone, props.holdMs, props.text]);
    return React.createElement(
      "div",
      { className: classNames("dshr-toast dshr-toast-" + (props.tone || "neutral")), role: "status" },
      props.icon || null,
      React.createElement("span", null, props.text),
    );
  }

  function SegmentedTabs(props) {
    var items = props.items || [];
    return React.createElement(
      "div",
      { role: "tablist", "aria-label": props.label, className: classNames("dshr-tabs", props.className) },
      items.map(function (item) {
        var selected = item.value === props.value;
        return React.createElement(
          "button",
          {
            key: item.value,
            id: item.id,
            type: "button",
            role: "tab",
            "aria-selected": selected,
            className: selected ? "dshr-selected" : "",
            onClick: function () {
              if (typeof props.onChange === "function") props.onChange(item.value);
            },
          },
          item.label,
        );
      }),
    );
  }

  function SegmentedControl(props) {
    var options = props.options || [];
    function onChange(value) {
      if (!props.disabled && typeof props.onChange === "function") props.onChange(value);
    }
    return React.createElement(
      "div",
      {
        role: "radiogroup",
        id: props.id,
        "aria-label": props.label,
        className: classNames("dshr-segmented", props.className),
        onKeyDown: function (event) {
          var enabled = options.filter(function (option) {
            return !(props.disabled || option.disabled);
          });
          var index = enabled.findIndex(function (option) {
            return option.value === props.value;
          });
          var step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
          var next;
          if (step !== undefined && enabled.length > 0) {
            var from = index < 0 ? 0 : index;
            next = enabled[(from + step + enabled.length) % enabled.length];
          } else if (event.key === "Home") {
            next = enabled[0];
          } else if (event.key === "End") {
            next = enabled[enabled.length - 1];
          } else {
            return;
          }
          if (!next) return;
          event.preventDefault();
          if (next.value !== props.value) onChange(next.value);
        },
      },
      options.map(function (option) {
        var selected = option.value === props.value;
        return React.createElement(
          "button",
          {
            key: option.value,
            type: "button",
            role: "radio",
            "aria-checked": selected,
            className: selected ? "dshr-selected" : "",
            disabled: props.disabled === true || option.disabled === true,
            onClick: function () {
              if (option.value !== props.value) onChange(option.value);
            },
          },
          option.label,
        );
      }),
    );
  }

  function DisclosureRow(props) {
    var header = React.createElement(
      "button",
      {
        type: "button",
        className: "dshr-disclosure-header",
        onClick: function () {
          if (props.expandable !== false && typeof props.onToggle === "function") props.onToggle(!props.open);
        },
        disabled: props.expandable === false,
      },
      props.icon || null,
      props.previewChevron !== false
        ? React.createElement(IconChevronRightOutlineRegular, {
            size: 12,
            className: props.open ? "dshr-disclosure-open" : "",
          })
        : null,
      React.createElement(
        "span",
        { className: classNames("dshr-disclosure-title", props.titleClassName) },
        props.title,
      ),
      props.open === false && props.collapsedContent !== undefined ? props.collapsedContent : null,
      props.headerAccessory !== undefined ? props.headerAccessory : null,
    );
    var content =
      props.open !== false && props.children !== undefined
        ? React.createElement(
            "div",
            { className: classNames("dshr-disclosure-content", props.contentClassName) },
            props.children,
          )
        : null;
    return React.createElement(
      "div",
      { className: classNames("dshr-disclosure", props.className), "aria-expanded": props.open === true },
      header,
      content,
    );
  }

  function PathLabel(props) {
    return React.createElement(
      "code",
      { className: classNames("dshr-path-label", props.className), title: props.path },
      props.path,
    );
  }

  function TextShimmer(props) {
    return React.createElement(
      "span",
      {
        className: classNames(
          "dshr-text-shimmer",
          props.active ? "" : "",
          props.className,
        ),
      },
      props.children,
    );
  }

  function JsonBlock(props) {
    var value = props.value !== undefined ? props.value : props.data !== undefined ? props.data : props.children;
    var text;
    try {
      text = typeof value === "string" ? value : JSON.stringify(value, null, 2);
    } catch (error) {
      text = String(value);
    }
    return React.createElement(
      "pre",
      { className: classNames("dshr-json-block", props.className) },
      text,
    );
  }

  function MarkdownText(props) {
    var value = props.source !== undefined ? props.source : props.text !== undefined ? props.text : props.children;
    var text = Array.isArray(value) ? value.join("\n") : value === undefined || value === null ? "" : String(value);
    return React.createElement(
      "div",
      { className: classNames("dshr-markdown-text", props.className) },
      text,
    );
  }

  function writeClipboard(text) {
    var value = String(text === undefined ? "" : text);
    if (navigator.clipboard && typeof navigator.clipboard.writeText === "function") {
      return navigator.clipboard.writeText(value);
    }
    return new Promise(function (resolve, reject) {
      try {
        var area = document.createElement("textarea");
        area.value = value;
        area.style.position = "fixed";
        area.style.opacity = "0";
        document.body.appendChild(area);
        area.select();
        var ok = document.execCommand("copy");
        document.body.removeChild(area);
        if (ok) resolve();
        else reject(new Error("clipboard copy failed"));
      } catch (error) {
        reject(error);
      }
    });
  }

  var surface = {
    Button: Button,
    Input: Input,
    Switch: Switch,
    Checkbox: Checkbox,
    Tag: Tag,
    Pill: Pill,
    StateDot: StateDot,
    Tooltip: Tooltip,
    Spinner: Spinner,
    Icon: Icon,
    Menu: Menu,
    MenuSurface: MenuSurface,
    Modal: Modal,
    Toast: Toast,
    SegmentedControl: SegmentedControl,
    SegmentedTabs: SegmentedTabs,
    DisclosureRow: DisclosureRow,
    PathLabel: PathLabel,
    TextShimmer: TextShimmer,
    JsonBlock: JsonBlock,
    MarkdownText: MarkdownText,
    writeClipboard: writeClipboard,
    useAnchoredPosition: useAnchoredPosition,
    useDismissOnOutsidePointer: useDismissOnOutsidePointer,
    IconCloseOutlineRegular: IconCloseOutlineRegular,
    IconChevronDownOutlineRegular: IconChevronDownOutlineRegular,
    IconChevronUpOutlineRegular: IconChevronUpOutlineRegular,
    IconChevronLeftOutlineRegular: IconChevronLeftOutlineRegular,
    IconChevronRightOutlineRegular: IconChevronRightOutlineRegular,
    IconChevronDownOutline14: IconChevronDownOutline14,
    IconSearchOutlineRegular: IconSearchOutlineRegular,
    IconSettingsOutlineRegular: IconSettingsOutlineRegular,
    IconLinkOutlineRegular: IconLinkOutlineRegular,
    IconRefreshOutlineRegular: IconRefreshOutlineRegular,
    IconWarningOutlineRegular: IconWarningOutlineRegular,
    IconWarningOutlineMedium: IconWarningOutlineMedium,
    IconSkillOutlineRegular: IconSkillOutlineRegular,
    IconCheckOutlineRegular: IconCheckOutlineRegular,
    IconInfoOutlineRegular: IconInfoOutlineRegular,
    IconPlusOutlineRegular: IconPlusOutlineRegular,
    IconTrashOutlineRegular: IconTrashOutlineRegular,
    IconCopyOutlineRegular: IconCopyOutlineRegular,
  };
  // Any icon name we have not modelled yet still resolves to a generic glyph, so
  // a real plugin that imports `IconFooOutlineRegular` renders instead of
  // crashing on `undefined` (C2).
  if (typeof Proxy === "function") {
    var iconFallbacks = {};
    surface = new Proxy(surface, {
      get: function (target, property) {
        if (property in target) return target[property];
        if (typeof property === "string" && /^Icon[A-Z]/.test(property)) {
          if (!iconFallbacks[property]) iconFallbacks[property] = iconComponent("info");
          return iconFallbacks[property];
        }
        return undefined;
      },
    });
  }
  window.__DSHR_MODULES__ = window.__DSHR_MODULES__ || {};
  window.__DSHR_MODULES__["@deepseek-ai/dsh-client-ui-primitives"] = surface;
})();
