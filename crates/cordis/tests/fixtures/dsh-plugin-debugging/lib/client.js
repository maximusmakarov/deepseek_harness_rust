window.__ModuleLoader__.load({
	id: "dsh-plugin-debugging",
	factory: (require) => {
		var module = { exports: {} };
		var exports = module.exports;
"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/client/index.tsx
var index_exports = {};
__export(index_exports, {
  apply: () => apply,
  inject: () => inject
});
module.exports = __toCommonJS(index_exports);
var import_react = require("react");
var import_dsh_client_ui_primitives = require("@deepseek-ai/dsh-client-ui-primitives");

// src/client/locales.ts
var LOCALE_NS = "dsh-plugin-debugging";
var zh = {
  readOnly: "\u8FD9\u4E2A\u8FDE\u63A5\u4E0D\u5141\u8BB8\u6539\u8BBE\u7F6E\u3002",
  enableRestartLabel: "\u5141\u8BB8 Agent \u91CD\u542F\u5BBF\u4E3B",
  // 状态行只留**后果**，不复述标题 —— 标题已经说了「允许重启」，再写一遍「Agent 可以重启」是同义反复。
  // 两半都要有：**中断**（用户要知道代价）与**随后自动恢复**（否则读成「对话就此没了」而不敢开）。
  enableRestartOn: "\u91CD\u542F\u4F1A\u4E2D\u65AD\u5BF9\u8BDD\uFF0C\u968F\u540E\u81EA\u52A8\u6062\u590D\u5E76\u7EE7\u7EED\u3002",
  enableRestartOff: "\u5BF9\u8BDD\u4E0D\u4F1A\u4E2D\u65AD\u3002",
  // 末句追加「仅 Web（Windows）」（维护者 2026-10-16 拍板，方案 D）。
  // 为什么落在**这一行**而不是标题或状态行：本行**无条件渲染**（`index.tsx` 里没有 `shown ?`），
  // 而状态行只在开关为「开」时才显示那句话 —— 桌面端用户进来看到的是**默认关**的那一支，
  // 限制写在那里等于对他不可见。
  // 为什么一句话盖住三档：真实判据是 `relaunchRouteFor` 的 `cli × win32`（`src/lib/carrier.ts`），
  // 桌面宿主（结构上没有通道）与 Linux/macOS（没有**已实现**的拉起路线）在**用户视角是同一件事**
  // ——「你这台机器上这个开关没用」。卡片是给人看的三行说明、不是判据表：三档的区分留在运行时
  // 拒绝输出（那里模型要读懂成因），这里只报「能不能用」。
  enableRestartHint: "\u6539\u52A8\u7ACB\u5373\u751F\u6548\uFF0C\u4E0D\u9700\u8981\u91CD\u542F\u3002\u4EC5 Web\uFF08Windows\uFF09\u3002",
  save: "\u4FDD\u5B58",
  saving: "\u4FDD\u5B58\u4E2D\u2026",
  saveRejected: "\u4FDD\u5B58\u5931\u8D25\uFF1A\u914D\u7F6E\u5728\u522B\u5904\u88AB\u6539\u8FC7\u4E86\u3002\u8BF7\u91CD\u65B0\u6253\u5F00\u8FD9\u4E2A\u9875\u9762\u518D\u8BD5\u3002"
};
var en = {
  readOnly: "This connection does not allow writing settings.",
  enableRestartLabel: "Let the Agent restart this Host",
  // 状态行只留后果，不复述标题（理由同中文那份）。
  // 与中文那份**逐条对应**：两半都在 —— `interrupts`（代价）+ `resumes them`（会自动接回来）。
  enableRestartOn: "A restart interrupts conversations, then resumes them.",
  enableRestartOff: "Conversations are unaffected.",
  // 末句逐条对应中文那份（含追加的「Web only (Windows)」）——理由与「为什么落在这一行」都在中文那份。
  enableRestartHint: "Takes effect immediately; no restart needed. Web only (Windows).",
  save: "Save",
  saving: "Saving\u2026",
  saveRejected: "Could not save: the configuration changed elsewhere. Please reopen this page and try again."
};
var RESTART_TOGGLE_LOCALES = { zh, en };

// src/client/index.tsx
var import_jsx_runtime = require("react/jsx-runtime");
var inject = ["slots", "locale"];
var BUNDLE_NAME = "dsh-plugin-debugging";
var ENTRY_ID = "dsh-plugin-debugging";
var ENABLE_FIELD = "enableRestart";
var PROBE_TIMEOUT_MS = 1e4;
var SERVICE_MISSING_WARNING = "[WARN] dsh-plugin-debugging: this Host provides no client configuration service (configForms), so the settings card cannot be registered. All eight tools keep working; the dsh_restart master switch stays off. Upgrade the Host to the version range this package declares in engines.dsh.";
var SERVICE_LATE_INFO = "[INFO] dsh-plugin-debugging: the client configuration service appeared after the probe window, so the earlier warning is withdrawn and the settings card is registered.";
var SLOT_MISSING_WARNING = "[WARN] dsh-plugin-debugging: this Host renders no plugins.bundle.config entry, so the settings card cannot be shown. All eight tools keep working. Mount the plugin as a bundle (dsh plugin --profile web add) to configure it in place.";
var SLOT_LATE_INFO = "[INFO] dsh-plugin-debugging: plugins.bundle.config appeared after the probe window, so the earlier warning is withdrawn and the settings card is registered.";
function configFormsOf(ctx) {
  return ctx.configForms;
}
function unrefTimer(timer) {
  const candidate = timer;
  if (typeof candidate === "object" && candidate !== null && "unref" in candidate && typeof candidate.unref === "function") {
    candidate.unref();
  }
}
var S = {
  form: { display: "flex", flexDirection: "column" },
  readOnly: {
    margin: "12px 0 0",
    fontSize: 12,
    lineHeight: 1.5,
    color: "var(--dsw-alias-label-tertiary)"
  },
  field: { display: "flex", flexDirection: "column", gap: 6, padding: "12px 0" },
  // 开关行：逐条对齐官方 SubagentModelSelectionCard 的 .toggleRow / .toggleLabel，
  // 右侧放原生 `Switch`（同一个 ui-primitives 包），因此外观与官方卡片一致。
  toggleRow: {
    display: "flex",
    alignItems: "flex-start",
    justifyContent: "space-between",
    gap: 16,
    fontSize: 13,
    lineHeight: 1.5,
    color: "var(--dsw-alias-label-primary)"
  },
  toggleLabel: { flex: 1, minWidth: 0, fontSize: 13, lineHeight: 1.5 },
  // 状态行：比说明行重一档（secondary 对 tertiary），一行说清开关当前的含义。
  stateNote: {
    margin: 0,
    fontSize: 12,
    lineHeight: 1.5,
    color: "var(--dsw-alias-label-secondary)"
  },
  hint: { margin: 0, fontSize: 12, lineHeight: 1.5, color: "var(--dsw-alias-label-tertiary)" },
  // 失败文本占满剩余宽度（flex:1），没有失败文本时按钮就靠左。
  footer: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    paddingTop: 16
  },
  // 保存按钮：**逐条对齐官方 `SettingsForm.module.css` 的 `.save`**（维护者 2026-10-15 指定）——
  // 官方设置页**不用**共享 Button（那是 md 档 36px 高），而是这一款紧凑定制按钮（5px 14px / 13px）。
  // 用共享 Button 会与同一张 Plugins 页上其它插件的保存按钮**长得不一样**，而它们本该是同一款。
  save: {
    appearance: "none",
    border: "1px solid transparent",
    borderRadius: "var(--dsw-radius-md)",
    padding: "5px 14px",
    font: "inherit",
    fontSize: 13,
    lineHeight: 1.5,
    cursor: "pointer",
    background: "var(--dsw-alias-label-primary)",
    color: "var(--dsw-alias-bg-layer-3)"
  },
  // 官方 `.save:disabled` 的 opacity 0.4 与默认光标，等价改写成对象形式。
  saveDisabled: { opacity: 0.4, cursor: "default" },
  // 错误色用有定义的那个令牌：原生照抄来的 `label-error` 在 ui-theme 里从未定义，
  // 写了不会报错但不会生效（见 [AGENTS.md](./AGENTS.md)）。
  failed: {
    flex: 1,
    minWidth: 0,
    margin: 0,
    fontSize: 12,
    lineHeight: 1.5,
    color: "var(--dsw-alias-state-error-primary)"
  }
};
function RestartCard({ form, t }) {
  const snapshot = (0, import_react.useSyncExternalStore)(
    (onChange) => form.subscribe(onChange),
    () => form.getSnapshot()
  );
  const section = snapshot.value ?? {};
  const [draft, setDraft] = (0, import_react.useState)(void 0);
  const [saving, setSaving] = (0, import_react.useState)(false);
  const [failed, setFailed] = (0, import_react.useState)("");
  const writable = snapshot.status === "ready" && snapshot.writable;
  const disabled = !writable || saving;
  const effective = section[ENABLE_FIELD] === true;
  const shown = draft ?? effective;
  const dirty = draft !== void 0 && draft !== effective;
  const blocked = !dirty || disabled;
  const save = async () => {
    setSaving(true);
    setFailed("");
    try {
      const ops = [{ op: "set", path: [ENABLE_FIELD], value: shown }];
      if (!await form.mutate(ops, snapshot.revision)) throw new Error(t("saveRejected"));
      setDraft(void 0);
    } catch (error) {
      setFailed(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };
  return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { style: S.form, children: [
    !writable && snapshot.status !== "loading" ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", { style: S.readOnly, role: "status", children: t("readOnly") }) : null,
    /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { style: S.field, children: [
      /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { style: S.toggleRow, children: [
        /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", { style: S.toggleLabel, children: t("enableRestartLabel") }),
        /* @__PURE__ */ (0, import_jsx_runtime.jsx)(
          import_dsh_client_ui_primitives.Switch,
          {
            checked: shown,
            label: t("enableRestartLabel"),
            disabled,
            onChange: (next) => {
              setDraft(next);
            }
          }
        )
      ] }),
      /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", { style: S.stateNote, children: t(shown ? "enableRestartOn" : "enableRestartOff") }),
      /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", { style: S.hint, children: t("enableRestartHint") })
    ] }),
    /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { style: S.footer, children: [
      failed !== "" ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", { style: S.failed, role: "status", children: failed }) : null,
      /* @__PURE__ */ (0, import_jsx_runtime.jsx)(
        "button",
        {
          type: "button",
          disabled: blocked,
          style: blocked ? { ...S.save, ...S.saveDisabled } : S.save,
          onClick: () => {
            void save();
          },
          children: saving ? t("saving") : t("save")
        }
      )
    ] })
  ] });
}
function apply(ctx) {
  ctx.effect(
    () => ctx.locale.register(LOCALE_NS, RESTART_TOGGLE_LOCALES),
    "dsh-plugin-debugging: card dictionaries"
  );
  let serviceDeclared = false;
  let slotDeclared = false;
  const armProbe = (warning, info, isDeclared, relevant = () => true) => {
    const ring = { timer: void 0, announced: false, settle: () => void 0 };
    ring.timer = setTimeout(() => {
      ring.timer = void 0;
      if (isDeclared()) return;
      ring.announced = true;
      if (relevant()) console.warn(warning);
    }, PROBE_TIMEOUT_MS);
    unrefTimer(ring.timer);
    ring.settle = () => {
      if (ring.timer !== void 0) clearTimeout(ring.timer);
      ring.timer = void 0;
      if (!ring.announced) return;
      ring.announced = false;
      console.info(info);
    };
    return ring;
  };
  let serviceProbe;
  ctx.effect(() => {
    serviceProbe = armProbe(SERVICE_MISSING_WARNING, SERVICE_LATE_INFO, () => serviceDeclared);
    return () => {
      if (serviceProbe?.timer !== void 0) clearTimeout(serviceProbe.timer);
      serviceProbe = void 0;
    };
  }, "dsh-plugin-debugging: configuration service probe");
  let slotProbe;
  ctx.effect(() => {
    slotProbe = armProbe(
      SLOT_MISSING_WARNING,
      SLOT_LATE_INFO,
      () => slotDeclared,
      () => serviceDeclared
    );
    return () => {
      if (slotProbe?.timer !== void 0) clearTimeout(slotProbe.timer);
      slotProbe = void 0;
    };
  }, "dsh-plugin-debugging: configuration slot probe");
  ctx.inject(["configForms"], (scoped) => {
    serviceDeclared = true;
    serviceProbe?.settle();
    const form = configFormsOf(scoped).get(ENTRY_ID);
    ctx.slots.inject("plugins.bundle.config", () => {
      slotDeclared = true;
      slotProbe?.settle();
      return ctx.slots.register(
        { name: "plugins.bundle.config", key: BUNDLE_NAME, locale: LOCALE_NS },
        (seat) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(RestartCard, { form, t: seat.t })
      );
    });
  });
}
		return module.exports;
	}
});
