/**
 * dsh-session-superagent — browser half.
 *
 * Registers one list entry into `conversation.view` (the centre-column view
 * ring: chat / trajectory / waterfall live there), so the panel appears as a
 * tab beside them. It reads the read-only host route the server half
 * registers; no credential ever crosses this boundary.
 */
window.__ModuleLoader__.load({
  id: 'dsh-session-superagent',
  factory(require) {
    const React = require('react')
    const h = React.createElement
    const { useCallback, useEffect, useRef, useState } = React

    // client.js 是 UMD：宿主原语只能走 factory 注入的 require，不能 import 相对 ESM 适配模块。
    // 这里的参数与 ui-host-primitives.js 的 createHostUi 保持一致，仅接入本面板需要的 Modal/Toast/Tooltip。
    function dismissToast(current, id) {
      return current && current.id === id ? null : current
    }

    function createHostUi({ React: HostReact, primitives } = {}) {
      if (!HostReact || typeof HostReact.createElement !== 'function' || typeof HostReact.isValidElement !== 'function') {
        throw new TypeError('createHostUi 需要宿主 React.createElement 与 React.isValidElement')
      }
      if (!primitives || (typeof primitives !== 'object' && typeof primitives !== 'function')) {
        throw new TypeError('createHostUi 需要 DSH primitives')
      }
      const hostH = HostReact.createElement
      const primitive = (name) => {
        const value = primitives[name]
        if (!value) throw new Error('DSH primitives.' + name + ' 不可用')
        return value
      }
      const HostModal = ({ open, title, description, footer, onClose, className, children } = {}) => {
        if (!open) return null
        return hostH(primitive('Modal'), {
          open, title, description, footer, onClose, className, closeLabel: '关闭',
        }, children)
      }
      const HostToast = ({ item, onDone } = {}) => {
        if (item === null || item === undefined) return null
        return hostH(primitive('Toast'), {
          key: item.id,
          text: item.text,
          tone: item.tone,
          holdMs: 3000,
          // success 使用宿主内建勾；error 必须显式使用官方 warning icon。
          icon: item.tone === 'error' ? hostH(primitive('IconWarningOutlineMedium')) : undefined,
          // 宿主 onDone 不带参数；由 dismiss 按 id 防止旧回调清除新通知。
          onDone: () => onDone(item.id),
        })
      }
      // DSH 只给 trigger 合并 aria-describedby，不维护 aria-expanded。
      // forwardRef 把宿主 ref/事件原样落到原 button；expanded 仅映射 DSH 已管理的 describedby，不另管 open。
      const TooltipTrigger = primitives.Tooltip && typeof HostReact.forwardRef === 'function'
        ? HostReact.forwardRef(function InfoTipTrigger({ text, icon = 'question', warn, triggerRef, 'aria-describedby': describedBy, ...triggerProps }, ref) {
          const triggerIcon = HostReact.isValidElement(icon) ? icon : hostH(Icon, { name: icon, size: 14 })
          const setTriggerRefs = (node) => {
            if (triggerRef) triggerRef.current = node
            if (typeof ref === 'function') ref(node)
            else if (ref && typeof ref === 'object') ref.current = node
          }
          return hostH('button', {
            ...triggerProps,
            ref: setTriggerRefs,
            type: 'button',
            className: cx('dt-info', warn && 'dt-info-warn'),
            'aria-label': text,
            'aria-describedby': describedBy,
            'aria-expanded': Boolean(describedBy),
          }, triggerIcon)
        })
        : null
      const HostTooltip = ({ text, icon = 'question', warn = false } = {}) => {
        const TooltipPrimitive = primitive('Tooltip')
        if (!TooltipTrigger) throw new Error('DSH Tooltip 需要宿主 React.forwardRef')
        const [remountKey, setRemountKey] = HostReact.useState(0)
        const [scrollFocusSuppressed, setScrollFocusSuppressed] = HostReact.useState(false)
        const triggerNodeRef = HostReact.useRef(null)
        const restoreFocusRef = HostReact.useRef(false)
        HostReact.useEffect(() => {
          if (typeof window === 'undefined') return undefined
          const closeOnScroll = () => {
            const triggerNode = triggerNodeRef.current
            const activeElement = typeof document === 'undefined' ? null : document.activeElement
            // 滚动卸载前记录：只有焦点原本位于 trigger 或其内部才允许恢复。
            restoreFocusRef.current = Boolean(triggerNode && activeElement && (
              activeElement === triggerNode
              || (typeof triggerNode.contains === 'function' && triggerNode.contains(activeElement))
            ))
            setRemountKey((current) => current + 1)
            setScrollFocusSuppressed(restoreFocusRef.current)
          }
          // Tooltip 没有受控 open API；capture 覆盖主面板及任意嵌套滚动容器。
          window.addEventListener('scroll', closeOnScroll, true)
          return () => window.removeEventListener('scroll', closeOnScroll, true)
        }, [])
        HostReact.useEffect(() => {
          if (!restoreFocusRef.current) return undefined
          restoreFocusRef.current = false
          const triggerNode = triggerNodeRef.current
          try {
            if (triggerNode && typeof triggerNode.focus === 'function'
              && (!('isConnected' in triggerNode) || triggerNode.isConnected)) {
              // disabled 只覆盖这次同步回焦，避免 focus 让新 Tooltip 立即重开。
              triggerNode.focus({ preventScroll: true })
            }
          } finally {
            setScrollFocusSuppressed(false)
          }
          return undefined
        }, [remountKey])
        return hostH(TooltipPrimitive, {
          key: remountKey,
          label: text,
          side: 'top',
          portal: true,
          openOnClick: true,
          disabled: scrollFocusSuppressed,
          gap: 8,
        }, hostH(TooltipTrigger, { text, icon, warn, triggerRef: triggerNodeRef }))
      }
      const createToastState = () => {
        const [item, setItem] = HostReact.useState(null)
        const sequence = HostReact.useRef(0)
        const notify = HostReact.useCallback((text, tone = 'info') => {
          sequence.current += 1
          const id = sequence.current
          setItem({ id, text, tone })
          return id
        }, [])
        const dismiss = HostReact.useCallback((id) => {
          setItem((current) => dismissToast(current, id))
        }, [])
        return { item, notify, dismiss }
      }
      return { Modal: HostModal, Toast: HostToast, Tooltip: HostTooltip, createToastState }
    }

    // 以下来源：ui-orbit-popover.js（L84e）的同一薄适配，因 client.js 是 UMD 而在 factory 内镜像。
    // createOrbitPopover({React, primitives}) 的参数、OrbitPopover/useOrbitConfirm 调用面与来源模块一致；
    // React.createPortal 由调用方用宿主 require('react-dom').createPortal 注入，不伪造 primitives.createPortal。
    const DEFAULT_ORBIT_Z_INDEX = 1100
    // confirm 由父组件的 useOrbitConfirm 直接关闭 orbit，不经过 OrbitPopover.close；按 ref 关联原因，避免新增公开参数。
    const orbitCloseReasons = new WeakMap()

    function focusIfConnected(node) {
      if (!node || typeof node.focus !== 'function') return false
      if ('isConnected' in node && !node.isConnected) return false
      try {
        node.focus()
        return true
      } catch {
        return false
      }
    }

    function closeReason(detail) {
      if (detail === false) return { reason: 'outside-pointer' }
      if (typeof detail === 'string') return { reason: detail }
      if (detail && typeof detail === 'object' && typeof detail.reason === 'string') return detail
      return { reason: 'dismiss' }
    }

    // 单独抽出确认 hook 只为让 StatusOrbit 在 fallback 也无条件执行同样数量的 hooks；
    // 弹层创建仍严格要求完整 DSH 能力，缺少能力不会创建替代 Popover。
    function createOrbitConfirm({ React: HostReact }) {
      return function useOrbitConfirm({ orbitRef, onCloseOrbit, onRequestConfirm } = {}) {
        const orbitRefRef = HostReact.useRef(orbitRef)
        const onCloseOrbitRef = HostReact.useRef(onCloseOrbit)
        const onRequestConfirmRef = HostReact.useRef(onRequestConfirm)
        const restoreFocusRef = HostReact.useRef(null)
        orbitRefRef.current = orbitRef
        onCloseOrbitRef.current = onCloseOrbit
        onRequestConfirmRef.current = onRequestConfirm

        const requestConfirm = HostReact.useCallback((detail = {}) => {
          restoreFocusRef.current = 'orbit'
          const ref = orbitRefRef.current
          if (ref && (typeof ref === 'object' || typeof ref === 'function')) {
            orbitCloseReasons.set(ref, 'confirm')
          }
          if (typeof onCloseOrbitRef.current === 'function') {
            onCloseOrbitRef.current({ reason: 'confirm' })
          }
          const request = onRequestConfirmRef.current
          if (typeof request === 'function') {
            const extra = detail && typeof detail === 'object' ? detail : { value: detail }
            return request({ ...extra, reason: 'confirm', restoreFocus: 'orbit' })
          }
          return undefined
        }, [])

        const onConfirmClose = HostReact.useCallback(() => {
          if (restoreFocusRef.current !== 'orbit') return false
          restoreFocusRef.current = null
          const ref = orbitRefRef.current
          return focusIfConnected(ref && ref.current)
        }, [])

        return {
          onRequestConfirm: requestConfirm,
          onConfirmClose,
          restoreFocus: restoreFocusRef.current,
        }
      }
    }

    const useOrbitConfirm = createOrbitConfirm({ React })

    function createOrbitPopover({ React: HostReact, primitives } = {}) {
      if (!HostReact || typeof HostReact.createElement !== 'function') {
        throw new TypeError('createOrbitPopover 需要宿主 React.createElement')
      }
      for (const hook of ['useRef', 'useCallback', 'useEffect']) {
        if (typeof HostReact[hook] !== 'function') {
          throw new TypeError(`createOrbitPopover 需要宿主 React.${hook}`)
        }
      }
      if (typeof HostReact.createPortal !== 'function') {
        throw new TypeError('createOrbitPopover 需要宿主 React.createPortal（从 react-dom 注入）')
      }
      if (!primitives || (typeof primitives !== 'object' && typeof primitives !== 'function')) {
        throw new TypeError('createOrbitPopover 需要 DSH primitives')
      }

      const primitive = (name, kind) => {
        const value = primitives[name]
        if (kind === 'function' ? typeof value !== 'function' : !value) {
          throw new Error(`DSH primitives.${name} 不可用；OrbitPopover 不会创建替代弹层`)
        }
        return value
      }
      const useAnchoredPosition = primitive('useAnchoredPosition', 'function')
      const useDismissOnOutsidePointer = primitive('useDismissOnOutsidePointer', 'function')
      const MenuSurface = primitive('MenuSurface')
      const hostH = HostReact.createElement
      const createPortal = HostReact.createPortal

      function OrbitPopover({
        open,
        anchorRef,
        orbitRef,
        panelRef: suppliedPanelRef,
        onClose,
        width,
        zIndex = DEFAULT_ORBIT_Z_INDEX,
        portalTarget: suppliedPortalTarget,
        className,
        style,
        ariaLabel,
        children,
      } = {}) {
        const internalPanelRef = HostReact.useRef(null)
        const panelRef = suppliedPanelRef || internalPanelRef
        const returnFocusRef = orbitRef || anchorRef
        const onCloseRef = HostReact.useRef(onClose)
        onCloseRef.current = onClose
        const didFocusRef = HostReact.useRef(false)
        const closeReasonRef = HostReact.useRef(null)

        const close = HostReact.useCallback((detail) => {
          const reason = closeReason(detail)
          closeReasonRef.current = reason.reason
          const callback = onCloseRef.current
          if (typeof callback === 'function') callback(reason)
        }, [])

        // 与来源适配器一致：宿主 hooks 和本地 effect 都在 open 分支前无条件调用。
        const position = useAnchoredPosition({
          open: Boolean(open),
          anchorRef,
          panelRef,
          side: 'top',
          align: 'end',
          gap: 8,
          margin: 8,
        })
        useDismissOnOutsidePointer(panelRef, Boolean(open), close, anchorRef)

        const portalTarget = suppliedPortalTarget === undefined
          ? (typeof document !== 'undefined' ? document.body : null)
          : suppliedPortalTarget
        const ownerDocument = portalTarget && portalTarget.ownerDocument
          ? portalTarget.ownerDocument
          : (typeof document !== 'undefined' ? document : null)

        HostReact.useEffect(() => {
          if (!open) {
            didFocusRef.current = false
            closeReasonRef.current = null
            orbitCloseReasons.delete(returnFocusRef)
            return undefined
          }
          // 首帧 position=null 时 surface 是 hidden，focus() 会被浏览器拒绝；等可见后只尝试一次。
          if (position && !didFocusRef.current) {
            didFocusRef.current = true
            focusIfConnected(panelRef.current)
          }
          if (!ownerDocument || typeof ownerDocument.addEventListener !== 'function') return undefined
          const onKeyDown = (event) => {
            if (!event || event.key !== 'Escape' || event.repeat) return
            if (typeof event.preventDefault === 'function') event.preventDefault()
            if (typeof event.stopPropagation === 'function') event.stopPropagation()
            close({ reason: 'escape' })
          }
          ownerDocument.addEventListener('keydown', onKeyDown, true)
          return () => {
            ownerDocument.removeEventListener('keydown', onKeyDown, true)
            closeReasonRef.current = closeReasonRef.current || orbitCloseReasons.get(returnFocusRef) || null
            if (closeReasonRef.current === 'escape') focusIfConnected(returnFocusRef && returnFocusRef.current)
            closeReasonRef.current = null
          }
        }, [open, position, close, ownerDocument, panelRef, returnFocusRef])

        if (!open) return null
        if (!anchorRef || typeof anchorRef !== 'object') {
          throw new TypeError('OrbitPopover 需要调用方传入 anchorRef')
        }
        if (!Number.isFinite(width) || width <= 0) {
          throw new TypeError('OrbitPopover 需要调用方传入正数 width；适配器不推导业务宽度')
        }
        if (!portalTarget) {
          throw new Error('OrbitPopover portal target 不可用；不会退化为非 portal surface')
        }
        // 与独立适配器一致：先缩到视口内，再交给 DSH 计算坐标。
        const viewportWidth = typeof window !== 'undefined' && Number.isFinite(window.innerWidth) ? window.innerWidth : width + 16
        const effectiveWidth = Math.min(width, Math.max(1, viewportWidth - 16))

        const surface = hostH(MenuSurface, {
          ref: panelRef,
          role: 'dialog',
          'aria-label': ariaLabel,
          tabIndex: -1,
          className,
          style: {
            ...style,
            position: 'fixed',
            left: position ? position.left : 0,
            top: position ? position.top : 0,
            width: effectiveWidth,
            zIndex,
            visibility: position ? 'visible' : 'hidden',
          },
        }, children)
        return createPortal(surface, portalTarget)
      }

      return { OrbitPopover, useOrbitConfirm: createOrbitConfirm({ React: HostReact }) }
    }

    // 老宿主没有这个映射时 require 会抛错；可选探测失败必须完整保留本地实现。
    let hostUi = null
    let hostButton = null
    let hostInput = null
    let hostSwitch = null
    let hostSegmentedTabs = null
    let hostSegmentedControl = null
    let hostTag = null
    let hostStateDot = null
    let hostPill = null
    let hostCollapsibleRow = null
    let hostTooltip = null
    let hostOrbit = null
    let hostOrbitRow = null
    try {
      const primitives = require('@deepseek-ai/dsh-client-ui-primitives')
      // Button 独立探测：老宿主缺 Modal/Toast 时仍可复用官方按钮；缺 Button 时所有 variant 完整回退旧 Btn。
      hostButton = primitives.Button || null
      // Input 同样独立探测：缺导出只回退共享输入框，不影响 Button/Tabs 等其它原语。
      hostInput = primitives.Input || null
      // Switch 同样独立探测：缺导出只回退共享 Toggle，不影响其它原语。
      hostSwitch = primitives.Switch || null
      // SegmentedTabs 同样独立探测：缺导出时完整保留下方 SegTabs，不影响其它原语。
      hostSegmentedTabs = primitives.SegmentedTabs || null
      // SegmentedControl 与 SegmentedTabs 是两个导出：缺导出时回退本地单选分段（方向键/Home/End 同语义）。
      hostSegmentedControl = primitives.SegmentedControl || null
      // Tag 与 StateDot 各自独立探测：任何一个缺导出都只回退自己，不影响另一个或其它原语。
      hostTag = primitives.Tag || null
      hostStateDot = primitives.StateDot || null
      // Pill 同样独立探测：缺导出时完整保留下方本地胶囊，不影响其它原语。
      hostPill = primitives.Pill || null
      // 仅供 CollapsibleList：WatcherForm 高级区需要 bodyId/aria-controls，必须继续走本地 DisclosureRow。
      hostCollapsibleRow = primitives.DisclosureRow || null
      if (primitives && primitives.Modal && primitives.Toast && primitives.IconWarningOutlineMedium) {
        hostUi = createHostUi({ React, primitives })
        // Tooltip 独立探测：缺少它只回退 InfoTip，不把已接入的 Modal/Toast 一起降级。
        if (primitives.Tooltip && typeof React.forwardRef === 'function') hostTooltip = hostUi.Tooltip
      }
    } catch {
      hostUi = null
      hostButton = null
      hostInput = null
      hostSwitch = null
      hostSegmentedTabs = null
      hostSegmentedControl = null
      hostTag = null
      hostStateDot = null
      hostPill = null
      hostCollapsibleRow = null
      hostTooltip = null
    }

    // Orbit 独立探测：容器用官方 MenuSurface，watcher 行用官方 DisclosureRow。
    // 没有通用 Popover/Card/List 导出；DockLayout/FloatLayer 是带分栏/拖拽动作的标签坞，不冒充本弹层。
    // 缺任一实际用到的能力（含 react-dom.createPortal）都完整保留旧 StatusOrbit，不做半套原生结构。
    try {
      const primitives = require('@deepseek-ai/dsh-client-ui-primitives')
      if (primitives && primitives.MenuSurface && primitives.DisclosureRow
        && typeof primitives.useAnchoredPosition === 'function'
        && typeof primitives.useDismissOnOutsidePointer === 'function') {
        const reactDom = require('react-dom')
        if (reactDom && typeof reactDom.createPortal === 'function') {
          hostOrbit = createOrbitPopover({ React: { ...React, createPortal: reactDom.createPortal }, primitives })
          hostOrbitRow = primitives.DisclosureRow
        }
      }
    } catch {
      hostOrbit = null
      hostOrbitRow = null
    }

    // 客户端版本探针：每次模块加载写一次，用来确认浏览器里跑的是哪一版
    const CLIENT_VERSION = '0.10.12'
    // SEMVER_COMPARATOR_SOURCE_START
    // This block is duplicated verbatim in client.js because the browser UMD cannot import ESM.
    // test/update-semver.test.mjs audits both copies and exercises their behavior.
    const SEMVER_RE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/

    function parseSemver(value) {
      if (typeof value !== 'string') return null
      const match = SEMVER_RE.exec(value)
      if (!match) return null
      return {
        core: [match[1], match[2], match[3]],
        prerelease: match[4] === undefined ? null : match[4].split('.'),
      }
    }

    // Numeric identifiers are compared as decimal strings: Number/BigInt would either
    // lose precision or impose a fixed-width limit that SemVer itself does not have.
    function compareNumericIdentifiers(left, right) {
      if (left.length !== right.length) return left.length < right.length ? -1 : 1
      if (left === right) return 0
      return left < right ? -1 : 1
    }

    function comparePrerelease(left, right) {
      if (left === null || right === null) {
        if (left === right) return 0
        return left === null ? 1 : -1
      }
      const length = Math.max(left.length, right.length)
      for (let index = 0; index < length; index += 1) {
        if (index >= left.length) return -1
        if (index >= right.length) return 1
        const a = left[index]
        const b = right[index]
        if (a === b) continue
        const aNumeric = /^\d+$/.test(a)
        const bNumeric = /^\d+$/.test(b)
        if (aNumeric && bNumeric) return compareNumericIdentifiers(a, b)
        if (aNumeric !== bNumeric) return aNumeric ? -1 : 1
        return a < b ? -1 : 1
      }
      return 0
    }

    /** Return -1/0/1 by SemVer precedence, or null when either input is invalid. */
    function compareSemver(left, right) {
      const a = parseSemver(left)
      const b = parseSemver(right)
      if (!a || !b) return null
      for (let index = 0; index < 3; index += 1) {
        const compared = compareNumericIdentifiers(a.core[index], b.core[index])
        if (compared !== 0) return compared
      }
      return comparePrerelease(a.prerelease, b.prerelease)
    }

    function isStrictSemver(value) {
      return parseSemver(value) !== null
    }
    // SEMVER_COMPARATOR_SOURCE_END
    const UPDATE_UNKNOWN_HINT = '请求结果未知，请先重新检查 installed/running，再手动确认'
    let BASE = '/session-superagent'
    /**
     * 挑「最新的那个服务端实例」。
     *
     * 宿主重载时旧 handler 摘不掉，所以可能同时存在多个前缀：默认的（旧代码）和 `/rN`（新代码）。
     * 甚至出现过「新实例在默认前缀注册成功、但请求仍被旧 handler 接走」的情况 ——
     * 所以不能只看「注册成功」，要按实例自报的版本 + routeBase 匹配来选。
     */
    async function discoverBase() {
      const candidates = [BASE]
      for (let i = 2; i <= 8; i += 1) candidates.push('/session-superagent/r' + i)
      let best = null
      for (const base of candidates) {
        try {
          const res = await fetch(base + '/state', { headers: { accept: 'application/json' } })
          if (!res.ok) continue
          const body = await res.json()
          // 只有「自报前缀 = 我请求的前缀」才说明这个实例真的在这条路径上服务
          if (!body || body.routeBase !== base) continue
          if (!isStrictSemver(body.serverVersion)) continue
          if (!best || compareSemver(body.serverVersion, best.version) === 1) best = { base, version: body.serverVersion }
        } catch {}
      }
      if (best && best.base !== BASE) {
        BASE = best.base
        // 记一笔：排查「面板到底在跟哪一代」时用得上（不像轮询那样完全不落痕迹）
        try { note('client-base', BASE) } catch {}
      } else if (best) {
        BASE = best.base
      }
      return BASE
    }

    const POLL_MS = 5000

    // 上一次 /state 的 ETag：带上它，没变化时服务端回 304，省掉整份 JSON。
    // 必须按作用域分开存（终审 N3）：状态栏圆环拉的是全局 /state，主面板拉的是 /state?sessionId=…；
    // 服务端 etag 对整份 body 求值，两个作用域 body 不同 —— 共用一个 etag 会让面板的条件请求永远打不中 304。
    // 不带 sessionId 时仍然只有一个键，与旧行为一致。
    const stateEtags = new Map()
    const readStateEtag = (path) => (typeof path === 'string' && path.startsWith('/state')
      ? (stateEtags.get(path) || null)
      : null)
    const writeStateEtag = (path, etag) => {
      if (typeof path !== 'string' || !path.startsWith('/state')) return
      if (etag) stateEtags.set(path, etag)
      else stateEtags.delete(path)
    }

    async function call(path, options, retried) {
      const init = {
        ...(options || {}),
        headers: { 'content-type': 'application/json', accept: 'application/json', ...((options && options.headers) || {}) },
      }
      // 更新可能已经开始安装；响应丢失不代表请求未执行，自动重放会重复修改安装副本。
      // 特例只覆盖更新 POST：GET 与其它 mutation 的 discover/retry 行为保持原样。
      const updatePost = path === '/update' && String(init.method || 'GET').toUpperCase() === 'POST'
      const unknownUpdateError = (error) => {
        const failure = error && (typeof error === 'object' || typeof error === 'function')
          ? error
          : new Error(String(error))
        failure.updateResultUnknown = true
        return failure
      }
      let res
      try {
        res = await fetch(BASE + path, init)
      } catch (error) {
        if (updatePost) throw unknownUpdateError(error)
        // 前缀失效（这一代被新实例接管并释放了）：重新探一次再试，还不行就把错抛出去
        if (retried) throw error
        await discoverBase()
        return call(path, options, true)
      }
      // 旧前缀被释放后宿主会 404：只读请求可重探一次；更新 POST 绝不自动重放。
      if (res.status === 404 && !retried && !updatePost) {
        await discoverBase()
        return call(path, options, true)
      }
      // 304：内容没变（服务端带 ETag）。不是错误，也没有正文。
      if (res.status === 304) {
        const etag = res.headers && res.headers.get ? res.headers.get('etag') : null
        if (etag) writeStateEtag(path, etag)
        return { notModified: true }
      }
      const freshEtag = res.headers && res.headers.get ? res.headers.get('etag') : null
      if (freshEtag && path.startsWith('/state')) writeStateEtag(path, freshEtag)
      const text = await res.text()
      let body = null
      try { body = text ? JSON.parse(text) : null } catch { body = { raw: text } }
      if (!res.ok) {
        const error = new Error((body && body.error) || ('HTTP ' + res.status))
        if (updatePost && res.status === 404) throw unknownUpdateError(error)
        throw error
      }
      return body
    }

    const S = {
      wrap: { padding: '16px 20px', font: '13px/1.5 system-ui, sans-serif', height: '100%', overflow: 'auto' },
      title: { fontSize: 15, fontWeight: 600, margin: '0 0 4px' },
      sub: { opacity: 0.65, margin: '0 0 14px' },
      row: { display: 'grid', gridTemplateColumns: '1.4fr 1fr 70px 1.2fr 1.2fr 70px 112px', gap: 10, padding: '7px 0', borderBottom: '1px solid rgba(128,128,128,.18)' },
      head: { fontWeight: 600, opacity: 0.7, borderBottom: '1px solid rgba(128,128,128,.35)' },
      mono: { fontFamily: 'ui-monospace, Consolas, monospace', fontSize: 12 },
      badgeOn: { color: 'var(--dsw-alias-state-success-primary)' },
      badgeOff: { color: 'var(--dsw-alias-state-warn-primary)' },
      err: { color: 'var(--dsw-alias-state-error-primary)', fontSize: 12 },
      empty: { opacity: 0.6, marginTop: 12 },
      foot: { marginTop: 14, opacity: 0.55, fontSize: 12 },
      card: { border: '1px solid rgba(128,128,128,.25)', borderRadius: 8, padding: '12px 14px', marginBottom: 14 },
      cardTitle: { fontWeight: 600, margin: '0 0 8px' },
      field: { display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8, flexWrap: 'wrap' },
      label: { minWidth: 72, opacity: 0.75 },
      input: { flex: 1, minWidth: 180, padding: '5px 8px', borderRadius: 6, border: '1px solid rgba(128,128,128,.4)', background: 'transparent', color: 'inherit', font: 'inherit' },
      button: { padding: '5px 12px', borderRadius: 6, border: '1px solid rgba(128,128,128,.5)', background: 'rgba(128,128,128,.12)', color: 'inherit', cursor: 'pointer' },
      tiny: { padding: '2px 8px', fontSize: 11, lineHeight: '16px', borderRadius: 5, border: '1px solid rgba(128,128,128,.45)', background: 'transparent', color: 'inherit', cursor: 'pointer' },
      ok: { color: 'var(--dsw-alias-state-success-primary)', fontSize: 12 },
    }

    // ── 移植自官方 dsh-superagent 的设计系统（scripts/port-official-ui.mjs 生成）──
    // 官方 GUI 的 376 条选择器 / 40KB 样式原样搬过来，类名统一 `dt-` 前缀，不污染宿主。
    // 两个表单都有这个开关：文案只写一份，改的时候不会漏掉另一处
    const ISOLATED_HINT = '隔离执行（每次唤醒开一个临时副本；在副本里改，用完自动回收）'
    const PORTED_CSS = "/* Panel layout and local UI controls styled with DSH `--dsw-alias-*` design tokens; one rule per line. */\n.dt-page { box-sizing: border-box; height: 100%; overflow: auto; scrollbar-gutter: stable; scroll-padding: 48px 0 72px; container-type: inline-size; padding: 28px clamp(24px, 4vw, 48px) 0; color: var(--dsw-alias-label-primary); }\n/* Deployment page rendered inside DSH Plugin Manager, which owns scrolling and padding. */\n.dt-embedded { container-type: inline-size; color: var(--dsw-alias-label-primary); }\n/* Disable sticky positioning inside DSH Plugin Manager, where the host container owns scrolling and bottom padding. */\n.dt-embedded .dt-savebar { position: static; }\n.dt-page > * + *, .dt-tabpanel > * + * { margin-top: 16px; }\n.dt-page > .dt-column { max-width: 960px; margin: 0 auto; padding-bottom: 48px; display: flex; flex-direction: column; gap: 24px; }\n.dt-head { position: relative; z-index: 3; display: flex; align-items: flex-start; justify-content: space-between; gap: 8px 16px; flex-wrap: wrap; }\n.dt-search { position: relative; width: 240px; max-width: 100%; }\n.dt-search-results { position: absolute; top: calc(100% + 6px); right: 0; width: min(560px, 90cqw); max-height: 60vh; overflow: auto; padding: 8px 16px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 12px; background: var(--dsw-alias-bg-layer-2); box-shadow: var(--dsw-elevation-prominent); z-index: 5; display: flex; flex-direction: column; gap: 8px; }\n.dt-mark { background: color-mix(in srgb, var(--dsw-alias-state-warn-primary) 25%, transparent); color: inherit; border-radius: 2px; }\n[data-platform='darwin'] .dt-page .dt-head { padding-top: var(--dsh-frame-top-clearance, 0px); }\n.dt-title { margin: 0; font-size: 15px; line-height: 22px; font-weight: 600; color: var(--dsw-alias-label-secondary); }\n.dt-toolbar { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }\n.dt-page .dt-tabs { overflow-x: auto; display: flex; gap: 20px; max-width: none; padding: 0; border-radius: 0; background: none; border-bottom: 0.5px solid var(--dsw-alias-border-l3); }\n.dt-segmented { width: 100%; max-width: 420px; box-sizing: border-box; display: inline-grid; gap: 2px; padding: 3px; border-radius: 9px; background: var(--dsw-alias-interactive-bg-hover); }\n.dt-ticket-body .dt-form { position: relative; z-index: 1; }\n.dt-form.dt-block { padding: 16px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 16px; gap: 16px; }\n.dt-block-title { display: flex; align-items: center; gap: 4px; margin: 0; font-size: 16px; line-height: 24px; font-weight: 500; }\n.dt-section { display: flex; flex-direction: column; gap: 8px; min-width: 0; }\n.dt-section-title { margin: 0; display: flex; flex-wrap: wrap; align-items: center; gap: 8px; font-size: 14px; line-height: 22px; font-weight: 600; color: var(--dsw-alias-label-primary); }\n.dt-section-name { white-space: nowrap; }\n.dt-count { font-size: 14px; color: var(--dsw-alias-label-caption); font-variant-numeric: tabular-nums; font-weight: 400; } .dt-section-sub { font-size: 12px; line-height: 18px; font-weight: 400; color: var(--dsw-alias-label-caption); }\n.dt-hint { margin: 0; font-size: 13px; line-height: 20px; color: var(--dsw-alias-label-tertiary); }\n.dt-error { margin: 0; font-size: 13px; line-height: 20px; color: var(--dsw-alias-state-error-primary); }\n.dt-banner { margin: 0; padding: 8px 12px; border-radius: 10px; font-size: 13px; line-height: 20px; background: color-mix(in srgb, var(--dsw-alias-state-warn-primary) 10%, transparent); overflow-wrap: anywhere; }\n.dt-banner.dt-danger { background: color-mix(in srgb, var(--dsw-alias-state-error-primary) 10%, transparent); color: var(--dsw-alias-state-error-primary); }\n.dt-list { list-style: none; margin: 0 -8px; padding: 0; display: flex; flex-direction: column; gap: 2px; }\n.dt-row { position: relative; min-width: 0; border-radius: 12px; padding: 10px 8px; display: flex; flex-wrap: wrap; gap: 4px 12px; align-items: flex-start; }\n.dt-row.dt-link:hover { background: var(--dsw-alias-interactive-bg-hover); }\n.dt-row-main { flex: 1 1 200px; min-width: 0; display: flex; flex-direction: column; gap: 4px; }\n.dt-disclosure-body .dt-row-main { flex-basis: 0; }\n.dt-row-title { display: flex; align-items: center; gap: 8px; min-width: 0; font-size: 14px; line-height: 20px; font-weight: 500; }\n.dt-open { max-width: 100%; padding: 0; border: 0; background: transparent; color: inherit; font: inherit; font-size: 14px; line-height: 20px; font-weight: 500; text-align: left; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }\n.dt-open::after { content: ''; position: absolute; inset: 0; border-radius: 12px; }\n.dt-open:focus-visible { outline: none; }\n.dt-open-wrap { white-space: normal; }\n.dt-open:focus-visible::after { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 2px; }\n.dt-row-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 4px 10px; font-size: 12px; line-height: 18px; color: var(--dsw-alias-label-caption); overflow-wrap: anywhere; }\n.dt-row-quote { font-size: 13px; line-height: 20px; color: var(--dsw-alias-label-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }\n.dt-clamp { display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }\n/* Transparent block borders preserve vertical spacing when line clamping clips padding. */\n.dt-banner:is(.dt-clamp, .dt-clamp-8) { padding-block: 0; border-block: 8px solid transparent; }\n.dt-banner-label { font-weight: 500; }\n.dt-banner .dt-md { display: inline; }\n.dt-banner .dt-md > p:first-child { display: inline; }\n.dt-row-buttons { position: relative; z-index: 1; gap: 8px; margin-top: 4px; }\n.dt-row-actions { position: relative; z-index: 1; display: flex; align-items: center; gap: 4px; flex: none; margin-left: auto; font-size: 13px; line-height: 20px; }\n.dt-dot { flex: none; margin-top: 5px; }\n.dt-inline-link { position: relative; z-index: 1; color: var(--dsw-alias-link); text-decoration: none; }\n.dt-inline-link:hover { text-decoration: underline; }\n.dt-textlink { padding: 0; border: 0; background: none; font: inherit; color: var(--dsw-alias-link); cursor: pointer; text-align: left; }\n.dt-textlink:hover { text-decoration: underline; }\n.dt-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 13px; line-height: 20px; }\n.dt-kv { display: grid; grid-template-columns: max-content 1fr; gap: 6px 16px; margin: 0; font-size: 13px; line-height: 20px; }\n.dt-kv dt { color: var(--dsw-alias-label-tertiary); }\n.dt-kv dd { margin: 0; min-width: 0; overflow-wrap: anywhere; }\n@container (max-width: 640px) { .dt-kv dt { margin-top: 4px; } .dt-kv dt:first-of-type { margin-top: 0; } }\n.dt-timeline-day { position: sticky; top: 0; z-index: 1; padding: 6px 0 2px; font-size: 12px; line-height: 18px; color: var(--dsw-alias-label-secondary); font-weight: 500; list-style: none; background: var(--dsw-alias-bg-base); }\n.dt-event-head { display: flex; align-items: baseline; gap: 8px; min-width: 0; }\n.dt-event-head .dt-event-time { margin-left: auto; flex: none; font-size: 12px; color: var(--dsw-alias-label-secondary); font-variant-numeric: tabular-nums; }\n.dt-event-nums { color: var(--dsw-alias-label-secondary); font-variant-numeric: tabular-nums; }\n.dt-timeline { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }\n.dt-event { padding: 12px 0; border-bottom: 0.5px solid var(--dsw-alias-border-l2); display: flex; flex-direction: column; gap: 6px; min-width: 0; }\n.dt-event:last-child { border-bottom: 0; }\n.dt-event.dt-highlight { background: var(--dsw-alias-interactive-bg-hover); border-radius: 8px; padding-inline: 8px; margin-inline: -8px; }\n.dt-event-text { margin: 0; font-size: 13px; line-height: 20px; white-space: pre-wrap; overflow-wrap: anywhere; }\n.dt-event-actor { font-weight: 500; color: var(--dsw-alias-label-secondary); }\n.dt-event-quiet .dt-event-text { color: var(--dsw-alias-label-tertiary); }\n.dt-flow { margin: 0; font-size: 13px; line-height: 20px; color: var(--dsw-alias-label-secondary); }\n.dt-clamp-8 { display: -webkit-box; -webkit-line-clamp: 8; -webkit-box-orient: vertical; overflow: hidden; }\n.dt-more { align-self: flex-start; font-size: 12px; }\n.dt-reveal { opacity: 0; }\n.dt-event:hover .dt-reveal, .dt-reveal:focus-visible { opacity: 1; }\n@media (hover: none) {\n  .dt-reveal { opacity: 1; }\n}\n.dt-event-foot { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 12px; line-height: 18px; color: var(--dsw-alias-label-tertiary); }\n.dt-filters { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }\n.dt-filters .dt-search { margin-left: auto; }\n.dt-form { display: flex; flex-direction: column; gap: 24px; }\n.dt-fieldset { display: flex; flex-direction: column; gap: inherit; min-width: 0; margin: 0; padding: 0; border: 0; }\n.dt-field { display: flex; flex-direction: column; gap: 6px; font-size: 13px; line-height: 20px; min-width: 0; }\n.dt-field small { color: var(--dsw-alias-label-caption); font-size: 12px; }\n.dt-textarea { box-sizing: border-box; width: 100%; min-width: 0; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 10px; padding: 8px 12px; font: inherit; font-size: 13px; line-height: 20px; color: var(--dsw-alias-label-primary); background: var(--dsw-alias-bg-layer-1, transparent); resize: vertical; }\n.dt-select { box-sizing: border-box; width: 100%; min-width: 0; height: 32px; border: 0.5px solid var(--dsw-alias-border-l4); border-radius: 8px; padding: 0 28px 0 8px; text-overflow: ellipsis; font: inherit; font-size: 14px; line-height: 22px; color: var(--dsw-alias-label-primary); background: var(--dsw-alias-bg-layer-1, transparent); }\n.dt-textarea:focus-visible, .dt-select:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 1px; }\n.dt-mono { font-family: var(--ds-font-family-code, Menlo, monospace); font-size: 12px; overflow-wrap: anywhere; }\n.dt-detail { margin-top: 2px; color: var(--dsw-alias-label-secondary); }\n.dt-savebar .dt-error { white-space: pre-line; }\n.dt-switch-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 36px; font-size: 13px; }\n.dt-subhead-actions { display: inline-flex; align-items: center; gap: 4px; }\n.dt-btn-busy .dt-btn-icon svg { animation: dt-spin 1s linear infinite; }\n.dt-subhead { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; font-size: 13px; line-height: 20px; color: var(--dsw-alias-label-secondary); }\n.dt-rules, .dt-changes { list-style: none; margin: 0; padding: 0; }\n.dt-field-head { display: inline-flex; align-items: center; gap: 4px; color: var(--dsw-alias-label-secondary); }\n.dt-info { display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px; padding: 0; border: 0; background: none; color: var(--dsw-alias-label-caption); cursor: help; }\n.dt-info:hover, .dt-info:focus-visible { color: var(--dsw-alias-label-secondary); }\n.dt-info:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 2px; border-radius: 50%; }\n.dt-tooltip { position: fixed; z-index: 1100; width: max-content; max-width: min(320px, calc(100vw - 24px)); padding: 3px 7px; border-radius: 8px; background: var(--dsw-alias-tooltip-bg); color: var(--dsw-static-neutral-bluish-00); font-size: 13px; line-height: 20px; overflow-wrap: break-word; pointer-events: none; animation: dt-tooltip-in 150ms ease-in-out; }\n@keyframes dt-tooltip-in { from { opacity: 0; } }\n.dt-rules { border-top: 0.5px solid var(--dsw-alias-border-l2); }\n.dt-rule { display: flex; align-items: flex-start; gap: 12px; padding: 6px 0; border-bottom: 0.5px solid var(--dsw-alias-border-l2); }\n.dt-rule > :first-child { flex: 1; min-width: 0; }\n.dt-rule-preview { flex: 1; min-width: 0; margin-left: 8px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; color: var(--dsw-alias-label-caption); }\n.dt-rule-switch { margin-top: 2px; flex: none; }\n.dt-rule-dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: var(--dsw-alias-label-caption); }\n.dt-rule-dot[data-enabled] { background: var(--dsw-alias-brand-primary); }\n.dt-rule-body { display: flex; flex-direction: column; gap: 12px; padding: 8px 0 8px 28px; }\n.dt-model { display: flex; flex-direction: column; gap: 6px; }\n.dt-grid3 { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }\n.dt-changes { display: flex; flex-direction: column; gap: 2px; font-size: 13px; line-height: 20px; }\n.dt-changes li::before { content: '· '; color: var(--dsw-alias-label-caption); }\n.dt-main { display: flex; flex-direction: column; gap: 24px; min-width: 0; }\n.dt-modal.dt-glossary-modal { width: min(680px, 100%); }\n.dt-nav-item { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; min-height: 32px; padding: 6px 10px; border: 0; border-radius: 8px; background: transparent; color: var(--dsw-alias-label-secondary); font: inherit; font-size: 13px; line-height: 20px; text-align: left; cursor: pointer; }\n.dt-nav-item:hover { background: var(--dsw-alias-interactive-bg-hover); }\n.dt-nav-item[aria-current='true'] { background: var(--dsw-alias-interactive-bg-hover); color: var(--dsw-alias-label-primary); font-weight: 500; }\n.dt-nav-item:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: -2px; }\n.dt-changed-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--dsw-alias-brand-primary); flex: none; }\n.dt-add-skill { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; padding-top: 8px; border-top: 0.5px solid var(--dsw-alias-border-l2); }\n.dt-prompt-main { display: flex; flex-direction: column; gap: 12px; min-width: 0; }\n.dt-editor { min-height: 360px; }\n.dt-inline-select { width: auto; }\n.dt-preview-part { display: flex; flex-direction: column; gap: 6px; padding: 2px 0 2px 12px; border-left: 2px solid var(--dsw-alias-border-l3); }\n.dt-preview-head { font-size: 13px; line-height: 20px; display: flex; align-items: baseline; gap: 8px; }\ndiv.dt-preview-head { color: var(--dsw-alias-label-secondary); }\n.dt-card { display: flex; flex-direction: column; gap: 12px; padding: 16px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 16px; min-width: 0; }\n.dt-card-head { display: flex; align-items: center; gap: 12px; min-height: 28px; }\n.dt-card-head .dt-row-main { flex: 1 1 0; min-width: 64px; }\n.dt-card-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }\n.dt-row-label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; font-weight: 500; }\n.dt-rows { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }\n.dt-rows-item { display: grid; grid-template-columns: repeat(var(--dt-columns), minmax(0, 1fr)) auto; gap: 8px; align-items: center; }\n.dt-filter-gap { flex: 1; }\n.dt-page-head { display: flex; flex-wrap: wrap; align-items: flex-start; justify-content: space-between; gap: 8px 16px; width: 100%; }\n.dt-page-head > div:first-child { display: flex; flex-direction: column; gap: 4px; min-width: 0; }\n.dt-page-head .dt-toolbar { flex: 0 1 auto; margin-left: auto; }\n.dt-page-head > .dt-btn { margin-left: -10px; }\n.dt-page-title { display: flex; align-items: center; gap: 8px 16px; flex-wrap: wrap; width: 100%; }\n.dt-page-title h2 { margin: 0; flex: 1 1 auto; min-width: 0; font-size: 20px; line-height: 28px; font-weight: 500; overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }\n.dt-status-line { margin-top: -16px; }\n.dt-plain-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; font-size: 13px; line-height: 20px; }\n.dt-plain-list > li { position: relative; padding-left: 12px; }\n.dt-plain-list > li::before { content: ''; position: absolute; left: 0; top: 5px; bottom: 5px; width: 2px; border-radius: 1px; background: currentColor; opacity: 0.45; }\n.dt-details { display: flex; flex-direction: column; gap: 2px; border-top: 0.5px solid var(--dsw-alias-border-l2); padding-top: 8px; }\n.dt-danger-zone { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; padding-top: 16px; border-top: 0.5px solid var(--dsw-alias-border-l2); }\n.dt-danger-zone .dt-btn { color: var(--dsw-alias-state-error-primary); }\n.dt-md { display: flex; flex-direction: column; gap: 6px; font-size: 13px; line-height: 20px; overflow-wrap: anywhere; }\n.dt-md p, .dt-md ul, .dt-md ol { margin: 0; }\n.dt-md ul, .dt-md ol { padding-left: 20px; display: flex; flex-direction: column; gap: 2px; }\n.dt-md-heading { font-weight: 500; }\n.dt-code { font-family: var(--ds-font-family-code, Menlo, monospace); font-size: 12px; padding: 0 4px; border-radius: 4px; background: var(--dsw-alias-bg-module-platform); }\n.dt-dot-wrap { display: inline-flex; flex: none; }\n.dt-search-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }\n.dt-ellipsis { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 280px; }\n.dt-legend-title { margin: 16px 0 6px; font-size: 13px; color: var(--dsw-alias-label-tertiary); }\n.dt-legend { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px 16px; font-size: 13px; }\n.dt-legend li { display: flex; align-items: center; gap: 6px; }\n.dt-legend .dt-dot { margin-top: 0; }\n.dt-savebar { position: sticky; bottom: 0; z-index: 2; display: flex; align-items: center; justify-content: flex-end; gap: 8px; flex-wrap: wrap; padding: 12px 0; background: var(--dsw-alias-bg-base, var(--dsw-alias-bg-layer-1)); border-top: 0.5px solid var(--dsw-alias-border-l2); }\n.dt-savebar .dt-hint { margin-right: auto; }\n.dt-card > .dt-savebar, .dt-block > .dt-savebar { margin: 0 -16px -16px; padding: 12px 16px; border-radius: 0 0 16px 16px; }\n.dt-glossary { width: 100%; border-collapse: collapse; font-size: 13px; line-height: 20px; }\n.dt-glossary th, .dt-glossary td { text-align: left; padding: 6px 8px; border-bottom: 0.5px solid var(--dsw-alias-border-l2); vertical-align: top; }\n.dt-glossary th { color: var(--dsw-alias-label-tertiary); font-weight: 400; }\n.dt-glossary th.dt-glossary-group { color: var(--dsw-alias-label-secondary); font-weight: 600; padding-top: 14px; }\n.dt-glossary th:nth-child(-n+2), .dt-glossary td:nth-child(-n+2) { white-space: nowrap; }\n.dt-values { display: grid; grid-template-columns: max-content 1fr; gap: 6px 16px; margin: 16px 0 0; font-size: 13px; }\n.dt-values dt { color: var(--dsw-alias-label-tertiary); }\n.dt-values dd { margin: 0; }\n.dt-pre { margin: 0; padding: 12px; border-radius: 10px; background: var(--dsw-alias-bg-module-platform); font-family: var(--ds-font-family-code, Menlo, monospace); font-size: 12px; line-height: 18px; white-space: pre-wrap; overflow-wrap: anywhere; max-height: 480px; overflow: auto; }\n.dt-pre ins { background: color-mix(in srgb, var(--dsw-alias-state-success-primary, green) 18%, transparent); text-decoration: none; }\n.dt-pre del { background: color-mix(in srgb, var(--dsw-alias-state-error-primary) 18%, transparent); text-decoration: none; }\n.dt-prompt { display: grid; grid-template-columns: 200px minmax(0, 1fr); gap: 20px; align-items: start; }\n.dt-prompt-nav { display: flex; flex-direction: column; gap: 2px; }\n.dt-prompt-nav button { justify-content: flex-start; }\n.dt-disclosure-body { padding: 8px 0 8px 28px; display: flex; flex-direction: column; gap: 8px; }\n@container (max-width: 640px) {\n  .dt-tab { padding: 0 8px; font-size: 13px; }\n  .dt-segment { padding: 0 10px; }\n  .dt-head { align-items: center; }\n  .dt-search-results { right: auto; left: 0; width: 100%; }\n  .dt-page-title h2 { font-size: 17px; line-height: 24px; }\n  .dt-status-line { margin-top: -12px; }\n  .dt-page .dt-tabs { display: flex; flex-wrap: nowrap; scrollbar-width: thin; }\n  .dt-page .dt-tab { flex: none; }\n  .dt-page .dt-tabs[data-more='end'] { mask-image: linear-gradient(to right, #000 calc(100% - 40px), transparent); }\n  .dt-page .dt-tabs[data-more='start'] { mask-image: linear-gradient(to right, transparent, #000 40px); }\n  .dt-page .dt-tabs[data-more='both'] { mask-image: linear-gradient(to right, transparent, #000 40px, #000 calc(100% - 40px), transparent); }\n  .dt-kv { grid-template-columns: 1fr; gap: 6px; }\n  .dt-grid3, .dt-prompt { grid-template-columns: 1fr; }\n  .dt-search { width: 100%; }\n  .dt-filters .dt-search { margin-left: 0; }\n}\n/* Local UI primitive styles (see controls.tsx). */\n.dt-btn { display: inline-flex; align-items: center; justify-content: center; gap: 4px; border: none; border-radius: 18px; cursor: pointer; font: inherit; font-size: 14px; line-height: 22px; color: var(--dsw-alias-label-primary); background: transparent; padding: 0 14px; white-space: nowrap; }\n.dt-btn:disabled { cursor: not-allowed; opacity: 0.4; }\n.dt-btn:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 2px; }\n.dt-btn-md { height: 36px; }\n.dt-btn-sm { height: 28px; font-size: 12px; line-height: 18px; padding: 0 10px; border-radius: 14px; }\n/* 主按钮：深色下 label-primary-foreground（深色）压绿底只有 4.09 —— 深色主题里前景该是浅的。 */\n/* 往白色混 70%：浅色下仍是浅字（4.5+），深色下变浅（>4.5），两边都过。 */\n.dt-btn-primary { background: var(--dsw-alias-button-primary-fill); color: var(--dsw-static-neutral-bluish-00); }\n.dt-btn-primary:hover:not(:disabled) { background: var(--dsw-alias-button-primary-hover); }\n.dt-btn-ghost { background: transparent; border: 0.5px solid var(--dsw-alias-border-l3); color: var(--dsw-alias-label-secondary); }\n.dt-btn-ghost:hover:not(:disabled), .dt-btn-outline:hover:not(:disabled) { background: var(--dsw-alias-interactive-bg-hover); }\n.dt-btn-ghost:active:not(:disabled) { background: var(--dsw-alias-interactive-bg-active); }\n.dt-btn-outline { border: 0.5px solid var(--dsw-alias-border-l3); }\n.dt-btn-icon { display: inline-flex; width: 16px; height: 16px; align-items: center; justify-content: center; }\n.dt-tag { display: inline-flex; align-items: center; border-radius: 999px; padding: 1px 8px; font-size: 11px; line-height: 17px; font-weight: 500; white-space: nowrap; }\n.dt-tag[data-tone='outline'] { border: 0.5px solid var(--dsw-alias-border-l4); color: var(--dsw-alias-label-tertiary); }\n.dt-tag[data-tone='neutral'] { background: var(--dsw-alias-bg-module-platform); color: var(--dsw-alias-label-secondary); }\n.dt-tag[data-tone='quiet'] { color: var(--dsw-alias-label-secondary); }\n.dt-tag[data-tone='success'] { background: color-mix(in srgb, var(--dsw-alias-state-success-primary) 10%, transparent); color: color-mix(in srgb, var(--dsw-alias-state-success-primary) 72%, var(--dsw-alias-label-primary)); }\n.dt-tag[data-tone='info'] { background: color-mix(in srgb, var(--dsw-alias-state-business-primary) 10%, transparent); color: var(--dsw-alias-state-business-primary); }\n/* 胶囊是「暂停/退避/判断」唯一的状态色：文字与 12% 同色底叠起来，浅色下警告只有 4.17。 */\n/* 往 label-primary 混 28%：浅色变深、深色变浅，两档都留出余量。 */\n.dt-tag[data-tone='warning'] { background: color-mix(in srgb, var(--dsw-alias-state-warn-primary) 12%, transparent); color: color-mix(in srgb, var(--dsw-alias-state-warn-primary) 72%, var(--dsw-alias-label-primary)); }\n.dt-tag[data-tone='danger'] { background: color-mix(in srgb, var(--dsw-alias-state-error-primary) 10%, transparent); color: color-mix(in srgb, var(--dsw-alias-state-error-primary) 76%, var(--dsw-alias-label-primary)); }\n.dt-pill { display: inline-flex; align-items: center; gap: 4px; height: 24px; padding: 0 8px; border: none; border-radius: 12px; font: inherit; font-size: 12px; line-height: 18px; color: var(--dsw-alias-label-secondary); background: var(--dsw-alias-bg-layer-2); cursor: pointer; }\n.dt-pill:hover { background: var(--dsw-alias-interactive-bg-hover); }\n.dt-pill-active, .dt-pill-active:hover { color: var(--dsw-alias-label-primary); background: var(--dsw-alias-button-ghost-active-fill); box-shadow: inset 0 0 0 1px var(--dsw-alias-button-ghost-active-border); }\n.dt-tabs { display: grid; padding: 4px; border-radius: 12px; background: var(--dsw-alias-bg-module-platform); }\n.dt-tab { box-sizing: border-box; height: 34px; padding: 0 12px; border: 0.5px solid transparent; border-radius: 8px; background: transparent; color: var(--dsw-alias-label-secondary); font: inherit; font-size: 14px; line-height: 20px; white-space: nowrap; cursor: pointer; transition: color 180ms ease, background 180ms ease; }\n.dt-tab:hover { color: var(--dsw-alias-label-primary); }\n.dt-tab[aria-selected='true'] { color: var(--dsw-alias-label-primary); font-weight: 600; background: var(--dsw-alias-bg-layer-3); border-color: var(--dsw-alias-border-l3); }\n.dt-tab:focus-visible { outline: none; text-decoration: underline; text-underline-offset: 4px; }\n.dt-segment { height: 28px; padding: 0 16px; border: 0; border-radius: 7px; background: transparent; color: var(--dsw-alias-label-secondary); font: inherit; font-size: 13px; line-height: 20px; font-weight: 500; white-space: nowrap; cursor: pointer; transition: color 120ms ease, background 160ms ease; }\n.dt-segment:hover:not(:disabled), .dt-segment[aria-selected='true'] { color: var(--dsw-alias-label-primary); }\n.dt-segment[aria-selected='true'] { background: var(--dsw-alias-bg-layer-1); box-shadow: var(--dsw-elevation-soft); }\n.dt-segment:disabled { cursor: default; opacity: 0.4; }\n.dt-segment:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: -2px; }\n.dt-input { box-sizing: border-box; display: inline-flex; align-items: center; gap: 6px; height: 32px; min-width: 0; padding: 0 8px; border: 0.5px solid var(--dsw-alias-border-l4); border-radius: 8px; background: var(--dsw-alias-bg-layer-1); }\n.dt-input:focus-within { border-color: var(--dsw-alias-brand-primary); }\n.dt-input-icon { display: inline-flex; width: 16px; height: 16px; align-items: center; justify-content: center; color: var(--dsw-alias-label-tertiary); }\n.dt-input input { flex: 1; min-width: 0; border: none; outline: none; background: transparent; font: inherit; font-size: 14px; line-height: 22px; color: var(--dsw-alias-label-primary); }\n.dt-input input/* 占位符：label-dimmed 在浅色下只有 2.17（深色 3.81），必须能读出来才叫占位符。 */\n/* 往 label-tertiary 混 60%：浅色变深、深色变浅，两档都过 4.5。 */\n::placeholder { color: color-mix(in srgb, var(--dsw-alias-label-dimmed) 40%, var(--dsw-alias-label-tertiary)); }\n.dt-input input::-webkit-search-cancel-button { -webkit-appearance: none; appearance: none; }\n.dt-field .dt-input, .dt-search .dt-input, .dt-add-skill .dt-input { display: flex; width: 100%; }\n.dt-switch { box-sizing: border-box; flex: 0 0 auto; width: 36px; height: 20px; padding: 2px; border: 0; border-radius: 10px; background: var(--dsw-alias-border-l3); cursor: pointer; }\n.dt-switch[aria-checked='true'] { background: var(--dsw-alias-brand-primary); }\n.dt-switch:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 2px; }\n.dt-switch-thumb { display: block; width: 16px; height: 16px; border-radius: 50%; background: var(--dsw-alias-label-primary-foreground); transition: transform 120ms ease; }\n.dt-switch[aria-checked='true'] .dt-switch-thumb { transform: translateX(16px); }\n.dt-state-dot { position: relative; display: inline-block; flex: none; width: 10px; height: 10px; color: var(--dsw-alias-state-idle-primary); }\n.dt-state-dot::after { content: ''; position: absolute; inset: 20%; border-radius: 50%; background: currentColor; }\n.dt-state-dot[data-state='done'] { color: var(--dsw-alias-state-success-primary); }\n.dt-state-dot[data-state='warning'] { color: var(--dsw-alias-state-warn-primary); }\n.dt-state-dot[data-state='error'] { color: var(--dsw-alias-state-error-primary); }\n.dt-spinner { flex: none; color: var(--dsw-alias-label-tertiary); }\n.dt-spinner g { transform-origin: center; animation: dt-spin 1.5s linear infinite; }\n.dt-spinner circle { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; }\n.dt-spinner circle:first-child { opacity: 0.25; }\n.dt-spinner circle:last-child { stroke-dasharray: 18 150; }\n@keyframes dt-spin { to { transform: rotate(360deg); } }\n.dt-disclosure-row { display: flex; align-items: center; gap: 4px; min-height: 28px; min-width: 0; border-radius: 8px; cursor: pointer; font-size: 13px; line-height: 20px; color: var(--dsw-alias-label-secondary); }\n.dt-disclosure-row:hover { color: var(--dsw-alias-label-primary); }\n.dt-disclosure-row:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 2px; }\n.dt-disclosure-lead { position: relative; display: inline-flex; flex: none; width: 24px; height: 24px; align-items: center; justify-content: center; }\n.dt-disclosure-icon { display: inline-flex; align-items: center; justify-content: center; }\n.dt-disclosure-chevron { position: absolute; inset: 0; display: none; align-items: center; justify-content: center; }\n.dt-disclosure-row:hover .dt-disclosure-icon, .dt-disclosure[data-open] .dt-disclosure-icon { visibility: hidden; }\n.dt-disclosure-row:hover .dt-disclosure-chevron, .dt-disclosure[data-open] .dt-disclosure-chevron { display: inline-flex; }\n.dt-disclosure-title { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }\n.dt-modal-root { position: fixed; inset: 0; z-index: 1000; display: flex; align-items: center; justify-content: center; padding: max(24px, var(--dsh-frame-top-clearance, 24px)) 24px; }\n.dt-modal-mask { position: absolute; inset: 0; background: var(--dsw-alias-bg-mask-1); backdrop-filter: var(--dsw-mask-blur); }\n.dt-modal { position: relative; z-index: 1; display: flex; flex-direction: column; gap: 20px; width: min(380px, 100%); max-height: 100%; padding: 0 0 24px; overflow: hidden; border-radius: 24px; background: var(--dsw-alias-bg-layer-2); box-shadow: var(--dsw-elevation-prominent); color: var(--dsw-alias-label-primary); outline: none; }\n.dt-modal-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 22px 14px 0 24px; }\n.dt-modal-header h2 { margin: 0; font-size: 16px; line-height: 24px; font-weight: 500; }\n.dt-modal-close { display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 28px; border: none; border-radius: 8px; background: transparent; cursor: pointer; color: var(--dsw-alias-label-secondary); }\n.dt-modal-close:hover { background: var(--dsw-alias-interactive-bg-hover); }\n.dt-modal-description { margin: -8px 0 0; padding: 0 24px; font-size: 14px; line-height: 22px; }\n.dt-modal-body { min-width: 0; min-height: 0; overflow: auto; padding: 0 24px; }\n.dt-modal-footer { display: flex; align-items: center; justify-content: flex-end; gap: 8px; padding: 0 24px; }\n.dt-toast { position: fixed; top: 40px; left: 50%; z-index: 1100; pointer-events: none; width: max-content; max-width: min(640px, calc(100vw - 48px)); padding: 12px 16px; border-radius: 14px; background: var(--dsw-alias-toast-bg); color: var(--dsw-static-neutral-bluish-00); font-size: 14px; line-height: 22px; box-shadow: var(--dsw-shadow-lv3); transform: translateX(-50%); animation: dt-toast-in 160ms ease-out, dt-toast-fade 1000ms ease var(--dt-toast-hold, 3000ms) forwards; }\n@keyframes dt-toast-in { from { opacity: 0; transform: translate(-50%, -6px); } to { opacity: 1; transform: translate(-50%, 0); } }\n@keyframes dt-toast-fade { to { opacity: 0; visibility: hidden; } }\n@media (prefers-reduced-motion: reduce) {\n  .dt-tab, .dt-segment, .dt-switch-thumb, .dt-spinner g, .dt-btn-busy .dt-btn-icon svg, .dt-tooltip { transition: none; animation: none; }\n  .dt-toast { animation: dt-toast-fade 1000ms ease var(--dt-toast-hold, 3000ms) forwards; }\n}\n.dt-page .dt-tab { height: 34px; padding: 0 2px; border: 0; border-radius: 0; color: var(--dsw-alias-label-tertiary); font-size: 14px; }\n.dt-page .dt-tab[aria-selected='true'] { background: none; color: var(--dsw-alias-label-primary); font-weight: 500; box-shadow: inset 0 -1.5px 0 var(--dsw-alias-label-primary); }\n.dt-status-head { display: flex; align-items: center; gap: 8px; }\n.dt-status-title { margin: 0; flex: 1; display: flex; align-items: center; gap: 4px; font-size: 15px; line-height: 22px; font-weight: 600; }\n.dt-status-text { margin-left: auto; flex: none; font-size: 12px; font-weight: 400; color: var(--dsw-alias-label-secondary); }\n.dt-status-line .dt-status-text { margin-left: 0; }\n.dt-status-text[data-state='warning'] { color: var(--dsw-alias-state-warn-primary); }\n.dt-status-text[data-state='error'] { color: var(--dsw-alias-state-error-primary); }\n.dt-row-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; }\n.dt-lv { white-space: nowrap; }\n.dt-lv-label { color: var(--dsw-alias-label-caption); }\n.dt-lv-value { color: var(--dsw-alias-label-secondary); font-variant-numeric: tabular-nums; }\n.dt-chip { display: inline-flex; flex-wrap: wrap; max-width: 100%; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 6px; overflow: hidden; font-size: 11px; line-height: 18px; color: var(--dsw-alias-label-secondary); }\n.dt-chip-source { padding: 0 6px; color: var(--dsw-alias-label-tertiary); background: var(--dsw-alias-bg-module-platform); }\n.dt-chip-fact { padding: 0 6px; border-left: 0.5px solid var(--dsw-alias-border-l3); white-space: nowrap; }\n.dt-chip-label { margin-right: 4px; color: var(--dsw-alias-label-caption); }\n.dt-chip-link { position: relative; z-index: 1; text-decoration: none; cursor: pointer; transition: border-color 120ms ease; }\n.dt-chip-link:hover { border-color: var(--dsw-alias-brand-primary); }\n.dt-chip-link:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 1px; }\n.dt-chip-open { display: inline-flex; align-items: center; padding: 0 5px; border-left: 0.5px solid var(--dsw-alias-border-l3); color: var(--dsw-alias-brand-primary); }\n.dt-chip-problem, .dt-chip-problem .dt-chip-label { color: var(--dsw-alias-state-error-primary); }\n.dt-cards { gap: 8px; }\n.dt-att .dt-inline-link { font-size: 13px; font-weight: 500; }\n.dt-list > .dt-att { margin: 2px 8px; }\n.dt-att { padding: 10px 12px 10px 16px; border: 0; border-radius: 0; border-bottom: 0.5px solid var(--dsw-alias-border-l3); }\n.dt-att::before { content: ''; position: absolute; left: 6px; top: 10px; bottom: 10px; width: 2px; border-radius: 1px; background: var(--dsw-alias-state-warn-primary); opacity: 0.7; }\n.dt-att-error::before { background: var(--dsw-alias-state-error-primary); }\n.dt-att-what { font-size: 12px; line-height: 18px; font-weight: 500; color: var(--dsw-alias-state-warn-primary); }\n.dt-att-error .dt-att-what, .dt-att-error .dt-att-what .dt-open { color: var(--dsw-alias-state-error-primary); }\n.dt-stats { display: flex; flex-wrap: wrap; gap: 6px 24px; padding: 10px 14px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 12px; background: var(--dsw-alias-bg-module-platform); }\n.dt-stat { display: inline-flex; align-items: baseline; gap: 6px; font-size: 12px; color: var(--dsw-alias-label-caption); }\n/* 可点的统计数字：去掉原生按钮外观，保留文字排版；选中时用浅底提示 */\n.dt-stat-btn { border: 0; background: none; padding: 2px 6px; margin: -2px -6px; border-radius: 8px; font: inherit; color: inherit; cursor: pointer; }\n.dt-stat-btn:hover { background: var(--dsw-alias-interactive-bg-hover); }\n/* 浅色下 interactive-bg-hover 与白底只差 1.03：点了统计数字在按筛选，零视觉反馈。 */\n/* 换成品牌色 12% 底 + 0.5px 品牌色描边（两个主题都分得开）。 */\n.dt-stat-btn[aria-pressed='true'] { background: color-mix(in srgb, var(--dsw-alias-brand-primary) 12%, transparent); box-shadow: inset 0 0 0 0.5px var(--dsw-alias-brand-primary); }\n.dt-stat-btn:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 1px; }\n/* 统计条单位：按钮上的 color:inherit 会把单位染成数字色，所以单位单独一档。 */\n/* 浅色下 caption（#818b98）在白底只有 3.14–3.45，深色下同一档 4.34–5.07。 */\n/* 往 label-primary 混 30%：浅色变深、深色变浅，两个主题的对比度同时变好（不用新 token）。 */\n/* 注意选择器**不带** .dt-stat-btn 前缀 —— 带前缀就只作用于可点的格子，不可点的仍是旧色（真机踩过）。 */\n.dt-stat-unit { color: color-mix(in srgb, var(--dsw-alias-label-caption) 70%, var(--dsw-alias-label-primary)); }\n/* 选中的统计格：品牌色 12% 底会把单位压到 4.20，再往主色多混 10% 拉回来。 */\n.dt-stat-btn[aria-pressed='true'] .dt-stat-unit { color: color-mix(in srgb, var(--dsw-alias-label-caption) 58%, var(--dsw-alias-label-primary)); }\n.dt-stat-value { font-size: 14px; font-weight: 500; color: var(--dsw-alias-label-primary); font-variant-numeric: tabular-nums; }\n.dt-more-link { font-size: 12px; font-weight: 400; }\n.dt-settings { display: grid; grid-template-columns: 140px minmax(0, 1fr); gap: 24px; align-items: start; }\n.dt-settings-nav { position: sticky; top: 0; display: flex; flex-direction: column; gap: 2px; }\n.dt-settings-link { padding: 6px 10px; border: 0; border-radius: 8px; background: none; color: var(--dsw-alias-label-secondary); font: inherit; font-size: 13px; line-height: 20px; text-align: left; cursor: pointer; }\n.dt-settings-link:hover { color: var(--dsw-alias-label-primary); }\n.dt-settings-link[aria-current='page'] { background: var(--dsw-alias-bg-module-platform); color: var(--dsw-alias-label-primary); font-weight: 500; }\n.dt-settings-link[data-stage] + .dt-settings-link:not([data-stage]) { margin-top: 10px; }\n.dt-settings-body { display: flex; flex-direction: column; gap: 16px; min-width: 0; padding: 16px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 16px; background: var(--dsw-alias-bg-layer-1); }\n.dt-settings-title { margin: 0; font-size: 15px; line-height: 22px; font-weight: 600; }\n.dt-settings-body .dt-section-title { font-size: 15px; line-height: 22px; }\n.dt-subtabs { display: flex; gap: 16px; border-bottom: 0.5px solid var(--dsw-alias-border-l3); }\n.dt-subtab { padding: 0 0 6px; border: 0; background: none; color: var(--dsw-alias-label-tertiary); font: inherit; font-size: 13px; cursor: pointer; }\n.dt-subtab[aria-selected='true'] { color: var(--dsw-alias-label-primary); font-weight: 500; box-shadow: inset 0 -1.5px 0 var(--dsw-alias-label-primary); }\n.dt-part { display: flex; flex-direction: column; gap: 12px; }\n.dt-rows-summary { text-align: left; color: var(--dsw-alias-label-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; }\n.dt-table { width: 100%; border-collapse: collapse; font-size: 12px; line-height: 18px; }\n.dt-table th { text-align: left; font-weight: 400; color: var(--dsw-alias-label-caption); padding: 2px 8px 2px 0; }\n.dt-table td { padding: 4px 8px 4px 0; border-top: 0.5px solid var(--dsw-alias-border-l3); vertical-align: top; color: var(--dsw-alias-label-secondary); overflow-wrap: anywhere; }\n@container (max-width: 640px) {\n  .dt-settings { grid-template-columns: 1fr; gap: 12px; }\n  .dt-settings-nav { position: static; flex-direction: row; flex-wrap: wrap; }\n  .dt-settings-link[data-stage] + .dt-settings-link:not([data-stage]) { margin-top: 0; }\n  .dt-page .dt-tabs { gap: 14px; }\n}\n.dt-list > .dt-ticket { display: grid; grid-template-columns: 112px minmax(0, 1fr); column-gap: 12px; align-items: start; margin: 0; padding: 12px 8px 12px 18px; border-radius: 0; border-bottom: 0.5px solid var(--dsw-alias-border-l3); }\n.dt-list > .dt-ticket[data-state]::before { content: ''; position: absolute; left: 6px; top: 12px; bottom: 12px; width: 2px; border-radius: 1px; background: var(--dsw-alias-state-warn-primary); opacity: 0.7; }\n.dt-list > .dt-ticket[data-state='error']::before { background: var(--dsw-alias-state-error-primary); }\n.dt-ticket-stage { align-self: stretch; padding-top: 1px; border-right: 0.5px solid var(--dsw-alias-border-l3); }\n.dt-ticket-body { display: flex; flex-direction: column; gap: 4px; min-width: 0; }\n.dt-ticket-body .dt-row-title .dt-open { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }\n.dt-ticket-meta { flex-wrap: nowrap; overflow: hidden; white-space: nowrap; }\n.dt-ticket-meta .dt-chip { flex: none; }\n.dt-ticket-meta .dt-ticket-actions { margin-top: 0; margin-left: auto; min-height: 0; flex-wrap: nowrap; }\n@container (max-width: 640px) { .dt-ticket-meta { flex-wrap: wrap; white-space: normal; } .dt-ticket-meta .dt-ticket-actions { flex-wrap: wrap; } }\n.dt-ticket-note { min-width: 0; overflow: hidden; text-overflow: ellipsis; color: var(--dsw-alias-label-primary); }\n.dt-ticket-note-error { color: var(--dsw-alias-state-error-primary); }\n.dt-ticket-note-pause { font-weight: 500; color: var(--dsw-alias-state-warn-primary); }\n.dt-ticket-actions { position: relative; z-index: 1; display: flex; flex-wrap: wrap; gap: 8px; min-height: 30px; margin-top: 2px; }\n/* Pass pointer events through action containers to the row link, keeping only child controls interactive. */\n.dt-ticket-actions, .dt-row-buttons, .dt-row-actions { pointer-events: none; }\n.dt-ticket-actions > *, .dt-row-buttons > *, .dt-row-actions > * { pointer-events: auto; }\n.dt-line { min-height: 20px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }\n.dt-row-meta.dt-line { flex-wrap: nowrap; }\n.dt-line-text { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--dsw-alias-label-secondary); }\n.dt-error-text { color: var(--dsw-alias-state-error-primary); }\n.dt-stage { display: inline-flex; align-items: flex-start; gap: 5px; font-size: 12px; line-height: 18px; font-weight: 500; color: var(--dsw-alias-label-secondary); }\n.dt-stage svg { flex: none; margin-top: 2px; }\n.dt-stage[data-state='warning'] { color: var(--dsw-alias-state-warn-primary); }\n.dt-stage[data-state='error'] { color: var(--dsw-alias-state-error-primary); }\n.dt-stage[data-state='ongoing'] { color: var(--dsw-alias-state-business-primary); }\n.dt-icon-link { display: inline-flex; align-items: center; gap: 4px; }\na.dt-btn, a.dt-btn:hover { text-decoration: none; }\n.dt-section-icon { display: inline-flex; align-self: center; color: var(--dsw-alias-label-tertiary); }\n.dt-trigger-link { display: flex; flex-direction: column; gap: 2px; width: 100%; margin: -4px 0 -4px -8px; padding: 4px 8px; border: 0; border-radius: 8px; background: none; font: inherit; color: inherit; text-align: left; cursor: pointer; }\n.dt-trigger-link:hover { background: var(--dsw-alias-interactive-bg-hover); }\n.dt-trigger-link:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 1px; }\n.dt-trigger-text { display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }\n.dt-kv-stack { display: flex; flex-direction: column; gap: 2px; min-width: 0; }\n.dt-banner-actions { margin-top: 6px; }\n@container (max-width: 640px) {\n  .dt-list > .dt-ticket { grid-template-columns: 1fr; row-gap: 4px; }\n  .dt-ticket-stage { border-right: 0; }\n}\n.dt-nav-group { margin: 10px 10px 2px; font-size: 12px; font-weight: 600; color: var(--dsw-alias-label-secondary); }\n.dt-nav-group:first-child { margin-top: 0; }\n.dt-prompt-nav .dt-nav-item { flex-direction: column; align-items: flex-start; gap: 2px; text-align: left; }\n.dt-nav-item > span { display: inline-flex; align-items: center; gap: 6px; }\n.dt-nav-item small { font-size: 12px; line-height: 18px; font-weight: 400; color: var(--dsw-alias-label-caption); }\n.dt-preview-part[data-editable] { border-left-color: var(--dsw-alias-brand-primary); }\n.dt-preview-head b { font-weight: 500; }\n.dt-preview-head > span { font-size: 12px; color: var(--dsw-alias-label-caption); }\n.dt-changed-tag { margin-left: 6px; padding: 0 5px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 4px; font-size: 11px; line-height: 16px; font-weight: 400; color: var(--dsw-alias-label-secondary); }\n.dt-updated-tag { border-color: var(--dsw-alias-state-warn-primary); color: var(--dsw-alias-state-warn-primary); }\n.dt-default-updated { display: flex; flex-direction: column; gap: 6px; padding: 2px 0 2px 12px; border-left: 2px solid var(--dsw-alias-state-warn-primary); }\n.dt-diff-fold { color: var(--dsw-alias-label-caption); font-style: italic; }\n.dt-compare { margin-left: auto; }\n.dt-admission th { white-space: nowrap; }\n.dt-admission thead tr:first-child th:not(:first-child) { color: var(--dsw-alias-label-secondary); font-weight: 500; }\n.dt-admission td { vertical-align: middle; }\n.dt-admission-scope { color: var(--dsw-alias-label-primary); font-size: 13px; }\n.dt-admission-scope small { display: block; font-size: 11px; color: var(--dsw-alias-label-caption); }\n.dt-admission-na { color: var(--dsw-alias-label-caption); }\n.dt-admission-cell { display: flex; align-items: center; gap: 4px; }\n.dt-admission-cell .dt-select { flex: 1; min-width: 0; }\n.dt-admission-slot { display: inline-flex; flex: none; width: 16px; }\n.dt-info-warn { color: var(--dsw-alias-state-warn-primary); }\n.dt-field-segmented { max-width: 240px; }\n/* 活跃时段：两个原生 time 选择器 + 全天按钮 */\n/* 表单控件不必满宽：短数字撑到 900px 只会显得空 */\n.dt-form > .dt-field > .dt-input { max-width: 480px; }\n.dt-error-card { border-color: color-mix(in srgb, var(--dsw-alias-state-error-primary) 35%, var(--dsw-alias-border-l3)); }\n.dt-error-card .dt-card-title { color: var(--dsw-alias-state-error-primary); }\n/* 空态引导：圆形步骤号 + 标题 + 说明，替代「① + 列表符号」的重复 */\n.dt-steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; counter-reset: none; }\n.dt-step { display: flex; align-items: flex-start; gap: 10px; min-width: 0; }\n.dt-step-num { flex: none; display: inline-flex; align-items: center; justify-content: center; width: 20px; height: 20px; border-radius: 50%; background: var(--dsw-alias-bg-module-platform); color: var(--dsw-alias-label-secondary); font-size: 12px; line-height: 1; font-variant-numeric: tabular-nums; }\n.dt-step-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }\n.dt-step-title { font-size: 13px; line-height: 20px; font-weight: 500; color: var(--dsw-alias-label-primary); }\n.dt-empty { display: flex; flex-direction: column; gap: 6px; padding: 16px; border: 0.5px solid var(--dsw-alias-border-l3); border-radius: 12px; }\n.dt-step-detail { font-size: 12px; line-height: 18px; color: var(--dsw-alias-label-tertiary); }\n.dt-hours { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }\n.dt-time { box-sizing: border-box; height: 32px; padding: 0 8px; border: 0.5px solid var(--dsw-alias-border-l4); border-radius: 8px; background: var(--dsw-alias-bg-layer-1); color: var(--dsw-alias-label-primary); font: inherit; font-size: 13px; font-variant-numeric: tabular-nums; }\n.dt-time:focus-visible { outline: 2px solid var(--dsw-alias-brand-primary); outline-offset: 1px; }\n.dt-hours-sep { color: var(--dsw-alias-label-tertiary); }\n.dt-hours-hint { font-size: 12px; color: var(--dsw-alias-label-caption); }\n/* 分诊信号表（设置 → 诊断）：左列「名字 + 两个标签」成组，右列 `+N 分` 定宽贴右、数字等宽。只挂局部类，全局 .dt-row-title 一行没动 —— 别的列表还在用它。 */\n.dt-signal-title { display: grid; grid-template-columns: minmax(0, 1fr) 4.5em; align-items: baseline; column-gap: 12px; }\n.dt-signal-lead { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 8px; min-width: 0; }\n.dt-signal-meta { display: block; justify-self: stretch; text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }\n/* 同一个维度（默认开/关）的两档要等权：`默认关` 原本是 49×19 的裸字，`默认开` 是 51×21 的描边胶囊。 */\n.dt-signal-lead .dt-tag[data-tone='quiet'] { border: 0.5px solid var(--dsw-alias-border-l4); }\n";

    /** 把移植过来的样式挂到 document 上，只挂一次。 */
    /**
     * 悬浮面板只保留「定位」用 inline style，视觉全部走 dt-* + token。
     * 以前这里是一整套自成体系的 inline style，和原生界面放一起能看出断层。
     */
    const DOCK_CSS = `
/* Composer dock 入口：保留 20px derived 几何；交互色与焦点复用官方 dt-btn / ghost 规则。 */
.dt-btn.sa-orbit-trigger { display: inline-flex; align-items: center; justify-content: center; height: 20px; min-width: 20px; padding: 0 2px; margin: 0 2px; border: 0; border-radius: 6px; background: transparent; cursor: pointer; color: inherit; font: inherit; opacity: 1; }
.dt-btn.sa-orbit-trigger:hover { background: var(--dsw-alias-interactive-bg-hover); }
.dt-btn.sa-orbit-trigger:active, .dt-btn.sa-orbit-trigger[aria-expanded='true'] { background: var(--dsw-alias-interactive-bg-active); }
/* 初始读取仍可点击；0.4 与官方 .dt-btn:disabled 同档，不再手写 0.45 inline。 */
.dt-btn.sa-orbit-trigger.is-loading { opacity: 0.4; }
/* 弹层：卡片 + 一行摘要 + 比例条，全部走 dt-* 与 --dsw-alias-* token（官方弹层的做法） */
/* dt-card 只有边框没有底色，弹层叠在聊天内容上会透字 —— 官方弹层是实心 layer-2 */
.sa-dock-panel { display: flex; flex-direction: column; gap: 0; padding: 0; overflow: hidden; border: 0.5px solid var(--dsw-alias-border-l3); background: var(--dsw-alias-bg-layer-2); box-shadow: var(--dsw-elevation-prominent); border-radius: 16px; font-size: 12px; line-height: 18px; max-width: calc(100vw - 16px); box-sizing: border-box; }
.sa-dock-head { display: flex; align-items: baseline; gap: 8px; padding: 12px 14px 10px; border-bottom: 0.5px solid var(--dsw-alias-border-l3); }
.sa-dock-title { font-size: 13px; line-height: 20px; font-weight: 500; color: var(--dsw-alias-label-primary); }
.sa-dock-metric { margin-left: auto; }
.sa-dock-bar { display: flex; gap: 2px; height: 2px; margin: 0; border-radius: 0; overflow: hidden; background: var(--dsw-alias-bg-module-platform); }
.sa-dock-bar > span { display: block; height: 100%; border-radius: 0; }
.sa-dock-bar > span[data-tone='ok'] { background: var(--dsw-alias-state-success-primary); }
.sa-dock-bar > span[data-tone='warn'] { background: var(--dsw-alias-border-l4); }
.sa-dock-bar > span[data-tone='err'] { background: var(--dsw-alias-state-error-primary); }
.sa-dock-rows { display: flex; flex-direction: column; gap: 2px; margin: 0; padding: 0 6px 4px; max-height: 264px; overflow: auto; list-style: none; }
.sa-dock-row { display: flex; align-items: flex-start; gap: 8px; padding: 7px 8px; border-radius: 10px; }
.sa-dock-row:hover, .sa-dock-row:focus-within { background: var(--dsw-alias-interactive-bg-hover); }
.sa-dock-row > .dt-state-dot { margin-top: 5px; }
.sa-dock-body { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.sa-dock-line { display: flex; align-items: center; gap: 6px; min-width: 0; }
/* DSH DisclosureRow 承载行头；sa-dock-* 只补 watcher 的多行详情与动作布局，不冒充官方类名。 */
.sa-dock-official-row { flex-direction: column; align-items: stretch; gap: 2px; }
.sa-dock-official-row .sa-dock-content { display: block; width: 100%; }
.sa-dock-official-row .sa-dock-name { flex: 0 1 auto; }
.sa-dock-official-row .sa-dock-body { margin-left: 22px; }
.sa-dock-session { flex: none; font-size: 11px; line-height: 18px; color: var(--dsw-alias-label-tertiary); }
.sa-dock-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; line-height: 18px; font-weight: 500; color: var(--dsw-alias-label-primary); }
.sa-dock-sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--dsw-alias-label-tertiary); }
.sa-dock-err { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--dsw-alias-state-error-primary); }
.sa-dock-actions { display: flex; flex: none; gap: 2px; opacity: 0; transition: opacity 120ms ease; }
.sa-dock-row:hover .sa-dock-actions, .sa-dock-row:focus-within .sa-dock-actions { opacity: 1; }
@media (hover: none) { .sa-dock-actions { opacity: 1; } }
@media (prefers-reduced-motion: reduce) { .sa-dock-actions { transition: none; } }
.sa-dock-empty { padding: 4px 14px 12px; color: var(--dsw-alias-label-tertiary); }
.sa-dock-foot { padding: 8px 14px 10px; border-top: 0.5px solid var(--dsw-alias-border-l3); color: var(--dsw-alias-label-caption); }
`

    /**
     * 注入设计系统样式表。
     *
     * `force = true` 时**无条件重建**：弹层每次打开都调一次，用来兜住
     * 「宿主热重载 / 路由切换把 <head> 清掉」这种情况 —— 那时标签不在了，
     * 弹层就会变成一堆无样式裸文字（2026-11-03 真机复现过）。
     * 正常渲染路径用默认的幂等模式，只有"就快显示了"时才付出重建代价。
     */
    function ensurePortedCss(force) {
      try {
        if (!force) {
          const existing = document.getElementById('dsh-zhuli-ported-css')
          if (existing) return
        } else {
          const stale = document.getElementById('dsh-zhuli-ported-css')
          if (stale && stale.parentNode) stale.parentNode.removeChild(stale)
        }
        const style = document.createElement('style')
        style.id = 'dsh-zhuli-ported-css'
        style.textContent = PORTED_CSS + DOCK_CSS
        document.head.appendChild(style)
      } catch {}
    }

    // ── 官方 GUI 原语（移植自 dsh-superagent 的 src/client/controls.tsx + widgets.tsx）──
    // 类名沿用官方 `dt-*`，样式由上面的 PORTED_CSS 提供，所以视觉与官方一致。
    const ICON_PATHS = {
      // 以下 16×16 path 直接复用 home/dsh-ui-official/assets（官方 0.2.1-rc.5），不是照截图重画。
      'chevron-left': 'M10 4L6.70711 7.29289C6.31658 7.68342 6.31658 8.31658 6.70711 8.70711L10 12',
      'chevron-right': 'M6 12L9.29289 8.70711C9.68342 8.31658 9.68342 7.68342 9.29289 7.29289L6 4',
      'chevron-down': 'M4 6L7.29289 9.29289C7.68342 9.68342 8.31658 9.68342 8.70711 9.29289L12 6',
      // derived：主库没有 chevron-up，按官方 chevron-down 做垂直镜像，仍是 16×16。
      'chevron-up': 'M4 10L7.29289 6.70711C7.68342 6.31658 8.31658 6.31658 8.70711 6.70711L12 10',
      search: [
        'M6.58727 11.8586C9.55061 11.8586 11.9529 9.45637 11.9529 6.49304C11.9529 3.5297 9.55061 1.12744 6.58727 1.12744C3.62394 1.12744 1.22168 3.5297 1.22168 6.49304C1.22168 9.45637 3.62394 11.8586 6.58727 11.8586Z',
        'M10.2991 10.3933L14.7783 14.8725',
      ],
      question: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 17h.01',
      plus: ['M8 2V14', 'M2 8H14'],
      trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
      refresh: 'M20 12a8 8 0 1 1-2.3-5.6M20 4v5h-5',
      clock: [
        'M8 14C11.3137 14 14 11.3137 14 8C14 4.68629 11.3137 2 8 2C4.68629 2 2 4.68629 2 8C2 11.3137 4.68629 14 8 14Z',
        'M8 4.31V8.46L11 10.08',
      ],
      history: 'M3.5 12a8.5 8.5 0 1 0 2.5-6M3.5 4v4h4M12 8v4l3 2',
      shield: 'M12 3l7 3v6c0 4.2-2.9 7.6-7 9-4.1-1.4-7-4.8-7-9V6z',
      decision: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM8.5 12.5l2.5 2.5 4.5-5',
      message: 'M5 5h14a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H10l-5 4V6a1 1 0 0 1 1-1z',
      inbox: 'M4 13l2.5-8h11l2.5 8M4 13v6h16v-6M4 13h5l1 2h4l1-2h5',
      hash: 'M10 4L8 20M16 4l-2 16M5 9h15M4 15h15',
      filter: 'M4 5h16l-6 7.5V18l-4 2v-7.5z',
      user: 'M12 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM4 20c1.5-3.5 4.5-5 8-5s6.5 1.5 8 5',
      close: ['M2.5 2.5L13.5 13.5', 'M13.5 2.5L2.5 13.5'],
      settings: [
        'M8 9.75012C8.9665 9.75012 9.75 8.96662 9.75 8.00012C9.75 7.03362 8.9665 6.25012 8 6.25012C7.0335 6.25012 6.25 7.03362 6.25 8.00012C6.25 8.96662 7.0335 9.75012 8 9.75012Z',
        'M13.0107 7.79377C12.9505 7.89401 12.9205 7.94413 12.9205 7.99951C12.9205 8.0549 12.9505 8.10502 13.0106 8.20528L13.9849 9.83006C14.045 9.93029 14.0751 9.9804 14.0751 10.0358C14.0751 10.0911 14.045 10.1413 13.9849 10.2415L13.0037 11.8777C12.9468 11.9726 12.9184 12.0201 12.8725 12.0461C12.8267 12.072 12.7713 12.072 12.6607 12.072H10.6704C10.5598 12.072 10.5045 12.072 10.4586 12.098C10.4128 12.1239 10.3843 12.1714 10.3274 12.2662L9.33825 13.9142C9.28133 14.009 9.25287 14.0564 9.20703 14.0823C9.16118 14.1083 9.10588 14.1083 8.99529 14.1083H7.00486C6.89426 14.1083 6.83896 14.1083 6.79312 14.0823C6.74727 14.0564 6.71881 14.009 6.6619 13.9142L5.67273 12.2662C5.61581 12.1714 5.58735 12.1239 5.54151 12.098C5.49566 12.072 5.44036 12.072 5.32977 12.072H3.33945C3.2288 12.072 3.17347 12.072 3.12761 12.0461C3.08176 12.0201 3.0533 11.9726 2.9964 11.8777L2.0152 10.2415C1.9551 10.1413 1.92505 10.0911 1.92505 10.0358C1.92505 9.9804 1.9551 9.93029 2.0152 9.83006L2.98951 8.20528C3.04963 8.10502 3.07969 8.0549 3.07969 7.99951C3.07968 7.94413 3.04961 7.89401 2.98946 7.79377L2.01529 6.17011C1.95514 6.06987 1.92507 6.01975 1.92507 5.96437C1.92506 5.90899 1.95512 5.85886 2.01524 5.7586L2.9964 4.1224C3.0533 4.0275 3.08176 3.98005 3.12761 3.95408C3.17347 3.92811 3.2288 3.92811 3.33945 3.92811H5.32977C5.44036 3.92811 5.49566 3.92811 5.54151 3.90216C5.58735 3.87621 5.61581 3.82879 5.67273 3.73397L6.6619 2.08599C6.71881 1.99116 6.74727 1.94375 6.79312 1.9178C6.83896 1.89185 6.89426 1.89185 7.00486 1.89185H8.99529C9.10588 1.89185 9.16118 1.89185 9.20703 1.9178C9.25287 1.94375 9.28133 1.99116 9.33825 2.08599L10.3274 3.73397C10.3843 3.82879 10.4128 3.87621 10.4586 3.90216C10.5045 3.92811 10.5598 3.92811 10.6704 3.92811H12.6607C12.7713 3.92811 12.8267 3.92811 12.8725 3.95408C12.9184 3.98005 12.9468 4.0275 13.0037 4.1224L13.9849 5.7586C14.045 5.85886 14.0751 5.90899 14.0751 5.96437C14.0751 6.01975 14.045 6.06987 13.9849 6.17011L13.0107 7.79377Z',
      ],
      external: 'M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5',
      session: 'M4 5h16v14H4zM8 10l3 2-3 2M13 15h3',
      stop: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM9.5 9.5h5v5h-5z',
      pause: 'M9 6v12M15 6v12',
      play: 'M8 5l11 7-11 7z',
      copy: 'M9 9h10v11H9zM5 15V4h10',
      alert: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7.5v5.5M12 16.5h.01',
      running: 'M12 3a9 9 0 1 0 9 9',
      blocked: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM5.6 5.6l12.8 12.8',
      queue: 'M4 6h16M4 12h16M4 18h10',
    }

    // 直接复用的官方图标都是 16×16；其余 derived 图标仍是 24×24，不能混用 viewBox。
    const OFFICIAL_16 = new Set(['chevron-left', 'chevron-right', 'chevron-down', 'chevron-up', 'search', 'plus', 'clock', 'close', 'settings'])

    function Icon({ name, size = 16 }) {
      const value = ICON_PATHS[name] || ICON_PATHS.question
      const paths = Array.isArray(value) ? value : [value]
      return h('svg', {
        width: size, height: size, viewBox: OFFICIAL_16.has(name) ? '0 0 16 16' : '0 0 24 24', fill: 'none',
        stroke: 'currentColor', strokeWidth: 1.8, strokeLinecap: 'round', strokeLinejoin: 'round',
        'aria-hidden': 'true',
      }, paths.map((d, index) => h('path', { key: index, d })))
    }

    const cx = (...names) => names.filter(Boolean).join(' ')

    // 安装版 Button 的真实 variant 映射：primary/ghost/outline/toolbar × md/sm，默认 ghost/md。
    // 业务的 link 没有官方同物：保留旧 dt-btn-link，不借 toolbar/ghost 自造映射。
    const HOST_BUTTON_VARIANTS = new Set(['primary', 'ghost', 'outline', 'toolbar'])

    function Btn({ variant = 'ghost', size = 'md', icon, children, className, ...rest }) {
      if (hostButton && HOST_BUTTON_VARIANTS.has(variant)) {
        // 只转发业务 className：旧 dt-btn + variant/size 会覆盖宿主主题，不能回填到官方路径。
        // type 默认 button，但显式 submit 等原生 props（含 ref）必须原样交给 DSH Button；key 仍由 React 管理。
        return h(hostButton, { type: 'button', ...rest, variant, size, icon, className }, children)
      }
      // className 必须最后合并：之前 ...rest 直接覆盖，导致 ConfirmButton 的删除按钮丢掉全部样式
      return h('button', { type: 'button', ...rest, className: cx('dt-btn', 'dt-btn-' + variant, 'dt-btn-' + size, className) },
        icon !== undefined ? h('span', { className: 'dt-btn-icon' }, icon) : null,
        children)
    }

    // 保留安装动作现有调用面；其它受支持 variant 也统一由 Btn 内的宿主分支委托，老宿主完整回退。
    const UpdateButton = hostButton || Btn

    function Tag({ tone = 'outline', title, className, children }) {
      if (hostTag) {
        const tag = h(hostTag, { tone, className }, children)
        // 官方 Tag 是静态只读组件，不收 title；只在确有提示时加无样式 wrapper，不丢原有悬停语义。
        return title === undefined ? tag : h('span', { title }, tag)
      }
      return h('span', { className: cx('dt-tag', className), 'data-tone': tone, title }, children)
    }

    /** 页级 tab 条：点当前页会再发一次 onChange（回调里可重置到根部）。 */
    function SegTabs({ items, value, onChange, label }) {
      // 官方组件拥有点击、方向键/Home/End、roving tab stop 与焦点移动；这里只转发受控 props。
      if (hostSegmentedTabs) return h(hostSegmentedTabs, { items, value, onChange, label })
      const buttons = useRef({})
      const walk = (event) => {
        if (items.length === 0) return
        const values = items.map((item) => item.value)
        const index = values.indexOf(value)
        const target = { ArrowRight: index + 1, ArrowLeft: index - 1, Home: 0, End: values.length - 1 }[event.key]
        if (target === undefined) return
        event.preventDefault()
        const next = values[(target + values.length) % values.length]
        if (next !== value) {
          onChange(next)
          // 焦点跟随：旧页签变成 tabIndex=-1 后，键盘用户会掉出 tablist
          const nextButton = buttons.current[next]
          if (nextButton && typeof nextButton.focus === 'function') nextButton.focus()
        }
      }
      return h('div', {
        role: 'tablist', 'aria-label': label, className: 'dt-tabs', onKeyDown: walk,
        style: { gridTemplateColumns: 'repeat(' + items.length + ', minmax(0, 1fr))' },
      }, items.map((item) => h('button', {
        key: item.value, id: item.id, type: 'button', role: 'tab',
        ref: (node) => { buttons.current[item.value] = node },
        'aria-selected': item.value === value,
        'aria-controls': item.panelId,
        tabIndex: item.value === value ? 0 : -1,
        className: 'dt-tab',
        onClick: () => onChange(item.value),
      }, item.label)))
    }

    function Input({ icon, className, ...rest }) {
      if (hostInput) {
        // 官方 className 作用于唯一 wrapper；保留 dt-input 让现有 Field/Search 布局选择器继续命中。
        // icon 只交给官方渲染，ref 随 rest 交给官方 forwardRef，不能再包本地 span 或复刻 icon。
        return h(hostInput, { ...rest, icon, className: cx('dt-input', className) })
      }
      // 老宿主完整保留 superagent 同源结构；ref 与全部原生 input 属性仍落在真正的 input 上。
      return h('span', { className: 'dt-input' },
        icon !== undefined ? h('span', { className: 'dt-input-icon' }, icon) : null,
        h('input', { ...rest, className }))
    }

    function Toggle({ checked, onChange, label, id, disabled, title }) {
      if (hostSwitch) {
        const control = h(hostSwitch, { checked: !!checked, onChange, label, disabled, title })
        // 官方 Switch 不接收 id；仅用无样式 span 保留调用方 id，官方仍拥有 switch 语义与交互。
        return id === undefined ? control : h('span', { id }, control)
      }
      return h('button', {
        id, type: 'button', role: 'switch', 'aria-checked': !!checked, 'aria-label': label, title,
        className: 'dt-switch', disabled, onClick: () => onChange(!checked),
      }, h('span', { className: 'dt-switch-thumb' }))
    }

    function StateDot({ state, size, className }) {
      if (hostStateDot) return h(hostStateDot, { state, size, className })
      if (state === 'ongoing') {
        return h('svg', { className: cx('dt-spinner', className), 'data-state': 'ongoing', width: size ?? 14, height: size ?? 14, viewBox: '0 0 24 24', 'aria-hidden': 'true' },
          h('g', null,
            h('circle', { cx: 12, cy: 12, r: 9.5 }),
            h('animateTransform', { attributeName: 'transform', type: 'rotate', from: '0 12 12', to: '360 12 12', dur: '1.6s', repeatCount: 'indefinite' })))
      }
      return h('span', { className: cx('dt-state-dot', className), 'data-state': state, 'aria-hidden': 'true' })
    }

    function Dot({ state, label }) {
      const dot = h(StateDot, { className: 'dt-dot', state })
      if (!label) return dot
      return h('span', { className: 'dt-dot-wrap', title: label, role: 'img', 'aria-label': label }, dot)
    }

    // 官方 Section 还支持 hint（说明）与 action（右上角动作），我们之前只有 title/count。
    /**
     * 官方统计单元：数值强（`dt-stat-value`）+ 单位弱（caption）。
     * 面板总览的统计条与弹层顶部指标共用 —— 以前是两份渲染代码，
     * 改一处忘一处就会变成"同一页两种数字排版"。
     */
    function Stat({ value, unit, className, onClick, active }) {
      const cls = 'dt-stat' + (className ? ' ' + className : '') + (onClick ? ' dt-stat-btn' : '')
      const kids = [
        h('span', { className: 'dt-stat-value' }, String(value)),
        h('span', { className: 'dt-stat-unit' }, unit),
      ]
      // 可点的统计数字就是筛选入口（官方 rows 的做法）：点「异常」只看异常，再点一次取消
      return onClick
        ? h('button', { type: 'button', className: cls, 'aria-pressed': !!active, onClick }, kids[0], kids[1])
        : h('span', { className: cls }, kids[0], kids[1])
    }

    // 审核 T26 W2：以前  传了但这里不解构 —— 传的人以为会显示，其实被静默吞掉。
    function Section({ title, count, subtitle, hint, action, children }) {
      return h('section', { className: 'dt-section' },
        title
          ? h('h2', { className: 'dt-section-title' },
            h('span', { className: 'dt-section-name' }, title),
            count !== undefined && count !== null ? h('span', { className: 'dt-count' }, String(count)) : null,
            subtitle ? h('span', { className: 'dt-section-sub' }, subtitle) : null,
            action ? h('span', { className: 'dt-subhead-actions' }, action) : null)
          : null,
        hint ? h(Hint, null, hint) : null,
        children)
    }

    /**
     * 第一次打开面板时的空态：官方那种「不是白屏，而是告诉你下一步做什么」。
     * 三步走完就能跑起来 —— 客户不用去翻 README。
     */
    function Onboarding({ onStart }) {
      // 空态是「引导」不是「表格」：① 不当列表符号，而是**步骤圆点 + 标题 + 说明**三件套。
      // 出图时看到「① 填一个仓库（owner/name，比如 …）」——列表符号和步骤号重复，说明文字又和标题挤在同一行。
      const steps = [
        ['填一个仓库', 'owner/name，例如 MYCF711/dsh-session-superagent'],
        ['填间隔', '新手先填 300 秒（5 分钟）—— 默认 60 秒适合盯得紧的仓库，公开仓库容易撞限流'],
        ['点「添加到当前会话」', '之后仓库有动静，这个会话就会被叫醒'],
      ]
      return h('div', { className: 'dt-card sa-onboarding' },
        h('div', { className: 'dt-card-head' },
          h('span', { className: 'dt-card-title' }, '三步让这个会话开始盯办')),
        h('ol', { className: 'dt-steps' },
          steps.map(([title, detail], index) => h('li', { key: title, className: 'dt-step' },
            h('span', { className: 'dt-step-num', 'aria-hidden': 'true' }, String(index + 1)),
            h('div', { className: 'dt-step-body' },
              h('span', { className: 'dt-step-title' }, title),
              h('span', { className: 'dt-step-detail' }, detail))))),
        h('div', { className: 'dt-actions' },
          h(Btn, { variant: 'primary', icon: h(Icon, { name: 'plus' }), onClick: onStart }, '去添加第一条')),
        h(Hint, null, '想先看看效果？在「触发」页勾上「仅观察」：它只记录本会唤醒哪几条，不会真打扰你。'))
    }

    /** 官方 DisclosureRow 承载过滤清单；缺导出时完整保留本地键盘与焦点行为。 */
    function CollapsibleList({ title, items }) {
      const [open, setOpen] = useState(false)
      // 字符串仍是兼容调用面；结构化记录带独立摘要、正文和稳定 key，避免先压平而丢失来源。
      const summaryOf = (item) => item && typeof item === 'object' && 'summary' in item ? item.summary : item
      const contentOf = (item) => item && typeof item === 'object' && 'content' in item ? item.content : item
      const rowProps = {
        title: title + '（' + items.length + '）',
        open, onToggle: () => setOpen((current) => !current),
        icon: h(Icon, { name: 'question', size: 14 }),
        collapsedContent: h('span', { className: 'dt-hint' }, items[0] ? String(summaryOf(items[0])).slice(0, 40) : ''),
      }
      return h(hostCollapsibleRow || DisclosureRow,
        hostCollapsibleRow ? { ...rowProps, expandable: true, expandOnRowClick: true } : rowProps,
        h('ul', { className: 'dt-plain-list' },
          items.map((item, index) => h('li', {
            key: item && typeof item === 'object' && item.key !== undefined ? item.key : index,
          }, contentOf(item)))))
    }

    /** DSH Pill：active 有定义时才是 toggle；无 active 的 onClick 只是动作，静态则无 onClick。 */
    function Pill({ active, className, onClick, children, ...rest }) {
      if (hostPill) {
        // 官方没有 aria-pressed；只给真正的 toggle 补齐，清除等普通动作不得误报 pressed=false。
        return h(hostPill, {
          ...rest, active, className, onClick,
          'aria-pressed': onClick && active !== undefined ? !!active : undefined,
        }, children)
      }
      if (!onClick) {
        return h('span', { className: cx('dt-pill', active && 'dt-pill-active', className) }, children)
      }
      return h('button', {
        type: 'button', ...rest, className: cx('dt-pill', active && 'dt-pill-active', className),
        'aria-pressed': active !== undefined ? !!active : undefined, onClick,
      }, children)
    }

    /** 官方 SegmentedControl：单选分段（与 SegTabs 的页签语义不同，用于表单内的互斥选择）。 */
    function SegmentedControl({ id, options, value, onChange, label, disabled }) {
      // 官方组件自带方向键 / Home / End / 跳过 disabled / roving tab stop；这里只转发受控 props。
      if (hostSegmentedControl) {
        return h(hostSegmentedControl, { id, options, value, onChange, label, disabled })
      }
      // 老宿主完整保留本地实现 —— 键盘行为与官方对齐，不再只靠鼠标。
      const buttons = useRef({})
      const enabled = options.filter((option) => !(disabled || option.disabled))
      const hasValue = options.some((option) => option.value === value)
      const walk = (event) => {
        const index = enabled.findIndex((option) => option.value === value)
        const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key]
        let next
        if (step !== undefined) {
          if (enabled.length === 0) return
          const from = index < 0 ? 0 : index
          next = enabled[(from + step + enabled.length) % enabled.length]
        } else if (event.key === 'Home') {
          next = enabled[0]
        } else if (event.key === 'End') {
          next = enabled[enabled.length - 1]
        } else {
          return
        }
        if (!next) return
        event.preventDefault()
        if (next.value !== value) onChange(next.value)
        // 焦点跟随：旧选中项变成 tabIndex=-1 后，键盘用户会掉出 radiogroup。
        const node = buttons.current[next.value]
        if (node && typeof node.focus === 'function') node.focus()
      }
      return h('div', {
        className: 'dt-segmented',
        id,
        role: 'radiogroup',
        'aria-label': label,
        onKeyDown: walk,
        style: { gridTemplateColumns: 'repeat(' + options.length + ', minmax(0, 1fr))' },
      },
        options.map((option, index) => {
          const selected = option.value === value
          const unavailable = Boolean(disabled || option.disabled)
          return h('button', {
            key: option.value, type: 'button', role: 'radio',
            'aria-checked': selected,
            ref: (node) => { buttons.current[option.value] = node },
            tabIndex: selected ? 0 : (hasValue ? -1 : (index === 0 ? 0 : -1)),
            className: 'dt-segment', disabled: unavailable,
            onClick: () => { if (!unavailable) onChange(option.value) },
          }, option.label)
        }))
    }

    /** 官方 DisclosureRow：折叠行的标准写法（Enter / 空格可切换，带 aria-expanded）。 */
    function DisclosureRow({ icon, title, open, onToggle, collapsedContent, bodyId, busy = false, disabled = false, children }) {
      const content = Array.isArray(children) && children.length === 1 ? children[0] : children
      const unavailable = Boolean(busy || disabled)
      return h('div', { className: 'dt-disclosure', 'data-open': open ? 'true' : undefined },
        h('div', {
          className: 'dt-disclosure-row', role: 'button', tabIndex: unavailable ? -1 : 0, 'aria-expanded': !!open,
          'aria-controls': bodyId,
          'aria-disabled': unavailable,
          'aria-busy': busy ? 'true' : undefined,
          'aria-label': busy && typeof title === 'string' ? title + '（保存中，暂不可开合）' : undefined,
          onClick: () => { if (!unavailable) onToggle() },
          onKeyDown: (event) => {
            if (event.key !== 'Enter' && event.key !== ' ') return
            event.preventDefault()
            // 长按只认第一次；busy/disabled 也必须在本层拦截，不能依赖调用方静默吞掉。
            if (unavailable || event.repeat) return
            onToggle()
          },
        },
          h('span', { className: 'dt-disclosure-lead' },
            h('span', { className: 'dt-disclosure-icon' }, icon),
            h('span', { className: 'dt-disclosure-chevron' }, h(Icon, { name: open ? 'chevron-up' : 'chevron-down' }))),
          h('span', { className: 'dt-disclosure-title' }, title),
          !open ? collapsedContent : null),
        open
          ? bodyId
            ? content && typeof content === 'object' && !Array.isArray(content)
              ? { ...content, props: { ...(content.props || {}), id: bodyId } }
              : h('div', { id: bodyId }, content)
            : children
          : null)
    }

    // 本地气泡的 id 序号：同一 trigger 反复开合复用同一个 id，trigger 才能稳定用 aria-describedby 指向它。
    // 用 ref 而不是新增 state，避免打乱父组件按位置取值的 hook 队列。
    let fallbackTipSerial = 0
    /** 无 DSH Tooltip 时完整保留原实现；补上 Escape 关闭与 trigger↔tooltip 关联（W15）。 */
    function FallbackInfoTip({ text, icon = 'question', warn }) {
      const [open, setOpen] = useState(false)
      const tipIdRef = useRef(null)
      if (!tipIdRef.current) {
        fallbackTipSerial += 1
        tipIdRef.current = 'dt-info-tip-' + fallbackTipSerial
      }
      const tipId = tipIdRef.current
      return h('span', { style: { position: 'relative', display: 'inline-flex' } },
        h('button', {
          type: 'button', className: cx('dt-info', warn && 'dt-info-warn'),
          'aria-label': text, 'aria-expanded': open,
          // 只有气泡真的在 DOM 里才关联；关闭时同步清掉，读屏不会指向已消失的节点。
          'aria-describedby': open ? tipId : undefined,
          onClick: () => setOpen(!open), onBlur: () => setOpen(false),
          onKeyDown: (event) => {
            if (event.key !== 'Escape' || !open) return
            // 键盘约定：Escape 只关气泡，焦点留在 trigger 上（不能 blur 掉）。
            event.preventDefault()
            setOpen(false)
          },
        }, h(Icon, { name: icon, size: 14 })),
        open ? h('span', {
          id: tipId, className: 'dt-tooltip', role: 'tooltip',
          style: { position: 'absolute', bottom: '130%', left: 0, visibility: 'visible' },
        }, text) : null)
    }

    // 保持 InfoTip({text,icon,warn}) 调用面；静态分支不会在组件生命周期内改变 hooks 顺序。
    function InfoTip(props) {
      return hostTooltip ? h(hostTooltip, props) : h(FallbackInfoTip, props)
    }

    const Hint = ({ children }) => h('p', { className: 'dt-hint' }, children)

    // ── L84c WatcherForm：新增与行内编辑共用的一份字段 schema 与草稿控件 ──
    // 这里刻意不引入 DSH InlineEditor：watcher 是多字段表单，不是单文本编辑器。
    const AUTHOR_LIST_PLACEHOLDER = 'GitHub 登录名，逗号分隔，留空 = 谁都算'
    const PRIORITY_LABEL = '优先级（在「单次最多唤醒条数」内先挑谁）'
    const CREATE_WATCHER_VALUES = {
      repo: '', intervalSec: '60', labels: '', ignoreLabels: '', assignee: '',
      activeHours: '', goal: '', ackLabel: '', onlyAssigned: false, allowAuthors: '',
      observe: false, isolated: false, workdir: '', batchLimit: '', maxWakesPerHour: '',
      sessionMaxWakes: '', isolateKeepDays: '', maxIsolated: '',
      priorityReopened: true, priorityComments: true, priorityLabels: true,
      priorityUnassigned: false, priorityStale: false,
    }
    const watcherNumberText = (value) => value === undefined || value === null || value === '' ? '' : String(value)
    const numberOrUndefined = (value) => {
      if (value === undefined || value === null || String(value).trim() === '') return undefined
      const parsed = Number(value)
      return Number.isFinite(parsed) ? parsed : undefined
    }
    // ack 默认值来自 /state.defaultAckLabels（issue #11）；老服务端没有这个字段时仍回落旧字面量。
    const ACK_LABEL_FALLBACK = { github: '主理鱼:已处理', gitee: '主理鱼-已处理' }
    const ackLabelFallback = (watcher, defaults) => {
      const provider = watcher && watcher.provider === 'gitee' ? 'gitee' : 'github'
      const fromState = defaults && typeof defaults[provider] === 'string' ? defaults[provider].trim() : ''
      return fromState || ACK_LABEL_FALLBACK[provider]
    }
    const watcherValuesFrom = (watcher, ackDefaults) => ({
      repo: watcher.repo || '',
      intervalSec: watcherNumberText(watcher.intervalSec ?? 60),
      labels: (watcher.filters && Array.isArray(watcher.filters.labels) ? watcher.filters.labels : []).join(','),
      ignoreLabels: (watcher.filters && Array.isArray(watcher.filters.ignoreLabels) ? watcher.filters.ignoreLabels : []).join(','),
      assignee: (watcher.filters && watcher.filters.assignee) || '',
      activeHours: watcher.activeHours || '',
      goal: watcher.note || (typeof watcher.goal === 'string' ? watcher.goal : (watcher.goal && watcher.goal.objective) || ''),
      ackLabel: watcher.ackLabel || ackLabelFallback(watcher, ackDefaults),
      onlyAssigned: !!(watcher.filters && watcher.filters.onlyAssigned),
      allowAuthors: (watcher.filters && Array.isArray(watcher.filters.allowAuthors) ? watcher.filters.allowAuthors : []).join(', '),
      observe: !!watcher.observe,
      isolated: !!watcher.isolated,
      workdir: watcher.workdir || '',
      batchLimit: watcherNumberText(watcher.batchLimit),
      maxWakesPerHour: watcherNumberText(watcher.maxWakesPerHour),
      sessionMaxWakes: watcherNumberText(watcher.sessionMaxWakes),
      isolateKeepDays: watcherNumberText(watcher.isolateKeepDays),
      maxIsolated: watcherNumberText(watcher.maxIsolated),
      priorityReopened: (watcher.priority && watcher.priority.reopened) !== false,
      priorityComments: (watcher.priority && watcher.priority.comments) !== false,
      priorityLabels: (watcher.priority && watcher.priority.labels) !== false,
      priorityUnassigned: Boolean(watcher.priority && watcher.priority.unassigned) === true,
      priorityStale: Boolean(watcher.priority && watcher.priority.stale) === true,
    })
    const watcherSameValue = (left, right) => String(left ?? '') === String(right ?? '')
    const watcherDirty = (value, snapshot) => Object.keys(snapshot || {}).some((key) => !watcherSameValue(value && value[key], snapshot[key]))

    // label/hint/note/control 的顺序只在这里声明一次；两个调用点只负责提供值与回调。
    const WATCHER_FIELD_SCHEMA = [
      { key: 'repo', label: '仓库', kind: 'text', modes: ['create'], required: true, placeholder: 'owner/name，或 gitee:owner/name' },
      { key: 'intervalSec', label: '间隔（秒）', kind: 'number', min: 30, max: 3600, required: true, hint: '多长时间检查一次这个仓库，最小 30 秒' },
      { key: 'labels', label: '只处理这些标签', kind: 'text', placeholder: '逗号分隔，留空 = 全部' },
      { key: 'ignoreLabels', label: '排除这些标签', kind: 'text', placeholder: '逗号分隔，留空 = 不排除', hint: '命中任意一个就跳过，优先级高于上面的白名单' },
      { key: 'allowAuthors', label: '只理这些人提的', kind: 'text', placeholder: AUTHOR_LIST_PLACEHOLDER, hint: '不在名单里的人提的 issue/PR 不会叫醒你（Gitee 也可以填）' },
      { key: 'activeHours', label: '活跃时段', kind: 'hours', hint: '只在这个时间段内唤醒；窗口外只攒条目，不打扰' },
      { key: 'goal', label: '目标/规则', kind: 'text', placeholder: '可选，会附在唤醒消息里' },
      { key: 'ackLabel', label: '已处理标签', kind: 'text', placeholder: '主理鱼:已处理', hint: 'watch_ack 会把同名标签打到 issue/PR 上；改名字等于换一套已处理标记' },
      { key: 'onlyAssigned', label: '只看已指派', kind: 'switch', hint: '只处理已指派给人的条目' },
      { key: 'assignee', label: '只看指派给谁', kind: 'text', when: (value) => !!value.onlyAssigned, placeholder: 'GitHub 登录名，留空 = 只要有指派就行' },
      { key: 'observe', label: '仅观察', kind: 'switch', hint: '仅观察（只记录本会唤醒哪几条，不真唤醒）' },
      { key: 'isolated', label: '隔离执行', kind: 'switch', hint: ISOLATED_HINT },
      { key: 'batchLimit', label: '单次最多唤醒条数', kind: 'number', advanced: true, min: 1, max: 20, placeholder: (defaults) => String(defaults.batchLimit ?? 5) + '（1-20）' },
      { key: 'maxWakesPerHour', label: '每小时最多唤醒次数', kind: 'number', advanced: true, min: 1, max: 60, placeholder: (defaults) => String(defaults.maxWakesPerHour ?? 6) + '（1-60）' },
      { key: 'sessionMaxWakes', label: '这条会话每小时最多被唤醒', kind: 'number', advanced: true, min: 1, max: 60, placeholder: (defaults) => String(defaults.sessionMaxWakes ?? 12) + '（1-60）', hint: '跨所有守累计：守 5 个仓库时，光有单守上限挡不住叠加' },
      { key: 'priority', label: PRIORITY_LABEL, kind: 'priority', advanced: true, hint: '全部关掉时按 GitHub 的「最近更新」顺序处理' },
      { key: 'workdir', label: '隔离工作目录', kind: 'text', advanced: true, when: (value) => !!value.isolated, placeholder: '绝对路径，留空 = 当前工作目录' },
      { key: 'isolateKeepDays', label: '隔离区保留天数', kind: 'number', advanced: true, when: (value) => !!value.isolated, min: 1, max: 30, placeholder: (defaults) => String(defaults.isolateKeepDays ?? 3) + '（1-30）', hint: '每次唤醒前回收这个天数之前的一次性工作区；留空用默认天数' },
      { key: 'maxIsolated', label: '最多保留几个隔离区', kind: 'number', advanced: true, when: (value) => !!value.isolated, min: 1, max: 50, placeholder: '5（1-50）', hint: '天数之外的第二道线：超出就回收最旧的（不影响正在用的那个）' },
    ]

    /** superagent Field 的 DOM：label 与 InfoTip 是兄弟节点，控件拿同一个唯一 id。 */
    let watcherFieldSerial = 0
    function Field({ id, label, hint, note, children }) {
      const generated = useRef(null)
      if (!generated.current) {
        watcherFieldSerial += 1
        generated.current = id || ('dt-watcher-field-' + watcherFieldSerial)
      }
      const fieldId = id || generated.current
      const child = Array.isArray(children) ? children[0] : children
      const control = child && typeof child === 'object'
        ? { ...child, props: { ...(child.props || {}), id: (child.props && child.props.id) || fieldId } }
        : child
      return h('div', { className: 'dt-field' },
        h('span', { className: 'dt-field-head' },
          h('label', { htmlFor: fieldId }, label),
          hint ? h(InfoTip, { text: hint }) : null),
        control,
        note ? h('small', null, note) : null)
    }

    /** superagent Fields：保存期间由原生 fieldset 统一禁用所有字段。 */
    function Fields({ disabled, children }) {
      return h('fieldset', { className: 'dt-fieldset', disabled: !!disabled }, children)
    }

    /** superagent SaveBar 的精简草稿状态：clean 不挂载，dirty/busy/error 才挂载。 */
    function SaveBar({ dirty, busy, error, disabled, onDiscard, onSave, saveLabel = '保存' }) {
      if (!dirty && !busy && !error) return null
      return h('div', { className: 'dt-savebar' },
        error
          ? h('p', { className: 'dt-error', role: 'alert' }, error)
          : h('p', { className: 'dt-hint', 'aria-live': 'polite' }, busy ? '保存中…' : '未保存'),
        h(Btn, { size: 'sm', variant: 'ghost', disabled: !dirty || !!busy || !!disabled, onClick: onDiscard }, '放弃'),
        h(Btn, { size: 'sm', variant: 'primary', type: 'submit', disabled: !dirty || !!busy || !!disabled, onClick: onSave }, busy ? '保存中…' : saveLabel))
    }

    /**
     * SaveBar 消失后只在焦点仍属于这张表单时归还；用户已移到其它存活控件就不抢。
     * 编辑保存会卸载整个 form，所以 opener 在提交前捕获；新增/放弃则回首个可编辑控件。
     */
    function restoreSaveBarFocus({ form, saveBar, opener, wasFocused = false, shouldRestore }) {
      if (typeof document === 'undefined') return
      const active = document.activeElement
      const inSaveBar = (node) => Boolean(node && saveBar && typeof saveBar.contains === 'function' && saveBar.contains(node))
      const lost = (node) => !node || node === document.body || node.isConnected === false
      if (!inSaveBar(active) && !wasFocused) return
      setTimeout(() => {
        const current = document.activeElement
        if (!inSaveBar(current) && !lost(current)) return
        if (typeof shouldRestore === 'function' && !shouldRestore()) return
        // busy=true 时 input 仍 disabled；必须等 finally 与 React commit 后再找可编辑控件。
        const target = opener && opener.isConnected !== false
          ? opener
          : form && form.isConnected !== false && typeof form.querySelector === 'function'
            ? form.querySelector('input:not(:disabled), textarea:not(:disabled), select:not(:disabled)')
            : null
        if (!target || target.isConnected === false || typeof target.focus !== 'function') return
        try { target.focus({ preventScroll: true }) } catch { try { target.focus() } catch {} }
      }, 0)
    }

    /** derived HoursField：两个原生时间控件，Field 的 label 只关联起始控件。 */
    function HoursField({ value, onChange, id, disabled }) {
      const match = String(value || '').match(/^(\d{1,2}):(\d{2})\s*-\s*(\d{1,2}):(\d{2})$/)
      const start = match ? String(match[1]).padStart(2, '0') + ':' + match[2] : ''
      const end = match ? String(match[3]).padStart(2, '0') + ':' + match[4] : ''
      const write = (nextStart, nextEnd) => onChange(nextStart && nextEnd ? nextStart + '-' + nextEnd : '')
      const overnight = Boolean(start && end && end <= start)
      return h('div', { className: 'dt-hours' },
        h('input', {
          id, type: 'time', className: 'dt-time', value: start, 'aria-label': '起始时间', disabled,
          onChange: (event) => write(event.target.value, end || '23:00'),
        }),
        h('span', { className: 'dt-hours-sep' }, '→'),
        h('input', {
          id: id ? id + '-end' : undefined, type: 'time', className: 'dt-time', value: end, 'aria-label': '结束时间', disabled,
          onChange: (event) => write(start || '09:00', event.target.value),
        }),
        value
          ? h('button', {
            type: 'button', className: 'dt-btn dt-btn-sm dt-btn-ghost', disabled,
            onClick: () => onChange(''),
          }, '全天')
          : h('span', { className: 'dt-hours-hint' }, '留空 = 全天'),
        overnight ? h('span', { className: 'dt-hours-hint' }, '跨夜窗口') : null)
    }

    function WatcherSwitch({ spec, value, onChange, disabled, id }) {
      return h('div', { className: 'dt-switch-row' },
        h('div', { className: 'dt-field-head' }, h('span', null, spec.label), spec.hint ? h(InfoTip, { text: spec.hint }) : null),
        h(Toggle, { id, checked: !!value, onChange, label: spec.label, disabled }),
        // 隔离的回收语义原本是可见说明，不能因迁移到 SwitchRow 而藏进 tooltip。
        spec.key === 'isolated' ? h('span', { className: 'dt-hint' }, spec.hint) : null)
    }

    function WatcherPriority({ value, onChange, disabled, idPrefix }) {
      const items = [
        ['priorityReopened', '重开优先', '被重开的排最前'],
        ['priorityComments', '评论优先', '有人在等回应的靠前'],
        ['priorityLabels', '标签优先', '带标签的靠前'],
        ['priorityUnassigned', '未认领优先', '没人认领的靠前（默认关：这是我们的判断，不是规则）'],
        ['priorityStale', '积压优先', '躺了 30 天以上的靠前（默认关，只展示）'],
      ]
      return h('div', { className: 'dt-actions' },
        h('div', { className: 'dt-field-head' }, h('span', null, PRIORITY_LABEL),
          h(InfoTip, { text: '全部关掉时按 GitHub 的「最近更新」顺序处理' })),
        h('div', { className: 'dt-actions' }, items.map(([key, label, hint]) => h('div', { className: 'dt-switch-row', key },
          h('div', { className: 'dt-field-head' }, h('span', null, hint)),
          h(Toggle, { id: idPrefix + '-' + key, checked: key === 'priorityUnassigned' || key === 'priorityStale' ? value[key] === true : value[key] !== false,
            onChange: (next) => onChange(key, next), label, disabled })))))
    }

    function WatcherForm({ mode = 'edit', idPrefix = 'watcher', value, snapshot, defaults = {}, disabled = false,
      onChange, onSave, onDiscard, submitLabel = '保存', sessionId }) {
      const [advancedOpen, setAdvancedOpen] = useState(false)
      const [busy, setBusy] = useState(false)
      const [error, setError] = useState(null)
      const busyRef = useRef(false)
      const formRef = useRef(null)
      const formFocusedRef = useRef(false)
      const values = value || {}
      const opening = snapshot || values
      const dirty = watcherDirty(values, opening)
      const change = (key, next) => { setError(null); onChange(key, next) }
      const captureFocus = () => {
        const form = formRef.current
        const active = typeof document === 'undefined' ? null : document.activeElement
        const ticket = mode === 'edit' && form && typeof form.closest === 'function' ? form.closest('.dt-ticket') : null
        return {
          form,
          saveBar: form && typeof form.querySelector === 'function' ? form.querySelector('.dt-savebar') : null,
          opener: ticket && typeof ticket.querySelector === 'function' ? ticket.querySelector('.dt-open') : null,
          wasFocused: formFocusedRef.current || Boolean(form && active && typeof form.contains === 'function' && form.contains(active)),
          shouldRestore: () => formFocusedRef.current,
        }
      }
      const submit = async (event) => {
        if (event && typeof event.preventDefault === 'function') event.preventDefault()
        // 原生点击保存时仍执行 constraints；测试桩没有 form，直接走同一提交函数。
        const form = event && event.currentTarget && event.currentTarget.form
        if (form && typeof form.checkValidity === 'function' && !form.checkValidity()) {
          if (typeof form.reportValidity === 'function') form.reportValidity()
          const invalid = typeof form.querySelector === 'function' ? form.querySelector(':invalid') : null
          if (invalid && typeof invalid.focus === 'function') invalid.focus()
          return false
        }
        if (busyRef.current || disabled || (mode === 'edit' && !dirty)) return false
        if (mode === 'create' && (!sessionId || !String(values.repo || '').trim())) return false
        const focus = captureFocus()
        busyRef.current = true
        setBusy(true)
        setError(null)
        try {
          const result = await onSave(values)
          if (result === false) throw new Error('保存失败，请重试')
          restoreSaveBarFocus(focus)
          return result
        } catch (failure) {
          setError(String((failure && failure.message) || failure || '保存失败，请重试'))
          return false
        } finally {
          busyRef.current = false
          setBusy(false)
        }
      }
      const discard = () => {
        if (busyRef.current) return
        const focus = captureFocus()
        setError(null)
        onDiscard(opening)
        restoreSaveBarFocus({ ...focus, opener: null })
      }
      const renderField = (spec) => {
        if (spec.modes && !spec.modes.includes(mode)) return null
        if (spec.when && !spec.when(values)) return null
        const id = idPrefix + '-' + spec.key
        if (spec.kind === 'switch') {
          return h(WatcherSwitch, { key: spec.key, spec, value: values[spec.key], disabled: busy || disabled, id,
            onChange: (next) => change(spec.key, next) })
        }
        if (spec.kind === 'priority') {
          return h(WatcherPriority, { key: spec.key, value: values, disabled: busy || disabled, idPrefix,
            onChange: change })
        }
        const common = {
          value: spec.kind === 'hours' ? (values[spec.key] || '') : watcherNumberText(values[spec.key]),
          disabled: busy || disabled,
          onChange: (next) => change(spec.key, next),
        }
        const control = spec.kind === 'hours'
          ? h(HoursField, common)
          : h(Input, {
            type: spec.kind === 'number' ? 'number' : 'text',
            value: common.value,
            min: spec.min, max: spec.max, step: spec.kind === 'number' ? 1 : undefined,
            required: !!spec.required,
            placeholder: typeof spec.placeholder === 'function' ? spec.placeholder(defaults) : spec.placeholder,
            disabled: busy || disabled,
            onChange: (event) => change(spec.key, event.target.value),
          })
        return h(Field, { key: spec.key, id, label: spec.label, hint: spec.hint, note: spec.note }, control)
      }
      const basic = WATCHER_FIELD_SCHEMA.filter((spec) => !spec.advanced).map(renderField)
      const advanced = WATCHER_FIELD_SCHEMA.filter((spec) => spec.advanced).map(renderField)
      const collapsedAdvanced = h('span', { className: 'dt-row-meta' },
        h('span', { className: 'dt-hint' }, '限额、优先级与隔离保留策略'),
        // 收起时只展示默认值摘要，不挂载高级 input；默认值仍来自 /state.defaults。
        WATCHER_FIELD_SCHEMA.filter((spec) => spec.advanced && spec.kind === 'number').map((spec) => {
          const placeholder = typeof spec.placeholder === 'function' ? spec.placeholder(defaults) : spec.placeholder
          return h('span', { key: spec.key, className: 'dt-hint', placeholder }, spec.label + ' ' + placeholder)
        }))
      const submitFromClick = (event) => submit(event)
      return h('form', {
        id: idPrefix, ref: formRef, className: 'dt-form dt-card', 'data-mode': mode, onSubmit: submit,
        onFocusCapture: () => { formFocusedRef.current = true },
        onBlurCapture: (event) => {
          const next = event.relatedTarget
          // disabled 让当前控件失焦时 relatedTarget 可能为空；保存 pending 不应因此丢掉回焦资格。
          if ((!next || !event.currentTarget.contains(next)) && !(busyRef.current && !next)) formFocusedRef.current = false
        },
      },
        h(Fields, { disabled: busy || disabled },
          basic,
          h(DisclosureRow, {
            icon: h(Icon, { name: 'settings' }), title: '高级设置', open: advancedOpen,
            busy, disabled,
            onToggle: () => { if (!busyRef.current && !busy && !disabled) setAdvancedOpen((current) => !current) },
            collapsedContent: collapsedAdvanced, bodyId: idPrefix + '-advanced-body',
          }, h('div', { className: 'dt-disclosure-body' }, advanced))),
        h(SaveBar, { dirty, busy, error, disabled, onDiscard: discard, onSave: submitFromClick, saveLabel: submitLabel }),
        // 新建空白表单也要给入口；开始输入后由 SaveBar 接管，不在屏幕上重复两个提交按钮。
        mode === 'create' && !dirty && !busy && !error
          ? h('div', { className: 'dt-actions' }, h(Btn, {
            variant: 'primary', icon: h(Icon, { name: 'plus' }), type: 'submit', onClick: submitFromClick,
            'aria-label': submitLabel, disabled: disabled || !String(values.repo || '').trim(),
          }, submitLabel))
          : null)
    }

    function KeyValues({ rows }) {
      return h('dl', { className: 'dt-kv' },
        rows.filter(Boolean).flatMap(([key, value]) => [
          h('dt', { key: key + '-k' }, key),
          h('dd', { key: key + '-v' }, value),
        ]))
    }

    // 官方 Modal 的键盘行为：Escape 关闭 + Tab 焦点陷阱 + 关闭后焦点回到触发元素。
    // 移植时只搬了样式，行为是后补的（2026-11-02 对官方 controls.tsx 逐行比对时发现）。
    // 本地 Modal 打开层数：Modal 存在时 Escape 属于最上层对话框，底层 Orbit 的 capture 必须让位（W16）。
    // 用模块级计数而不是给 StatusOrbit 加 hook，保持它按位置取值的 hook 契约不变。
    let localDialogDepth = 0
    const FOCUSABLE = 'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
    function trapTab(event, dialog) {
      const items = [...dialog.querySelectorAll('*')].filter((element) => element.matches(FOCUSABLE))
      const first = items[0]
      const last = items[items.length - 1]
      const active = document.activeElement
      const target = event.shiftKey ? last : first
      if (!target || !dialog.contains(active) || active === dialog || active === (event.shiftKey ? first : last)) {
        event.preventDefault()
        target.focus()
      }
    }

    function Modal({ open, title, description, footer, className, children, onClose }) {
      const dialog = useRef(null)
      const restoreTo = useRef(null)
      // 与官方 controls.tsx 一致：回调放 ref，父组件轮询重渲染不能重建 effect、抢走弹层里的焦点。
      const close = useRef(onClose)
      close.current = onClose
      useEffect(() => {
        if (!open || typeof document === 'undefined') return undefined
        localDialogDepth += 1
        restoreTo.current = document.activeElement
        // 打开后把焦点挪进对话框，否则键盘用户还停在背后的页面上
        const timer = setTimeout(() => {
          const node = dialog.current
          if (!node) return
          const first = [...node.querySelectorAll('*')].filter((el) => el.matches(FOCUSABLE))[0]
          ;(first || node).focus()
        }, 0)
        const onKey = (event) => {
          if (event.key === 'Escape') {
            // 已被更上层控件（如 fallback Tooltip）消费的 Escape 不再关 Modal。
            if (event.defaultPrevented) return
            event.preventDefault(); close.current(); return
          }
          if (event.key === 'Tab' && dialog.current) trapTab(event, dialog.current)
        }
        document.addEventListener('keydown', onKey)
        return () => {
          localDialogDepth = Math.max(0, localDialogDepth - 1)
          clearTimeout(timer)
          document.removeEventListener('keydown', onKey)
          const back = restoreTo.current
          if (back && typeof back.focus === 'function') { try { back.focus() } catch {} }
        }
      }, [open])
      if (!open) return null
      // class 必须与移植来的官方 CSS 对齐：dt-modal-root / dt-modal-mask / dt-modal-header
      return h('div', { className: 'dt-modal-root' },
        h('div', { className: 'dt-modal-mask', role: 'presentation', onClick: onClose }),
        h('div', {
          ref: dialog, tabIndex: -1,
          className: cx('dt-modal', className), role: 'dialog', 'aria-modal': 'true',
          'aria-label': typeof title === 'string' ? title : undefined,
        },
          h('div', { className: 'dt-modal-header' },
            h('span', { className: 'dt-title' }, title),
            h('button', { type: 'button', className: 'dt-modal-close', onClick: onClose, 'aria-label': '关闭' }, h(Icon, { name: 'close' }))),
          description ? h('p', { className: 'dt-modal-description' }, description) : null,
          children !== undefined && children !== null ? h('div', { className: 'dt-modal-body' }, children) : null,
          footer !== undefined ? h('div', { className: 'dt-modal-footer' }, footer) : null))
    }

    // 主面板使用 DSH Modal；没有宿主原语时回退到上面的 superagent 同源实现。
    // 直接别名而不是 h(Component, props) 转发，避免测试桩与真 React 的 children 处理差异。
    const PanelModal = hostUi ? hostUi.Modal : Modal

    /** 确认层与触发器分离：DSH orbit 关闭后，同一个本地 Modal 仍能由 StatusOrbit 持有。 */
    function ConfirmDialog({ open, label, confirmLabel = '确定执行', title = confirmLabel, message, onConfirm, onClose, modalContext = 'panel', ButtonComponent = Btn, danger = true, preventDismissOnBusy = false, error = null, busyLabel = confirmLabel, busyHint = '安装进行中，完成后才能关闭' }) {
      const [busy, setBusy] = useState(false)
      const busyRef = useRef(false)
      const dismiss = () => {
        if (preventDismissOnBusy && (busy || busyRef.current)) return false
        onClose()
        return true
      }
      const confirm = async () => {
        // state 还没来得及重渲染时也可能连点；同步 ref 是第二道锁。
        if (busyRef.current) return false
        busyRef.current = true
        setBusy(true)
        try {
          const done = await onConfirm()
          // 成功关闭走原回调，只执行一次，不能被 busy dismiss guard 挡住。
          if (done) onClose()
          return done
        } finally {
          busyRef.current = false
          setBusy(false)
        }
      }
      // orbit 删除明确保留 superagent 同源 Modal；普通主面板删除仍走 PanelModal，不换 RiskConfirmation。
      const ModalView = modalContext === 'orbit' ? Modal : PanelModal
      const feedback = error || (preventDismissOnBusy && busy)
        ? h(React.Fragment, null,
          error ? h('div', { className: 'dt-error', role: 'alert' }, error) : null,
          preventDismissOnBusy && busy ? h(Hint, null, busyHint) : null)
        : null
      return h(ModalView, {
        open,
        title,
        description: message || ('确定要执行「' + label + '」操作吗？'),
        onClose: dismiss,
        footer: h(React.Fragment, null,
          h(ButtonComponent, { variant: 'ghost', disabled: preventDismissOnBusy && busy, onClick: dismiss }, '取消'),
          h(ButtonComponent, {
            variant: 'primary',
            className: cx(danger && 'dt-danger', busy && 'dt-btn-busy'),
            disabled: busy,
            'aria-label': confirmLabel,
            'aria-busy': busy ? 'true' : undefined,
            onClick: confirm,
          }, busy ? busyLabel : confirmLabel)),
      }, feedback)
    }

    /** 官方 ConfirmButton 动线：触发按钮只开 Modal，取消不执行，确认期间 busy 防重复。 */
    function ConfirmButton({ label, confirmLabel = '确定执行', title = confirmLabel, message, onConfirm, variant = 'ghost', icon, size, modalContext = 'panel', ButtonComponent = Btn, danger = true, preventDismissOnBusy = false, error = null, busyLabel = confirmLabel, busyHint, onOpen }) {
      const [open, setOpen] = useState(false)
      const close = useCallback(() => setOpen(false), [])
      return h(React.Fragment, null,
        h(ButtonComponent, {
          variant,
          size,
          icon,
          'aria-label': label,
          'aria-haspopup': 'dialog',
          onClick: () => {
            // 打开前先跑调用方的清理（例如清掉上一次「删除失败」）；打开行为本身不变。
            if (typeof onOpen === 'function') onOpen()
            setOpen(true)
          },
        }, label),
        h(ConfirmDialog, { open, label, confirmLabel, title, message, onConfirm, onClose: close, modalContext, ButtonComponent, danger, preventDismissOnBusy, error, busyLabel, busyHint }))
    }

    const watcherId = (w) => (w && w.id) || String((w && w.repo) || '') + '#' + String((w && w.sessionId) || '')
    const orbitActionKey = (w, action) => JSON.stringify([watcherId(w), action])
    const short = (value, width = 19) => (typeof value === 'string' && value.length > width ? value.slice(0, width - 1) + '…' : (value ?? '—'))
    const when = (iso) => (typeof iso === 'string' ? iso.replace('T', ' ').replace('Z', '').slice(5, 19) : '—')

    // ── orbit badge: session header indicator ──────────────────────────────
    // Static geometry, three states: green dot orbiting (watching), amber dot on
    // a dashed ring (paused), red dot breathing (error). Icon only — the text
    // lives in the native tooltip so it does not compete with the session title.
    // ── status orbit: composer-dock indicator ─────────────────────────────
    // One ring for the whole machine. Every satellite on the ring is one
    // session-level superagent (a live watcher): green = watching, amber =
    // paused, red = last poll failed. Icon only — details live in the
    // click-through panel, and a native tooltip carries the one-line summary.
    /**
     * 现在是否落在活跃时段里。支持跨夜（22:00-06:00）。
     * 解析不了就返回 null —— 宁可显示「主理中」，也不要乱标。
     */
    function inWindowNow(hours) {
      const match = String(hours || '').match(/^(\d{1,2}):(\d{2})\s*-\s*(\d{1,2}):(\d{2})$/)
      if (!match) return null
      const now = new Date()
      const current = now.getHours() * 60 + now.getMinutes()
      const from = Number(match[1]) * 60 + Number(match[2])
      const to = Number(match[3]) * 60 + Number(match[4])
      return from <= to ? current >= from && current < to : current >= from || current < to
    }

    function orbitStateOf(w) {
      if (!w) return 'paused'
      // 历史数据里可能残留「gh: HTTP 304」这类不是错误的记录 —— 不把它当异常
      if (w.lastError && !/HTTP 304|304 Not Modified/i.test(String(w.lastError))) return 'error'
      if (w.enabled === false) return 'paused'
      // 设了活跃时段、而此刻在窗口外：它其实不会唤醒，只是把条目攒着 ——
      // 这时候还标「主理中」就是在骗人（2026-11-02 用户截图指出）。
      if (w.activeHours && inWindowNow(w.activeHours) === false) return 'dormant'
      return 'active'
    }

    // 状态色一律走 token（写死 hex 在深色/浅色两套主题里会有一个是错的）
    const SAT_COLOR = {
      active: 'var(--dsw-alias-state-success-primary)',
      paused: 'var(--dsw-alias-state-warn-primary)',
      error: 'var(--dsw-alias-state-error-primary)',
      dormant: 'var(--dsw-alias-label-caption)',
    }
    const STATE_TEXT = { active: '主理中', paused: '已暂停', error: '异常', dormant: '时段外' }

    /** 尊重系统的「减少动态效果」设置。 */
    const prefersReducedMotion = () => {
      try { return window.matchMedia('(prefers-reduced-motion: reduce)').matches } catch { return false }
    }

    function OrbitMark(props) {
      const watchers = props.watchers || []
      const shown = watchers.slice(0, 6)
      const states = watchers.map(orbitStateOf)
      const anyActive = states.indexOf('active') >= 0
      // Zero watchers still draws the ring, just grey: an idle machine should
      // read as "nothing being watched", not as "the plugin is broken".
      const empty = states.length === 0
      const ringColor = empty
        ? 'currentColor'
        : states.indexOf('error') >= 0
          ? SAT_COLOR.error
          : anyActive ? SAT_COLOR.active : states.indexOf('dormant') >= 0 ? SAT_COLOR.dormant : SAT_COLOR.paused
      const dashed = states.indexOf('paused') >= 0
      const still = prefersReducedMotion()

      const dots = shown.map((w, i) => {
        const angle = (i / Math.max(shown.length, 1)) * Math.PI * 2 - Math.PI / 2
        const state = orbitStateOf(w)
        return h('circle', {
          key: String(w.repo || '') + '#' + String(w.sessionId || i),
          cx: 8 + Math.cos(angle) * 6.1,
          cy: 8 + Math.sin(angle) * 6.1,
          r: shown.length > 3 ? 1.75 : 2.05,
          fill: SAT_COLOR[state],
        }, state === 'error' && !still
          ? h('animate', { attributeName: 'opacity', values: '1;.3;1', dur: '1.6s', repeatCount: 'indefinite' })
          : null)
      })

      // 动画用 SMIL 写进 SVG 本身：不依赖注入的 <style>，也不吃 CSP /
      // transform-box 的差异；单独存成 .html 就能预览。
      const spin = anyActive && !still
        ? h('animateTransform', {
          attributeName: 'transform', type: 'rotate',
          from: '0 8 8', to: '360 8 8', dur: '6s', repeatCount: 'indefinite',
        })
        : null
      const breathe = anyActive && !still
        ? h('animate', { attributeName: 'opacity', values: '.95;.45;.95', dur: '2.6s', repeatCount: 'indefinite' })
        : null

      return h('svg', {
        width: 16, height: 16, viewBox: '0 0 16 16', 'aria-hidden': 'true',
        style: { overflow: 'visible' },
      },
        h('circle', {
          cx: 8, cy: 8, r: 6.1, fill: 'none', stroke: ringColor, strokeWidth: 1.1,
          strokeDasharray: dashed ? '2.2 1.9' : undefined,
          opacity: empty ? 0.38 : 0.9,
        }, breathe),
        h('g', null, spin, dots))
    }

    const notedOnce = new Set()
    /** Diagnostics: write once per page load, never on every poll. */
    function noteOnce(key, value) {
      if (notedOnce.has(key)) return
      notedOnce.add(key)
      note(key, value)
    }

    function note(key, value) {
      try { fetch(BASE + '/client-note?key=' + encodeURIComponent(key) + '&value=' + encodeURIComponent(String(value || ''))).catch(() => {}) } catch {}
    }

    /** 8-char session tag. Titles come from the title model and several sessions
     *  can share one, so every row carries the id suffix to stay unambiguous. */
    const sessionTag = (id) => {
      const raw = String(id || '')
      const core = raw.indexOf('session-') === 0 ? raw.slice(8) : raw
      return core.slice(0, 8) || '—'
    }

    /** 把 filters 压成一行人话，空配置返回空串。 */
    const filterText = (f) => {
      if (!f) return ''
      const parts = []
      if (f.labels && f.labels.length) parts.push('标签: ' + f.labels.join(','))
      if (f.ignoreLabels && f.ignoreLabels.length) parts.push('排除: ' + f.ignoreLabels.join(','))
      if (f.onlyAssigned) parts.push(f.assignee ? '只看指派给 @' + f.assignee : '只看已指派')
      // 设了作者白名单就要能自查；只在编辑器里出现，列表上看不出「到底生效的是谁」。
      if (f.allowAuthors && f.allowAuthors.length) parts.push('作者: ' + f.allowAuthors.join(','))
      return parts.join(' · ')
    }

    /** 只留路径最后两段，面板里放得下。 */
    const pathTail = (value) => String(value || '')
      .split(String.fromCharCode(92)).join('/')
      .split('/').filter(Boolean).slice(-2).join('/')

    /** 隔离执行状态：开着隔离就显示上次用的 worktree。 */
    const isolationText = (w) => {
      if (!w || !w.isolated) return ''
      return '隔离 · ' + (pathTail(w.lastWorktree) || pathTail(w.workdir) || '未建')
    }

    /** 最近一次唤醒的结果（面板第二行用）。 */
    const historyText = (w) => {
      const list = (w && w.history) || []
      const last = list[list.length - 1]
      if (!last) return ''
      const nums = (last.numbers || []).map((n) => '#' + n).join(' ')
      return (last.ok ? '上次唤醒 ' : '上次唤醒失败 ') + nums + ' · ' + ageText(last.at)
    }

    /** 失败退避中显示剩余秒数，否则空串。 */
    const backoffText = (w) => {
      if (!w || !w.nextAt) return ''
      const left = Date.parse(w.nextAt) - Date.now()
      if (!isFinite(left) || left <= 0) return ''
      return '退避 ' + Math.ceil(left / 1000) + 's'
    }

    const ageText = (iso) => {
      if (typeof iso !== 'string') return '—'
      const ms = Date.now() - Date.parse(iso)
      if (!isFinite(ms) || ms < 0) return '—'
      const s = Math.round(ms / 1000)
      if (s < 90) return s + ' 秒前'
      if (s < 5400) return Math.round(s / 60) + ' 分钟前'
      return Math.round(s / 3600) + ' 小时前'
    }
    const wakeText = (value) => {
      if (typeof value !== 'string' || value === '') return '—'
      if (value.indexOf('failed') === 0) return '失败'
      return ageText(value)
    }

    const PANEL_W = 368
    const BORDER = '1px solid var(--dsw-alias-border-l2, rgba(128,128,128,.22))'

    /** Dock entry: the machine-wide ring plus its click-through detail list. */
    /** 把错误拆成「现象」和「建议」：服务端存的是 short +（hint），建议在末尾的括号里。 */
    // 唤醒失败 vs 轮询失败：前者轮询是成功的，说成「轮询失败」会让客户去查网络
    // KB: paused-means-no-new-facts.md —— 暂停=不再产生新事实（不要拿旧数据冒充当前状态）
    function isWakeFailure(text) { return /^wake:/.test(String(text || '')) }

    // KB: error-must-have-next-step.md —— 每个错误都要带下一步（现象与建议分开存）
    // KB: error-needs-a-button.md —— 能一键解决的错误，不要只给一句话
    function splitErrorText(text) {
      const value = String(text || '')
      const match = /（[^（）]*）\s*$/.exec(value)
      if (!match) return { main: value, hint: '' }
      return { main: value.slice(0, match.index).trim(), hint: match[0].trim() }
    }
    function StatusOrbit() {
      const [watchers, setWatchers] = useState(null)
      // 写操作的失败提示（M4）：为空表示没有错误
      const [orbitError, setOrbitError] = useState(null)
      const [open, setOpen] = useState(false)
      const [anchor, setAnchor] = useState(null)
      // 品牌名（issue #11）：/state 下发；老服务端没有这个字段时仍是「主理鱼」。
      // 只能在末尾追加状态槽，前面的位置契约（观测/删除确认）保持不变。
      const [brandName, setBrandName] = useState('主理鱼')
      const btnRef = useRef(null)
      const panelRef = useRef(null)
      const actionLocks = useRef(new Set())
      const stateSequence = useRef(0)
      const orbitAlive = useRef(true)
      const orbitConfirm = useOrbitConfirm({
        orbitRef: btnRef,
        onCloseOrbit: () => setOpen(false),
        // DSH 路径复用旧 anchor 槽保存待删 watcher；fallback 仍只保存手算坐标，状态槽位不变。
        // 打开时先清掉上一次的「删除失败：…」，否则新确认框一打开就带着别人的错误。
        onRequestConfirm: ({ watcher }) => openDeleteConfirm(watcher),
      })

      const isReadError = (value) => typeof value === 'string'
        && (value.startsWith('读取失败：') || value.startsWith('写成功了，但刷新列表失败：'))
      // 定时轮询与写后回读共用同一个代际：只允许最后一次请求改 watchers/读取错误。
      const loadState = async (failurePrefix = '读取失败：') => {
        const sequence = ++stateSequence.current
        try {
          const body = await call('/state')
          if (!orbitAlive.current || sequence !== stateSequence.current) return { applied: false, ok: true }
          setWatchers(Array.isArray(body && body.watchers) ? body.watchers : [])
          setBrandName((typeof (body && body.brandName) === 'string' && body.brandName.trim()) || '主理鱼')
          setOrbitError((current) => isReadError(current) ? null : current)
          return { applied: true, ok: true }
        } catch (error) {
          if (orbitAlive.current && sequence === stateSequence.current) {
            setOrbitError(failurePrefix + String((error && error.message) || error).slice(0, 160))
          }
          return { applied: orbitAlive.current && sequence === stateSequence.current, ok: false }
        }
      }

      // 弹层的样式（DOCK_CSS）挂在 PORTED_CSS 同一张 <style> 里，而那张表以前只有「主理鱼」面板挂载时才注入。
      // 用户从没打开过面板就直接点状态栏圆环时，弹层就是一堆无样式的裸文字（真机复现过）。
      useEffect(() => { ensurePortedCss() }, [])

      useEffect(() => {
        orbitAlive.current = true
        noteOnce('orbit-mount', 'ok')
        const load = () => loadState()
        load()
        const timer = setInterval(load, POLL_MS)
        return () => { orbitAlive.current = false; clearInterval(timer) }
      }, [])

      // 弹层每次打开都把样式表钉一遍：宿主热重载可能把 <style> 清掉，
      // 而弹层是"一开就要好看"的东西，不能等到下次渲染才发现没样式
      useEffect(() => { if (open) ensurePortedCss(true) }, [open])

      // 终审 N2：删除失败只属于「那一次确认框」。打开新的删除确认时必须先清掉它，
      // 否则 A 的失败会跟着 B 的确认框一起出现（B 还没发任何请求）。读取错误属于列表状态，不能一起清。
      const clearDeleteError = () => setOrbitError((current) =>
        typeof current === 'string' && current.startsWith('删除失败：') ? null : current)
      const openDeleteConfirm = (watcher) => {
        // 先选中待删 watcher（保持原有 state 调用顺序），再清旧错误；两者同批渲染。
        setAnchor(watcher)
        clearDeleteError()
      }

      useEffect(() => {
        // 仅旧路径保留 capture mousedown / 手算焦点；DSH 由 OrbitPopover 与宿主 dismiss hook 接管。
        if (!open || hostOrbit) return undefined
        const onDown = (event) => {
          // 本地确认 Modal 在最上层时，点背板/取消不能穿透到 Orbit（W16）。
          if (localDialogDepth > 0) return
          const target = event.target
          if (panelRef.current && panelRef.current.contains(target)) return
          if (btnRef.current && btnRef.current.contains(target)) return
          setOpen(false)
        }
        const onKey = (event) => {
          if (event.key !== 'Escape' || event.repeat) return
          // 第一次 Escape 只关 Modal；Modal 的 keydown 在 bubble 阶段处理并归还焦点（W16）。
          if (localDialogDepth > 0) return
          setOpen(false)
        }
        document.addEventListener('mousedown', onDown, true)
        document.addEventListener('keydown', onKey, true)
        // 键盘可达：打开后把焦点送进弹层（Tab 才能顺序走进去），关掉时还给圆环按钮。
        // 官方 Modal 就是这么做的（见上面 Modal 的 Escape + 焦点归还），弹层同样适用。
        const focusTimer = setTimeout(() => {
          const panel = panelRef.current
          if (panel && typeof panel.focus === 'function') panel.focus()
        }, 0)
        return () => {
          clearTimeout(focusTimer)
          document.removeEventListener('mousedown', onDown, true)
          document.removeEventListener('keydown', onKey, true)
          const button = btnRef.current
          if (button && typeof button.focus === 'function') button.focus()
        }
      }, [open])

      const list = watchers || []
      const readError = isReadError(orbitError) ? orbitError : null
      // Never hide the entry: with nothing being watched it stays as a grey
      // ring, so the affordance is always findable in the dock row.
      const running = list.filter((w) => orbitStateOf(w) === 'active').length
      const broken = list.filter((w) => orbitStateOf(w) === 'error').length
      const summary = readError && watchers === null
        ? '读不到' + brandName + '状态'
        : list.length === 0
          ? '还没有' + brandName
          : '会话级 superagent：' + running + ' 运行中'
            + (broken ? '，' + broken + ' 异常' : '')
            + '（共 ' + list.length + ' 条）'

      const toggle = () => {
        if (!hostOrbit) {
          const el = btnRef.current
          if (el) {
            const rect = el.getBoundingClientRect()
            setAnchor({ x: rect.left + rect.width / 2, y: rect.top })
          }
        }
        setOpen((value) => !value)
      }

      const button = h('button', {
        ref: btnRef,
        type: 'button',
        title: readError && watchers === null
          ? readError
          : list.length
            ? list.map((w) => (w.repo + ' · ' + (w.title || '未命名会话') + ' · ' + sessionTag(w.sessionId) + ' · ' + STATE_TEXT[orbitStateOf(w)])).join('\n')
            : '还没有' + brandName + ' —— 在「' + brandName + '」页加一条',
        'aria-label': summary,
        'aria-expanded': open ? 'true' : 'false',
        className: cx('dt-btn', 'dt-btn-ghost', 'sa-orbit-trigger', watchers === null && 'is-loading'),
        onClick: toggle,
      }, h(OrbitMark, { watchers: list }))

      // DSH 固定业务宽度 252/368，坐标全部交给 useAnchoredPosition；只有 fallback 保留视口夹紧与手算 bottom。
      const viewport = Number(window && window.innerWidth) || 1200
      const width = hostOrbit
        ? (list.length === 0 ? 252 : PANEL_W)
        : Math.min(list.length === 0 ? 252 : PANEL_W, Math.max(240, viewport - 16))
      const left = !hostOrbit && anchor
        ? Math.max(8, Math.min(anchor.x - width / 2, (window.innerWidth || 1200) - width - 8))
        : 8
      const bottom = !hostOrbit && anchor ? Math.max(8, (window.innerHeight || 800) - anchor.y + 8) : 64

      const act = async (w, action) => {
        const key = orbitActionKey(w, action)
        if (actionLocks.current.has(key)) return false
        actionLocks.current.add(key)
        // 不新增根 useState：旧 fallback 的位置化 hook 契约必须保持不变。
        // 新数组只触发一次渲染，watcher 内容不变；可见 pending 直接读取同步锁集合。
        setWatchers((current) => Array.isArray(current) ? [...current] : current)
        if (action === 'delete') {
          clearDeleteError()
        }
        try {
          // 删除只走 ConfirmButton + Modal；不要再叠一层原生确认框（同一动作问两次）。
          try {
            await call('/watch', { method: 'POST', body: JSON.stringify({ action, id: watcherId(w) }) })
          } catch (error) {
            const message = String((error && error.message) || error).slice(0, 160)
            setOrbitError(action === 'delete' ? '删除失败：' + message : message)
            return false
          }
          // 写已成功：清掉上一代操作/读取错误；回读失败会由 loadState 写入新的可见错误。
          setOrbitError(null)
          await loadState('写成功了，但刷新列表失败：')
          // 刷新失败不等于删除失败：动作已成功，确认框仍应关闭。
          return true
        } finally {
          actionLocks.current.delete(key)
          setWatchers((current) => Array.isArray(current) ? [...current] : current)
        }
      }

      // fallback 完全保留原提前返回；DSH 即使 orbit 已关，也要继续渲染由本组件持有的确认 Modal。
      if (!open && !hostOrbit) return button
      const closeConfirm = () => {
        // 关掉确认框等于放弃这一次删除：错误跟着走，不要留在弹层里冒充「刚发生的失败」。
        clearDeleteError()
        setAnchor(null)
        orbitConfirm.onConfirmClose()
      }
      const deleteError = typeof orbitError === 'string' && orbitError.startsWith('删除失败：') ? orbitError : null
      const actionPending = (w, action) => actionLocks.current.has(orbitActionKey(w, action))

      // 悬浮面板也走移植来的 dt-* 设计系统（以前这里是一套自成体系的 inline style，
      // 和原生界面放在一起能看出断层；ui-audit ⑥ 就是在盯这件事）。
      // 摘要行（官方弹层的写法）：主名一行 + 一行 muted 细节，细节超长就省略号。
      // 以前每条守最多 5 行提示挤在一起（goal / 时段 / 隔离 / 错误 / 历史），弹层读不动。
      // 弹层里先给人看的名字：会话标题（客户自己起的），没有标题才退回短 ID。
      // 以前只放 8 位 ID —— 客户认不出这是哪个会话。
      const whoName = (w) => String(w.title || '').trim() || ('会话 ' + sessionTag(w.sessionId))
      const whoId = (w) => (String(w.title || '').trim() ? ' · #' + sessionTag(w.sessionId) : '')
      const shortSub = (w) => {
        const parts = [whoName(w) + whoId(w), '每 ' + (Number(w.intervalSec) || 60) + 's']
        // ageText 自己已经带「前」（「76 秒前」），别再拼一个 —— 真机上显示成「76 秒前前」
        if (w.lastPoll) parts.push(ageText(w.lastPoll))
        if (w.activeHours) parts.push('时段 ' + w.activeHours)
        return parts.join(' · ')
      }
      const detailTitle = (w) => [
        w.title || '未命名会话',
        '会话 ' + sessionTag(w.sessionId),
        filterText(w.filters),
        isolationText(w),
        backoffText(w),
        historyText(w),
        w.lastDeferred,
        w.goal && w.goal.objective ? 'goal · ' + String(w.goal.objective).slice(0, 80) : '',
        w.lastError || '',
      ].filter(Boolean).join('\n')

      const noRowToggle = () => {}
      const rows = list.map((w) => {
        const state = orbitStateOf(w)
        const dotState = state === 'active' ? 'done' : state === 'paused' ? 'warning' : state === 'error' ? 'error' : 'idle'
        const tone = state === 'active' ? 'success' : state === 'paused' ? 'neutral' : state === 'error' ? 'danger' : 'quiet'
        const key = String(w.repo || '') + '#' + String(w.sessionId || '')
        const statusDot = h('span', { className: 'dt-state-dot', 'data-state': dotState })
        const statusTag = h('span', { className: 'dt-tag', 'data-tone': tone }, STATE_TEXT[state])
        const pollPending = actionPending(w, 'poll')
        const toggleAction = w.enabled ? 'pause' : 'resume'
        const togglePending = actionPending(w, toggleAction)
        const detailContent = h(React.Fragment, null,
          h('div', { className: 'sa-dock-sub' }, shortSub(w)),
          w.lastError
            ? h('div', { className: 'sa-dock-err' }, splitErrorText(w.lastError).main)
            : null)
        const details = h('div', { className: 'sa-dock-body' }, detailContent)
        const actions = h('div', { className: 'sa-dock-actions' },
          h(Btn, {
            size: 'sm', variant: 'ghost', disabled: pollPending,
            className: cx(pollPending && 'dt-btn-busy'), 'aria-busy': pollPending ? 'true' : undefined,
            onClick: () => act(w, 'poll'),
          }, pollPending ? '轮询中…' : '立刻'),
          h(Btn, {
            size: 'sm', variant: 'ghost', disabled: togglePending,
            className: cx(togglePending && 'dt-btn-busy'), 'aria-busy': togglePending ? 'true' : undefined,
            onClick: () => act(w, toggleAction),
          }, togglePending ? (toggleAction === 'pause' ? '暂停中…' : '恢复中…') : (w.enabled ? '暂停' : '恢复')),
          hostOrbit
            ? h(Btn, {
              variant: 'ghost', size: 'sm', icon: h(Icon, { name: 'trash' }),
              'aria-label': '删除', 'aria-haspopup': 'dialog',
              // 适配器保证先 onCloseOrbit，再由 onRequestConfirm 选中待删 watcher。
              onClick: () => orbitConfirm.onRequestConfirm({ watcher: w }),
            }, '删除')
            : h(ConfirmButton, {
              label: '删除', confirmLabel: '确认删除', variant: 'ghost', size: 'sm', modalContext: 'orbit',
              icon: h(Icon, { name: 'trash' }),
              // 打开确认框时清掉上一次的删除失败（读取错误不动）；本次失败由 act() 重新写入。
              onOpen: clearDeleteError,
              onConfirm: () => act(w, 'delete'), error: deleteError,
              preventDismissOnBusy: true, busyLabel: '删除中…', busyHint: '删除进行中，完成后才能关闭',
            }))

        if (hostOrbitRow) {
          // 官方非展开 DisclosureRow：状态/仓库/状态标签走行头，三个独立动作走 headerAccessory。
          // 语义列表沿用 conversation QueueDock 的 ul>li；watcher 详情是官方无同物的 derived 内容。
          return h('li', { key, className: 'sa-dock-row sa-dock-official-row', title: detailTitle(w) },
            h(hostOrbitRow, {
              icon: statusDot,
              title: String(w.repo || '?'),
              open: false,
              expandable: false,
              previewChevron: false,
              onToggle: noRowToggle,
              collapsedContent: statusTag,
              headerAccessory: actions,
              className: 'sa-dock-disclosure',
              contentClassName: 'sa-dock-content',
              contentLayoutClassName: 'sa-dock-line',
              titleClassName: 'sa-dock-name',
            }),
            details)
        }

        // 旧宿主完整回退：不改 DOM、不依赖 DisclosureRow 或语义列表。
        return h('div', { key, className: 'sa-dock-row', title: detailTitle(w) },
          statusDot,
          h('div', { className: 'sa-dock-body' },
            h('div', { className: 'sa-dock-line' },
              h('span', { className: 'sa-dock-name' }, String(w.repo || '?')),
              statusTag),
            detailContent),
          actions)
      })

      // 比例条：和官方的「上下文已用」同一种表达（一眼看出多少在跑）
      // 为 0 的类别不画那一小截（以前用 0.001 撑出 1px 细丝，看着像脏点）
      const rest = list.length - running - broken
      const barCounts = [['ok', running], ['warn', rest], ['err', broken]].filter(([, n]) => n > 0)
      const bar = barCounts.length === 0 ? null : h('div', { className: 'sa-dock-bar', 'aria-hidden': 'true' },
        barCounts.map(([tone, n]) => h('span', { key: tone, 'data-tone': tone, style: { flexGrow: n } })))

      const panelBody = h(React.Fragment, null,
        h('div', { className: 'sa-dock-head' },
          h('span', { className: 'sa-dock-title' }, brandName),
          // 和面板统计条同一套官方素材：dt-stat（数值强 + 单位弱），不再自己发明 sa-dock-metric-*
          h(Stat, {
            className: 'sa-dock-metric',
            value: watchers === null ? '—' : running + ' / ' + list.length,
            unit: watchers === null ? (readError ? '读取失败' : '读取中') : '主理中',
          })),
        bar,
        watchers === null
          ? (!readError ? h('div', { className: 'sa-dock-empty' }, '正在读取…') : null)
          : list.length === 0
            ? h('div', { className: 'sa-dock-empty' }, '还没有。在「' + brandName + '」标签页填 owner/name 加一条。')
            : h(hostOrbitRow ? 'ul' : 'div', { className: 'sa-dock-rows' }, rows),
        // 读失败与写操作失败分开说；旧列表仍可保留在错误上方。
        readError ? h('div', { className: 'sa-dock-err', role: 'alert' }, readError) : null,
        !readError && orbitError ? h('div', { className: 'sa-dock-err', role: 'alert' }, '操作失败：' + orbitError) : null,
        list.length > 0
          ? h('div', { className: 'sa-dock-foot' }, '详细状态与凭证在「' + brandName + '」标签页')
          : null)

      const panel = hostOrbit
        ? h(hostOrbit.OrbitPopover, {
          open,
          anchorRef: btnRef,
          orbitRef: btnRef,
          panelRef,
          onClose: () => setOpen(false),
          width,
          className: 'dt-card sa-dock-panel',
          ariaLabel: summary,
        }, panelBody)
        : h('div', {
          ref: panelRef, role: 'dialog', 'aria-label': summary, tabIndex: -1, className: 'dt-card sa-dock-panel',
          style: { position: 'fixed', left, bottom, width, zIndex: 2147483000 },
        }, panelBody)

      // 选中待删 watcher 时 open 已被适配器先置 false：portal surface 卸载，本地确认 Modal 才打开。
      const confirmation = hostOrbit && anchor
        ? h(ConfirmDialog, {
          open: true,
          label: '删除',
          confirmLabel: '确认删除',
          title: '确认删除',
          modalContext: 'orbit',
          onConfirm: () => act(anchor, 'delete'),
          onClose: closeConfirm,
          error: deleteError,
          preventDismissOnBusy: true,
          busyLabel: '删除中…',
          busyHint: '删除进行中，完成后才能关闭',
        })
        : null

      return h(React.Fragment, null, button, panel, confirmation)
    }


    // ── 主理鱼面板（按官方 GUI 的信息架构重写：总览 / 触发 / 设置）────────────────
    // 布局、类名、控件都来自上面移植的 dt-* 设计系统，数据和动作接我们自己的 /state、/watch。
    // ── 官方 GUI 的三个交互件（移植自 src/client/provider.tsx + draft.tsx + src/client/controls.tsx）──

    /** 轮询：可见时按 interval，隐藏时降频 6 倍；上一轮没跑完就跳过；卸载时清干净。 */
    function usePoll(task, intervalMs = POLL_MS) {
      const ref = useRef(task)
      ref.current = task
      const busy = useRef(false)
      useEffect(() => {
        let alive = true
        let timer = null
        let discovered = false
        const step = async () => {
          if (!alive) return
          if (!discovered) {
            discovered = true
            try { await discoverBase() } catch {}
          }
          const hidden = typeof document !== 'undefined' && document.hidden === true
          const delay = hidden ? intervalMs * 6 : intervalMs
          if (!busy.current) {
            busy.current = true
            try { await ref.current() } catch {}
            finally { busy.current = false }
          }
          if (alive) timer = setTimeout(step, delay)
        }
        step()
        return () => { alive = false; if (timer) clearTimeout(timer) }
      }, [intervalMs])
    }

    /** 轻量 Toast：DSH 可用时由宿主计时；fallback 保留 4 秒自动消失。 */
    const TOAST_HOLD_MS = 3000
    const TOAST_FADE_MS = 1000
    function useToast() {
      if (hostUi) {
        const { item, notify, dismiss } = hostUi.createToastState()
        // 保持 [view, notify] 调用面；item 替换加 key 保证同时只有最新一条。
        const view = h(hostUi.Toast, { item, onDone: dismiss })
        return [view, notify]
      }
      // 官方 .dt-toast 是 position:fixed 顶部居中、且带自动淡出动画 —— 同时挂两条会重叠，
      // 所以这里保持「同一时刻只有最新一条」，停留时长用官方的 --dt-toast-hold 控制。
      const [item, setItem] = useState(null)
      const timer = useRef(null)
      useEffect(() => () => { if (timer.current) clearTimeout(timer.current) }, [])
      const notify = useCallback((text, tone = 'info') => {
        if (timer.current) clearTimeout(timer.current)
        setItem({ id: Math.random().toString(36).slice(2), text, tone })
        // timer 覆盖 hold + fade 全段；比只等 hold 长 1 秒，淡出不会被提前卸载打断。
        timer.current = setTimeout(() => setItem(null), TOAST_HOLD_MS + TOAST_FADE_MS)
      }, [])
      const view = item === null ? null : h('div', {
        // key 用 item.id：连续通知换 id 时 React 会重建节点，dt-toast-in 动画才会从头播（W17）。
        key: item.id,
        className: cx('dt-toast', item.tone === 'error' && 'dt-danger'),
        role: 'status',
        'aria-live': 'polite',
        // 官方 toast 是深底白字；错误态不要改成浅红底（白字会看不清），改用左侧红条标记
        style: item.tone === 'error'
          ? { '--dt-toast-hold': TOAST_HOLD_MS + 'ms', borderLeft: '3px solid var(--dsw-alias-state-error-primary)' }
          : { '--dt-toast-hold': TOAST_HOLD_MS + 'ms' },
      }, item.text)
      return [view, notify]
    }

    /** 判定页内联追溯时间线：结构化 traces 优先，旧服务端只有 text 时降级。 */
    function TraceTimeline({ payload }) {
      if (payload.loading) return h('p', { className: 'dt-hint' }, '正在追溯 #' + payload.number + '…')
      if (payload.error) return h('p', { className: 'dt-error', role: 'alert', 'aria-atomic': 'true' }, '追溯失败：' + payload.error)
      const traces = Array.isArray(payload.traces) ? payload.traces : []
      if (traces.length === 0) return h('p', { className: 'dt-hint' }, payload.text || '没有结果')
      return h('ol', { className: 'dt-timeline' }, traces.map((trace) => {
        const events = []
        if (trace.seen) events.push({ at: trace.seen, label: '首次看到' })
        if (trace.reported) events.push({ at: trace.reported, label: '上次上报' })
        for (const item of trace.filtered || []) events.push({ at: item.at, label: '被过滤：' + (item.reason || '未知') })
        for (const item of trace.absorbed || []) events.push({ at: item.at, label: '被冷却期吸收' })
        for (const item of trace.wakes || []) events.push({ at: item.at, label: item.ok ? '唤醒成功' : '唤醒失败：' + (item.reason || '未知') })
        events.sort((a, b) => String(a.at || '').localeCompare(String(b.at || '')))
        // 官方 Thread/ConfigHistory 是「一个事件一个 li」，不是一整个 trace 里塞一叠 foot。
        return [
          h('li', { className: 'dt-event', key: 'head' },
            h('div', { className: 'dt-event-head' }, '#' + trace.number + ' @ ' + (trace.repo || '?') + '：' + trace.verdict)),
          ...events.map((event, index) => h('li', { className: 'dt-event', key: index },
            h('div', { className: 'dt-event-text' }, event.label),
            h('div', { className: 'dt-event-foot' }, event.at))),
        ]
      }))
    }

    // ── G3 · 设置 → 门禁 ────────────────────────────────────────────────────────
    // 门禁是内容层检查：只看要发出去的正文，不改工具权限（权限层留给官方那套）。
    // 所有状态都挂在这个子组件里 —— 主面板的 useState 队列按位置取值，
    // 往主面板加槽位会让一批用例静默错位（AGENTS 坑 9）。
    const GATE_SCOPES = [
      { value: 'default', label: '默认（只读）' },
      { value: 'user', label: '我的' },
      { value: 'project', label: '项目' },
    ]
    const GATE_LAYER_LABELS = { default: '默认', user: '我的', project: '项目', local: '本地' }
    // 四层顺序：数值越大越靠后、优先级越高。面板只能写 user / project 两层。
    const GATE_LAYER_RANK = { default: 0, user: 1, project: 2, local: 3 }
    const GATE_SCOPE_HINTS = {
      default: '默认规则随包发布，这里只读；要调整就切到「我的」或「项目」。',
      user: '「我的」写到用户全局配置，本机所有仓库都生效。',
      project: '「项目」写到这个仓库的 .superagent/gates.json，跟着仓库走。',
    }
    const GATE_TRIGGER_LABELS = {
      'issue.reply': '议题回复',
      'pr.reply': 'PR 回复',
      'pr.handle': '处理 PR',
      merge: '合并',
      release: '发布',
    }
    const GATE_TYPE_LABELS = {
      regex: '正则匹配',
      contains: '包含指定文字',
      maxLength: '长度上限',
      nonEmpty: '内容不能为空',
      'foreign-ref': '只引用当前编号',
    }
    // 语义复审只对这两类触发生效（G1 契约）；面板只如实说明，不另开判断。
    const GATE_SEMANTIC_TRIGGERS = ['merge', 'release']

    /**
     * 规则的可比较快照：会落盘的字段，**包含** `enabled`（缺省 = 开）。
     * 不把开关算进来，「本层只有开关不同」就会被当成没改 → 不落盘，等于假启停。
     */
    const gateRuleKey = (rule) => JSON.stringify([
      rule.id, rule.label, rule.description || '', rule.severity || 'error',
      rule.triggers || null, rule.type, rule.pattern === undefined ? null : rule.pattern,
      rule.flags || '', rule.value === undefined ? null : rule.value, rule.message || '',
      rule.enabled !== false,
    ])

    /** 某一层的原始配置对象（GET /gates 的 scopes[scope].config）；null = 这层没有文件。 */
    const gateRawLayer = (data, scope) => {
      const layer = data && data.scopes && data.scopes[scope] ? data.scopes[scope].config : null
      return layer && typeof layer === 'object' && !Array.isArray(layer) ? layer : null
    }

    /**
     * 合并后是哪一层把这条规则写成停用的（取最靠后、也即真正生效的那一层）。
     * 返回层级名；'' = 没有哪一层显式停用（或这条规则来自 extends 基文件）。
     */
    const gateDisabledLayer = (data, id) => {
      let found = ''
      for (const name of ['default', 'user', 'project', 'local']) {
        const layer = gateRawLayer(data, name)
        const rules = layer && Array.isArray(layer.rules) ? layer.rules : []
        const rule = rules.find((item) => item && item.id === id)
        if (rule) found = rule.enabled === false ? name : ''
      }
      return found
    }

    /** 这条规则被哪几层定义（卡片上的来源标签；本地层也要显示，否则看不出是手工覆盖）。 */
    const gateLayerNames = (data, ruleId) => ['default', 'user', 'project', 'local'].filter((name) => {
      const layer = data && data.scopes && data.scopes[name] && data.scopes[name].config
      const rules = layer && Array.isArray(layer.rules) ? layer.rules : []
      return rules.some((rule) => rule && rule.id === ruleId)
    })

    /**
     * 某一层的语义复审配置：本层写过就用本层的，没写就继承生效值。
     * 本层写 `semantic:false`（或 `{enabled:false}`）就是「这一层关了」：`enabled` 必须跟着是 false。
     * 开关、底部状态、预览三处都读 `enabled`，拿继承来的 true 会渲染出「开关亮着 + 底部说关闭」。
     */
    const gateSemanticOf = (data, scope) => {
      const inherited = (data && data.config && data.config.semantic) || {}
      const layer = data && data.scopes && data.scopes[scope] ? data.scopes[scope].config : null
      const own = layer ? layer.semantic : undefined
      const fields = own && typeof own === 'object' ? own : {}
      const off = own === false || Boolean(own && typeof own === 'object' && own.enabled === false)
      return {
        enabled: off ? false : (typeof fields.enabled === 'boolean' ? fields.enabled : inherited.enabled === true),
        model: typeof fields.model === 'string' && fields.model ? fields.model : (inherited.model || ''),
        prompt: typeof fields.prompt === 'string' && fields.prompt ? fields.prompt : (inherited.prompt || ''),
        off,
      }
    }

    /** 草稿：规则取「四层合并结果」，开关照合并后的真实状态（停用的就是关的）。 */
    const gateDraftOf = (data, scope) => {
      const rules = data && data.config && Array.isArray(data.config.rules) ? data.config.rules : []
      return {
        rules: rules.map((rule) => ({ ...rule, enabled: rule.enabled !== false })),
        semantic: gateSemanticOf(data, scope),
      }
    }

    /** GET /gates 回显的仓库（服务端没回显时返回 ''）。 */
    const gateDataRepo = (data) => (data && typeof data.repo === 'string' && data.repo) || ''

    /**
     * 草稿显式记着它来自哪个仓库（U2）。
     * data.repo 之后可能被新仓库的响应改写；保存/预览/优化和确认条都只认草稿自己的来源，
     * 否则「切仓库在途时改的草稿」会被写进新仓库（项目层 / 全局 user 层都复现过）。
     */
    const gateDraftRepo = (draft) => (draft && typeof draft.repo === 'string' && draft.repo) || ''

    /**
     * 落盘形状：完整规则 + 布尔 `enabled`。
     * 复启也必须写完整规则 —— 只写 `{ id, enabled:true }` 缺 type，validateConfig 会判非法。
     */
    const gateRuleBody = (rule) => {
      const entry = {
        id: rule.id,
        label: String(rule.label || rule.id),
        description: rule.description || '',
        severity: rule.severity === 'warn' ? 'warn' : 'error',
        type: rule.type,
        message: rule.message || rule.label || '未通过门禁',
        enabled: rule.enabled !== false,
      }
      if (Array.isArray(rule.triggers) && rule.triggers.length > 0) entry.triggers = rule.triggers
      if (rule.pattern !== undefined) entry.pattern = rule.pattern
      if (rule.flags) entry.flags = rule.flags
      if (rule.value !== undefined) entry.value = rule.value
      return entry
    }

    /**
     * 落盘配置（写当前层）。
     * - 本层已有的规则原样保留：保存是**整层替换**，漏一条就等于把它删了；
     * - 只有和「四层合并结果」不同的规则才新建；
     * - 关掉写 `enabled:false`（完整规则），打开写 `enabled:true`（完整规则）；
     * - 本层原有的 `semantic`（对象或 `false`）没改就原样写回 —— 否则一次只改规则的保存
     *   会顺手把「语义复审开/关」删掉，改变模型调用边界（Q1-F2）；
     * - 本层原有的 `extends:false / 'none'` 也原样写回 —— 这两个是「不再继承上层」的合法写法，
     *   整层重写时丢掉它，replace 就会变回 merge（上层规则重新混进来）。
     */
    const gateConfigOf = (scope, draft, data) => {
      const derived = gateDraftOf(data, scope)
      const baseById = new Map(derived.rules.map((rule) => [rule.id, rule]))
      const rules = new Map()
      const layer = gateRawLayer(data, scope)
      const ownRules = layer && Array.isArray(layer.rules) ? layer.rules : []
      for (const own of ownRules) {
        if (!own || !own.id) continue
        rules.set(own.id, gateRuleBody({ ...own, enabled: own.enabled !== false }))
      }
      for (const rule of draft.rules) {
        const base = baseById.get(rule.id)
        // 没动过的规则不写：本层没有覆盖就继续跟着下层走。
        if (base && gateRuleKey(base) === gateRuleKey(rule)) continue
        rules.set(rule.id, gateRuleBody(rule))
      }
      const config = {
        name: scope === 'user' ? 'user' : 'project',
        rules: [...rules.values()],
      }
      const rawExtends = layer && Object.prototype.hasOwnProperty.call(layer, 'extends') ? layer.extends : undefined
      if (rawExtends === false || rawExtends === 'none') config.extends = rawExtends
      const base = gateSemanticOf(data, scope)
      const semantic = draft.semantic
      const rawSemantic = layer && Object.prototype.hasOwnProperty.call(layer, 'semantic') ? layer.semantic : undefined
      const changed = semantic.enabled !== base.enabled || semantic.model !== base.model || semantic.prompt !== base.prompt
      if (scope === 'project') {
        // 项目层只能关，不能开或换模型（G1 契约）。
        if (semantic.off !== base.off) {
          if (semantic.off) config.semantic = false
          // 取消勾选「关闭」= 去掉本层覆盖、回到继承：不写 semantic 这一项。
        } else if (rawSemantic !== undefined) {
          // 没动过：本层原本写的是什么就原样写回（`false` 或对象）—— 否则一次规则保存
          // 会让项目层重新开始请求模型（Q1-F2 证据 B）。
          config.semantic = rawSemantic
        }
      } else if (changed) {
        // 面板改过语义复审：以本层原有的 semantic 对象为底，保留 mode / baseURL / timeout
        // 这些面板不展示的字段。
        const baseFields = rawSemantic && typeof rawSemantic === 'object' && !Array.isArray(rawSemantic) ? rawSemantic : {}
        config.semantic = { ...baseFields, enabled: semantic.enabled, model: semantic.model, prompt: semantic.prompt }
      } else if (rawSemantic !== undefined) {
        // 没改语义复审：本层原有的 semantic 原样写回（Q1-F2 证据 A）。
        config.semantic = rawSemantic
      }
      return config
    }

    /** 草稿相对「它自己那个仓库的数据」是不是真有未保存改动（只比落盘形状，来源字段不参与）。 */
    const gateDraftDirty = (candidate, source, layerScope) => {
      const derived = gateDraftOf(source, layerScope)
      return JSON.stringify(gateConfigOf(layerScope, candidate, source))
        !== JSON.stringify(gateConfigOf(layerScope, derived, source))
    }

    /** 模型下拉：当前值一定在列表里，取不到列表时也要有个能显示的项。 */
    const gateModelOptions = (value, models) => {
      const list = (models && Array.isArray(models.list) ? models.list : [])
        .map((item) => ({ id: String((item && item.id) || ''), label: String((item && (item.label || item.id)) || '') }))
        .filter((item) => item.id)
      if (value && !list.some((item) => item.id === value)) list.unshift({ id: value, label: value })
      if (list.length === 0) list.push({ id: '', label: '（还没读到模型）' })
      return list
    }

    /** `extends` 的可读形态：只用于「为什么不能从面板保存」的说明（false / 'none' 不算共享）。 */
    const gateExtendsText = (value) => {
      if (typeof value === 'string') return 'extends:' + value
      if (value === true) return 'extends:true'
      if (value && typeof value === 'object') return 'extends 内联对象'
      return 'extends:' + String(value)
    }

    /** `extends:false / 'none'` 的面板文案：这两个值是「不再继承上层」，不是共享规则。 */
    const gateExtendsDetachedText = (value) => (value === false ? 'extends:false' : "extends:'none'")

    function GateSettings({ repo, notify }) {
      // 12 个 useState 全在这里，顺序被 test/client.test.mjs 的 gateQueue 按位置锁定。
      const [data, setData] = useState(null)
      const [loadError, setLoadError] = useState('')
      const [scope, setScope] = useState('user')
      const [draft, setGateDraft] = useState(null)
      const [advanced, setAdvanced] = useState('')
      const [sample, setSample] = useState('')
      const [trigger, setTrigger] = useState('issue.reply')
      const [preview, setPreview] = useState(null)
      const [models, setModels] = useState({ repo: '', list: [], loading: false, error: '' })
      const [busy, setBusy] = useState('')
      const [status, setStatus] = useState(null)
      // 切作用域 / 切仓库时如果还有草稿：先问一句 —— 取消保留草稿，确认才放弃。
      // 形状：{ type: 'scope', next }（作用域确认，可带 repoHeld）或 { repoHeld: true }（换仓库已选择保留草稿）。
      // 只能在末尾追加（位置化队列），同步 test/client.test.mjs 的 gateQueue。
      const [pendingSwitch, setPendingSwitch] = useState(null)
      const repoQuery = (value) => (value ? '?repo=' + encodeURIComponent(String(value)) : '')
      const scopeQuery = repoQuery(repo)
      // U1：load 的代际与取消。切仓库 / 保存后重读 / 点「重新读取」都可能并发：
      // 没有序号时后到的旧响应会覆盖新仓库的数据；没有取消时卸载后还会继续 setState。
      const loadSeqRef = useRef(0)
      const aliveRef = useRef(true)
      // 响应回来时用它判断「仓库还是发起这次请求的那个」：/state 后到时仓库已经换了，
      // 旧仓库的响应不能再提交（否则面板显示旧配置，保存却按新仓库写）。
      const repoRef = useRef(repo)
      repoRef.current = repo
      // 草稿脏不脏要按「提交那一刻的作用域」算 —— 请求在途时用户可能已经切过作用域。
      const scopeRef = useRef(scope)
      scopeRef.current = scope
      // load 的闭包停在请求发起那一刻；提交时要拿「现在」的草稿和数据判断能不能落地。
      const draftRef = useRef(draft)
      draftRef.current = draft
      const dataRef = useRef(data)
      dataRef.current = data
      const modelsSeqRef = useRef(0)
      // 模型列表属于哪个仓库（落地时记进 state.repo）；这里的镜像让异步回调能同步读到最新值。
      const modelsStateRef = useRef(models)
      modelsStateRef.current = models
      // 「面板这份配置现在属于哪个仓库」的渲染值镜像：草稿来源优先（与下面 panelRepo 同一份计算）。
      const panelRepoRef = useRef('')
      useEffect(() => {
        // StrictMode（开发态）会 mount → cleanup → 再 mount：effect 体里必须复位 alive，
        // 否则第一次 cleanup 把它置 false 之后就永远拦掉响应（门禁页停在「正在读取…」，Q2 F3）。
        aliveRef.current = true
        return () => { aliveRef.current = false }
      }, [])

      /**
       * 面板这份配置属于哪个仓库：dirty 草稿认它自己的来源，否则认 data.repo，最后才是 prop。
       * 异步回调（models 落地、load 的闸门分支）都只认这一个值 —— 换仓库时 prop 先变，
       * 面板数据/草稿还是旧仓库的，模型列表跟着 prop 走就是跨仓库（v4 R1）。
       */
      const panelRepoOf = () => panelRepoRef.current || gateDataRepo(dataRef.current) || repoRef.current

      async function load() {
        const seq = ++loadSeqRef.current
        const requestedRepo = repo
        setLoadError('')
        try {
          const value = await call('/gates' + scopeQuery)
          // 卸载了，或者已经有更新的一次 load：这份响应作废（不许再 setState）。
          if (!aliveRef.current || seq !== loadSeqRef.current) return
          // 仓库已经换过：旧仓库的响应不能提交（新仓库的读取由 effect 或确认流程负责）。
          if (repoRef.current !== requestedRepo) return
          if (!value || value.ok === false) throw new Error((value && value.error) || '服务端没有返回门禁配置')
          // 草稿来自旧仓库、这次响应的却是新仓库：先不动 data（Q2 F1）。
          // 提交会让 repoMismatch 变 false、确认条消失，保存/预览/优化随后就会把旧仓库的
          // 规则状态和提示词写进新仓库（项目层和全局 user 层都复现过）。
          // 只有草稿其实「和它自己那份配置一致」（改回原样）才放行新数据 —— 它没有要保留的内容。
          const replyRepo = gateDataRepo(value) || requestedRepo
          const pendingDraft = draftRef.current
          const pendingSource = gateDraftRepo(pendingDraft)
          if (pendingSource && pendingSource !== replyRepo) {
            const sourceData = gateDataRepo(dataRef.current) === pendingSource ? dataRef.current : null
            const stillDirty = sourceData ? gateDraftDirty(pendingDraft, sourceData, scopeRef.current) : true
            if (stillDirty) {
              // 这份响应被闸门挡住：在途的模型列表同样不能落地（新仓库的 baseURL 换了模型 id），
              // 再把列表拉回草稿来源仓库 —— 保留草稿时面板显示的还是那个仓库的配置（v4 R1）。
              modelsSeqRef.current += 1
              setModels((prev) => (prev.loading ? { ...prev, loading: false } : prev))
              loadModels(pendingSource)
              return
            }
            setGateDraft(null)
          }
          setData(value)
          // 请求在途时用户可能已经改出草稿：只放下「和新数据一致」的草稿，
          // dirty 的一律留着 —— 旧实现无条件 setGateDraft(null)，切仓库在途的草稿会被静默清掉。
          setGateDraft((prev) => {
            if (!prev) return null
            try {
              return gateDraftDirty(prev, value, scopeRef.current) ? prev : null
            } catch {
              // 算不清就保留：宁可多留一份草稿，也不能替用户丢掉改动。
              return prev
            }
          })
        } catch (error) {
          if (!aliveRef.current || seq !== loadSeqRef.current) return
          if (repoRef.current !== requestedRepo) return
          setLoadError(String((error && error.message) || error))
        }
      }

      async function loadModels(forRepo = panelRepoOf()) {
        // 模型列表同样是「按仓库读」的请求：乱序/卸载时旧响应不能盖新列表。
        const requestedRepo = forRepo || repoRef.current
        const seq = ++modelsSeqRef.current
        setModels((prev) => ({ ...prev, loading: true, error: '' }))
        try {
          const value = await call('/gates/models' + repoQuery(requestedRepo))
          if (!aliveRef.current || seq !== modelsSeqRef.current) return
          // 只认「现在」的面板仓库：响应在途时仓库/草稿可能已经换过，
          // 落错仓库的列表一旦被选中保存，就是把别的仓库的模型 id 写进这份配置。
          if (requestedRepo !== panelRepoOf()) return
          const list = value && Array.isArray(value.models)
            ? value.models.map((item) => ({ id: String((item && item.id) || ''), label: String((item && (item.label || item.id)) || '') })).filter((item) => item.id)
            : []
          setModels({ repo: requestedRepo, list, loading: false, error: value && value.ok === false ? String(value.error || '') : '' })
        } catch (error) {
          if (!aliveRef.current || seq !== modelsSeqRef.current) return
          if (requestedRepo !== panelRepoOf()) return
          setModels({ repo: requestedRepo, list: [], loading: false, error: String((error && error.message) || error) })
        }
      }

      const derived = gateDraftOf(data, scope)
      const current = draft || derived
      const readOnly = scope === 'default'
      // 项目层只允许「关闭语义复审」，模型与提示词在项目层不可编辑。
      const semanticLocked = readOnly || scope === 'project'
      const dirty = !readOnly && gateDraftDirty(current, data, scope)
      const scopePath = data && data.scopes && data.scopes[scope] ? data.scopes[scope].path : ''
      // 本层的 `extends` 分两种（v3 冻结行为）：
      // - 共享规则（字符串 / 对象 / true）：面板整层重写会把共享规则冲掉 → 明确拒绝保存，让人去改文件；
      // - 断开继承（false / 'none'）：不是共享，能保存；写回时原样带上这一项（见 gateConfigOf）。
      const rawLayer = gateRawLayer(data, scope)
      const rawExtends = rawLayer && Object.prototype.hasOwnProperty.call(rawLayer, 'extends') ? rawLayer.extends : undefined
      const extendsDetached = rawExtends === false || rawExtends === 'none'
      const layerExtends = rawExtends !== undefined && !extendsDetached
      // 面板里这份配置属于哪个仓库（GET /gates 会回显 repo）：写盘与预览都认它，
      // 否则「/state 换了 workdir 但用户还没确认」时会把旧配置写到新仓库去。
      const dataRepo = gateDataRepo(data)
      // dirty 草稿认**自己的来源仓库**：data.repo 可能已经（或即将）换成新仓库，
      // 保存/预览/优化和确认条都必须跟草稿走 —— 跟 data.repo 走就会把旧仓库的草稿写进新仓库（Q2 F1）。
      const draftRepo = gateDraftRepo(draft)
      const panelRepo = dirty && draftRepo ? draftRepo : dataRepo
      // 渲染值镜像给异步回调用：models 落地、load 的闸门分支都要在「现在」判断面板仓库。
      panelRepoRef.current = panelRepo
      const targetRepo = panelRepo || repo
      const repoMismatch = Boolean(panelRepo && repo && panelRepo !== repo)
      // 模型列表只属于面板这份配置的仓库：列表带了别的仓库（旧版本/迟到响应）就忽略，
      // 下拉里只剩当前值 —— 选它保存就是跨仓库模型 id（v4 R1）。
      const modelsRepo = models && typeof models.repo === 'string' ? models.repo : ''
      const modelsPanelRepo = panelRepo || repo
      const modelsStale = Boolean(modelsRepo && modelsRepo !== modelsPanelRepo)
      const modelsView = modelsStale ? { list: [], loading: models.loading, error: models.error } : models
      // 创建/修改草稿时记下来源仓库；之后 data 怎么变，这份草稿都只写回它自己的仓库。
      const draftSourceRepo = () => draftRepo || dataRepo || repo
      const setDraftConfig = (next) => setGateDraft({ ...next, repo: draftSourceRepo() })
      // 切仓库遇到草稿：确认（取消 → 保留草稿并显示提示；确认 → 放弃后才去读新仓库）。
      const repoHeld = Boolean(pendingSwitch && pendingSwitch.repoHeld === true && repoMismatch && dirty)
      const repoConfirm = repoMismatch && dirty && !repoHeld
      const scopeConfirm = Boolean(pendingSwitch && pendingSwitch.type === 'scope' && pendingSwitch.next)

      // 依赖 repo：面板先挂载、/state 后到的顺序下，仓库路径变了要重读一次（否则项目层会写到服务端 cwd）。
      // 有草稿时**不发新请求**：换仓库这件事交给页面上的确认条（确认 / 保存 / 放弃 都会各自 load）；
      // 已经在途的响应由 load() 里的「草稿来源仓库」闸门挡住，不会把面板数据翻到新仓库去。
      useEffect(() => {
        if (dirty) return
        load()
      }, [repo])
      // 草稿变干净之后（保存 / 放弃 / 手工改回）如果仓库已经换过，补一次重读，别一直停在旧仓库上。
      // 模型列表跟着面板仓库走：取消换仓库后可能在途响应被作废，这里要把它补回来。
      useEffect(() => {
        if (dirty || !repoMismatch) return
        load()
        // 同仓库的列表不用重读（切数据后由 [modelsPanelRepo] effect 负责）；不同仓库才补。
        const currentModels = modelsStateRef.current
        if (currentModels.loading || currentModels.repo !== modelsPanelRepo) loadModels(modelsPanelRepo)
      }, [dirty])
      // 模型列表跟着「面板这份配置」的仓库走：保留草稿时是草稿来源，数据换了之后才是新仓库。
      // 以前依赖 prop repo —— 换仓库会立刻读新仓库的模型列表，而面板数据还是旧仓库的（R1）。
      useEffect(() => { loadModels(modelsPanelRepo) }, [modelsPanelRepo])

      async function save() {
        if (!data || readOnly || busy) return
        if (layerExtends) {
          setStatus({
            tone: 'error',
            text: '保存失败：本层写了 ' + gateExtendsText(rawExtends) + '（共享规则），面板整层重写会把共享规则冲掉；请直接改配置文件' + (scopePath ? '：' + scopePath : ''),
          })
          return
        }
        setBusy('save')
        setStatus(null)
        try {
          const value = await call('/gates', { method: 'POST', body: JSON.stringify({ scope, config: gateConfigOf(scope, current, data), repo: targetRepo }) })
          if (!value || value.ok === false) throw new Error((value && value.error) || '服务端没有返回保存结果')
          const where = value.path ? '：' + value.path : ''
          setGateDraft(null)
          setPendingSwitch(null)
          setStatus({ tone: 'ok', text: '已保存到「' + GATE_LAYER_LABELS[scope] + '」' + where })
          if (notify) notify('门禁配置已保存', 'info')
          await load()
        } catch (error) {
          const text = '保存失败：' + String((error && error.message) || error)
          setStatus({ tone: 'error', text })
          if (notify) notify(text, 'error')
        } finally {
          setBusy('')
        }
      }

      async function runPreview() {
        if (!data || busy) return
        setBusy('preview')
        setStatus(null)
        try {
          const value = await call('/gates/preview', {
            method: 'POST',
            body: JSON.stringify({
              repo: targetRepo,
              trigger,
              body: sample,
              semantic: semanticOn && GATE_SEMANTIC_TRIGGERS.includes(trigger),
            }),
          })
          if (!value || typeof value !== 'object') throw new Error('服务端没有返回预览结果')
          setPreview(value)
        } catch (error) {
          setPreview({ ok: false, hits: [], failed: String((error && error.message) || error) })
        } finally {
          setBusy('')
        }
      }

      async function optimizePrompt() {
        if (busy || semanticLocked) return
        setBusy('optimize')
        setStatus(null)
        try {
          const value = await call('/gates/optimize', {
            method: 'POST',
            body: JSON.stringify({ repo: targetRepo, prompt: current.semantic.prompt, model: current.semantic.model }),
          })
          if (!value || value.ok === false) throw new Error((value && value.error) || '服务端没有返回提示词')
          const text = String(value.text || '').trim()
          if (!text) throw new Error('模型返回空内容')
          setDraftConfig({ ...current, semantic: { ...current.semantic, prompt: text } })
          setStatus({ tone: 'ok', text: '提示词已更新，确认后点保存' })
        } catch (error) {
          setStatus({ tone: 'error', text: '优化提示词失败：' + String((error && error.message) || error) })
        } finally {
          setBusy('')
        }
      }

      // 改草稿一律走 setDraftConfig：顺手记下来源仓库，后面 data.repo 怎么变都不影响保存目标。
      const setSemantic = (next) => setDraftConfig({ ...current, semantic: { ...current.semantic, ...next } })
      const toggleRule = (id, next) => setDraftConfig({
        ...current,
        rules: current.rules.map((rule) => (rule.id === id ? { ...rule, enabled: next } : rule)),
      })
      // 切作用域：没有草稿就直接切（保持原行为）；有草稿先挂确认，取消保留草稿，确认才放弃。
      const applyScope = (next) => {
        setScope(next)
        setGateDraft(null)
        setStatus(null)
        setPreview(null)
      }
      const requestScope = (next) => {
        if (!next || next === scope) return
        if (dirty) {
          // 带上 repoHeld：取消这次作用域切换后，之前「保留草稿」的换仓库决定不能被抹掉。
          setPendingSwitch({ ...(pendingSwitch || {}), type: 'scope', next })
          setStatus(null)
          return
        }
        setPendingSwitch(null)
        applyScope(next)
      }
      const cancelSwitch = () => setPendingSwitch(pendingSwitch && pendingSwitch.repoHeld === true ? { repoHeld: true } : null)
      const confirmScopeSwitch = () => {
        const next = scopeConfirm ? pendingSwitch.next : ''
        setPendingSwitch(null)
        if (next) applyScope(next)
      }
      // 取消换仓库：把已经发出去的新仓库读取一并作废。否则它晚到时代码会把数据切到新仓库，
      // 和「已保留草稿：面板仍是旧仓库」的承诺相反（草稿还在，数据却换了仓库）。
      const cancelRepoSwitch = () => {
        loadSeqRef.current += 1
        // 模型列表也属于面板这份配置（baseURL 来自它）：新仓库的 /gates/models 不能落地，
        // 再把面板仓库（草稿来源）的模型列表重新读一次，别让下拉里留着新仓库的模型（Q2 F2）。
        modelsSeqRef.current += 1
        setModels((prev) => (prev.loading ? { ...prev, loading: false } : prev))
        const panelRepoNow = gateDraftRepo(draftRef.current) || gateDataRepo(dataRef.current)
        if (panelRepoNow) loadModels(panelRepoNow)
        setPendingSwitch({ repoHeld: true })
      }
      // 切仓库由父面板的 workdir 驱动，没法拦在「点击」上：草稿还在时不去读新仓库，
      // 只有这里（明确放弃）才 reset + load，且 load 用的是新的 repo prop。
      const confirmRepoSwitch = () => {
        setGateDraft(null)
        setStatus(null)
        setPreview(null)
        setPendingSwitch(null)
        load()
      }
      // 本层显式关闭（off）优先于继承来的 enabled：项目层写 semantic:false 时，
      // 合并后的 enabled 可能还是上层继承值，不能同屏出现「已关闭 + 当前开启」。
      const semanticOn = current.semantic.off !== true && current.semantic.enabled === true
      const semanticState = !semanticOn
        ? '当前关闭：规则照跑，语义复审不发请求。'
        : !current.semantic.model
          ? '开了但没选模型：语义复审会跳过，硬门禁照常。'
          : '当前开启：命中只给提示，不阻断回复。'
      const enabledRules = current.rules.filter((rule) => rule.enabled).length

      return h(Section, { key: 'gate', title: '门禁' },
        h(Hint, null, '内容层检查：只看要发出去的正文，不改工具权限。关掉一条规则会真正停用，不再参与检查。'),
        h('div', { className: 'dt-field' },
          h('span', { className: 'dt-field-head' }, h('span', null, '作用域')),
          h(SegmentedControl, {
            id: 'gate-scope',
            options: GATE_SCOPES,
            value: scope,
            label: '门禁作用域',
            onChange: requestScope,
          }),
          h('small', null, GATE_SCOPE_HINTS[scope] || '')),
        scopeConfirm
          ? h('div', { className: 'dt-banner', role: 'group', 'aria-label': '切换作用域确认' },
            h('p', null, '还有未保存的改动：切到「' + (GATE_LAYER_LABELS[pendingSwitch.next] || pendingSwitch.next) + '」会丢掉这份草稿。'),
            h('div', { className: 'dt-actions dt-banner-actions' },
              h(Btn, { size: 'sm', variant: 'ghost', onClick: cancelSwitch }, '取消'),
              h(Btn, { size: 'sm', variant: 'primary', onClick: confirmScopeSwitch }, '放弃草稿并切换')))
          : null,
        repoConfirm
          ? h('div', { className: 'dt-banner', role: 'group', 'aria-label': '切换仓库确认' },
            h('p', null, '仓库已切换到 ' + repo + '：当前面板还是 ' + panelRepo + ' 的配置，这份草稿还没保存。'),
            h('div', { className: 'dt-actions dt-banner-actions' },
              h(Btn, { size: 'sm', variant: 'ghost', onClick: cancelRepoSwitch }, '取消'),
              h(Btn, { size: 'sm', variant: 'primary', onClick: confirmRepoSwitch }, '放弃草稿并切换')))
          : null,
        repoHeld
          ? h('div', { className: 'dt-banner' },
            h('p', null, '已保留草稿：面板仍是 ' + panelRepo + '，保存或放弃这份草稿后才会切到 ' + repo + '。'),
            h('div', { className: 'dt-actions dt-banner-actions' },
              h(Btn, { size: 'sm', variant: 'primary', onClick: confirmRepoSwitch }, '放弃草稿并切换')))
          : null,
        h('p', { className: 'dt-hint' }, '配置文件：', h('span', { className: 'dt-mono' }, scopePath || '（服务端没有返回路径）')),
        !readOnly && data && extendsDetached
          ? h('p', { className: 'dt-hint' },
            '本层用 ' + gateExtendsDetachedText(rawExtends) + ' 断开了上层规则：保存会把这一项原样写回，上层规则不会重新合进来。')
          : null,
        !data && !loadError ? h('p', { className: 'dt-hint' }, '正在读取门禁配置…') : null,
        loadError
          ? h('div', { className: 'dt-banner dt-danger' },
            h('p', { className: 'dt-error', role: 'alert' }, '读取门禁配置失败：' + loadError),
            h('div', { className: 'dt-actions dt-banner-actions' },
              h(Btn, { size: 'sm', variant: 'outline', onClick: () => { load() } }, '重新读取')))
          : null,
        data
          ? h('div', { className: 'dt-subhead' },
            h('span', null, '规则 ' + enabledRules + ' / ' + current.rules.length),
            readOnly ? h(Tag, { tone: 'neutral' }, '只读') : null)
          : null,
        data
          ? h('ul', { className: 'dt-rules' }, current.rules.map((rule) => {
            const open = advanced === rule.id
            const disabledLayer = gateDisabledLayer(data, rule.id)
            // 停用来自更靠后的层（项目 / 本地）：当前作用域写什么都不生效，开关不能假装能改。
            const lockedByLaterLayer = Boolean(disabledLayer && GATE_LAYER_RANK[disabledLayer] > GATE_LAYER_RANK[scope])
            return h('li', { key: rule.id, className: 'dt-rule' },
              h('div', { className: 'dt-row-main' },
                h('div', { className: 'dt-row-title' },
                  h('span', { className: 'dt-row-name' }, rule.label),
                  h(Tag, { tone: rule.severity === 'warn' ? 'warning' : 'quiet' }, rule.severity === 'warn' ? '提示' : '阻断'),
                  gateLayerNames(data, rule.id).map((name) => h(Tag, { key: name, tone: 'outline' }, GATE_LAYER_LABELS[name]))),
                h('div', { className: 'dt-hint' }, rule.description || '没有说明'),
                !rule.enabled
                  ? h('div', { className: 'dt-hint' }, readOnly
                    ? '已停用：这条规则不参与检查。'
                    : lockedByLaterLayer
                      ? disabledLayer === 'local'
                        ? '本地层（.superagent/gates.local.json，优先级最高）停用了这条：面板改不回来，要恢复请手改这个文件。'
                        : '「' + GATE_LAYER_LABELS[disabledLayer] + '」层比当前作用域更靠后，停用了这条：这里改不回来，请到「' + GATE_LAYER_LABELS[disabledLayer] + '」层处理。'
                      : disabledLayer === scope
                        ? '本层已停用：这条规则不参与检查；打开会把完整规则（含 enabled:true）写回本层。'
                        : '上层已停用：打开会写一条完整规则到本层，把它恢复。')
                  : null,
                h(DisclosureRow, {
                  title: '高级',
                  icon: h(Icon, { name: 'question', size: 14 }),
                  open,
                  bodyId: 'gate-rule-' + rule.id + '-body',
                  onToggle: () => setAdvanced(open ? '' : rule.id),
                }, h('div', { className: 'dt-rule-body' },
                  h('div', { className: 'dt-field' },
                    h('span', { className: 'dt-field-head' }, h('span', null, '类型')),
                    h('small', null, GATE_TYPE_LABELS[rule.type] || rule.type)),
                  rule.pattern
                    ? h('div', { className: 'dt-field' },
                      h('span', { className: 'dt-field-head' }, h('span', null, '正则')),
                      h('span', { className: 'dt-mono' }, rule.pattern))
                    : null,
                  h('div', { className: 'dt-field' },
                    h('span', { className: 'dt-field-head' }, h('span', null, '命中后提示')),
                    h('small', null, rule.message || '没有提示')),
                  h('div', { className: 'dt-field' },
                    h('span', { className: 'dt-field-head' }, h('span', null, '适用触发')),
                    h('small', null, (rule.triggers || []).map((item) => GATE_TRIGGER_LABELS[item] || item).join('、') || '未指定'))))),
              h('div', { className: 'dt-rule-switch' },
                h(Toggle, {
                  id: 'gate-rule-' + rule.id + '-switch',
                  checked: rule.enabled,
                  disabled: readOnly || lockedByLaterLayer,
                  label: '启用「' + rule.label + '」',
                  onChange: (next) => toggleRule(rule.id, next),
                })))
          }))
          : null,
        !readOnly && data && layerExtends
          ? h('div', { className: 'dt-banner dt-danger' },
            h('p', { className: 'dt-error', role: 'alert' },
              '这一层用了 ' + gateExtendsText(rawExtends) + '（共享规则）：面板保存会整层重写并把共享规则冲掉，所以这里不写入。请直接改配置文件：'
              + (scopePath || '（服务端没有返回路径）')))
          : null,
        !readOnly && data
          ? h('div', { className: 'dt-savebar' },
            h('p', { className: 'dt-hint', 'aria-live': 'polite' },
              busy === 'save' ? '保存中…' : layerExtends ? '本层有 extends：保存已禁用' : dirty ? '未保存' : '与已保存的一致'),
            h(Btn, {
              size: 'sm',
              variant: 'ghost',
              disabled: !dirty || Boolean(busy),
              // 放弃草稿后，如果父面板已经换了仓库，这一步才真的切过去（否则草稿一清就静默换仓库）。
              onClick: () => {
                setGateDraft(null)
                setStatus(null)
                setPendingSwitch(null)
                if (repoMismatch) load()
              },
            }, '放弃'),
            h(Btn, { size: 'sm', variant: 'primary', disabled: !dirty || Boolean(busy) || layerExtends, onClick: save }, busy === 'save' ? '保存中…' : '保存'))
          : null,
        status
          ? h('p', { className: status.tone === 'error' ? 'dt-error' : 'dt-hint', role: status.tone === 'error' ? 'alert' : 'status' }, status.text)
          : null,
        h(Hint, null, '保存是整层替换：本层已有的覆盖会原样带上，没改过的规则继续跟着下层走。关掉一条会写成 enabled:false 真正停用，重新打开会写成完整规则 + enabled:true。'),
        data
          ? h('div', { className: 'dt-part' },
            h('div', { className: 'dt-subhead' }, h('span', null, '试跑预览')),
            h(Hint, null, '预览跑的是已保存的配置：先把改动保存，再看效果。'),
            h('div', { className: 'dt-field' },
              h('span', { className: 'dt-field-head' }, h('span', null, '触发')),
              h('select', {
                className: 'dt-select',
                id: 'gate-preview-trigger',
                value: trigger,
                onChange: (event) => { setTrigger(event.target.value); setPreview(null) },
              }, Object.keys(GATE_TRIGGER_LABELS).map((value) => h('option', { key: value, value }, GATE_TRIGGER_LABELS[value])))),
            h('div', { className: 'dt-field' },
              h('span', { className: 'dt-field-head' }, h('span', null, '试跑正文')),
              h('textarea', {
                className: 'dt-textarea',
                id: 'gate-preview-text',
                rows: 4,
                value: sample,
                placeholder: '把要发出去的那段回复粘进来',
                onChange: (event) => { setSample(event.target.value); setPreview(null) },
              })),
            h('div', { className: 'dt-actions' },
              h(Btn, { size: 'sm', variant: 'outline', disabled: Boolean(busy), onClick: runPreview }, busy === 'preview' ? '预览中…' : '预览'),
              GATE_SEMANTIC_TRIGGERS.includes(trigger)
                ? h(Tag, { tone: 'info' }, semanticOn ? '这类触发会追加语义复审' : '这类触发支持语义复审（当前关闭）')
                : null),
            preview && preview.failed
              ? h('p', { className: 'dt-error', role: 'alert' }, '预览失败：' + preview.failed)
              : preview
                ? h('div', { className: preview.ok ? 'dt-banner' : 'dt-banner dt-danger' },
                  h('p', { className: 'dt-banner-label' },
                    preview.ok ? '通过：没有阻断项' : '命中 ' + ((preview.hits || []).length) + ' 条'),
                  (preview.hits || []).map((hit) => h('p', { key: hit.id || hit.label, className: 'dt-hint' },
                    (hit.severity === 'error' ? '阻断 · ' : '提示 · ') + (hit.label || hit.id) + '：' + (hit.message || ''))),
                  preview.semantic && preview.semantic.warning
                    ? h('p', { className: 'dt-hint' }, '语义复审：' + preview.semantic.warning)
                    : null,
                  preview.semantic && preview.semantic.skipped
                    ? h('p', { className: 'dt-hint' }, '语义复审：已跳过（只对合并/发布生效，或没开、没配模型）')
                    : null)
                : null)
          : null,
        data
          ? h('div', { className: 'dt-part' },
            h('div', { className: 'dt-subhead' }, h('span', null, '语义复审')),
            h(Hint, null, '只对合并（merge）和发布（release）生效；议题与 PR 回复永远只跑规则。模型不可用时放行并给提示，不阻断硬门禁。'),
            scope === 'project'
              ? h('div', { className: 'dt-switch-row' },
                h('div', { className: 'dt-field-head' }, h('span', null, '在本项目关闭语义复审')),
                h(Toggle, {
                  id: 'gate-semantic-off',
                  checked: current.semantic.off,
                  disabled: readOnly,
                  label: '在本项目关闭语义复审',
                  onChange: (next) => setSemantic({ off: next }),
                }))
              : h('div', { className: 'dt-switch-row' },
                h('div', { className: 'dt-field-head' }, h('span', null, '开启语义复审')),
                h(Toggle, {
                  id: 'gate-semantic-enabled',
                  // 本层写了 semantic:false 时 gateSemanticOf 已经把 enabled 算成 false，三处一致。
                  checked: current.semantic.enabled,
                  disabled: readOnly,
                  label: '开启语义复审',
                  // 打开时清掉 off：本层的 semantic:false 不擦掉就永远显示关闭（开关成了装饰）。
                  onChange: (next) => setSemantic({ enabled: next, off: next ? false : current.semantic.off }),
                })),
            h('div', { className: 'dt-field' },
              h('span', { className: 'dt-field-head' }, h('span', null, '模型')),
              h('select', {
                className: 'dt-select',
                id: 'gate-semantic-model',
                value: current.semantic.model,
                disabled: semanticLocked,
                onChange: (event) => setSemantic({ model: event.target.value }),
              }, gateModelOptions(current.semantic.model, modelsView).map((item) => h('option', { key: item.id || 'empty', value: item.id }, item.label === item.id ? item.id : item.label + '（' + item.id + '）')))),
            h('div', { className: 'dt-actions' },
              h(Btn, { size: 'sm', variant: 'ghost', disabled: models.loading, onClick: () => { loadModels() } }, models.loading ? '读取中…' : '读取模型列表'),
              models.error ? h('span', { className: 'dt-error' }, '模型列表读取失败：' + models.error) : null,
              modelsStale
                ? h('span', { className: 'dt-error' }, '模型列表属于 ' + modelsRepo + '，不是当前面板的 ' + modelsPanelRepo + '：已忽略，请重新读取')
                : null,
              scope === 'project' ? h('span', { className: 'dt-hint' }, '项目层只能关闭语义复审，不能换模型或改提示词') : null),
            h('div', { className: 'dt-field' },
              h('span', { className: 'dt-field-head' }, h('span', null, '审核提示词')),
              h('textarea', {
                className: 'dt-textarea',
                id: 'gate-semantic-prompt',
                rows: 5,
                value: current.semantic.prompt,
                disabled: semanticLocked,
                placeholder: '给模型审核用的提示词，要求它输出 JSON',
                onChange: (event) => setSemantic({ prompt: event.target.value }),
              })),
            h('div', { className: 'dt-actions' },
              h(Btn, { size: 'sm', variant: 'outline', disabled: Boolean(busy) || semanticLocked, onClick: optimizePrompt },
                busy === 'optimize' ? '优化中…' : '优化提示词'),
              h('span', { className: 'dt-hint' }, semanticState)))
          : null)
    }

    function ZhuliPanel(props) {
      const sessionId = props && props.sessionId
      /** 暂停原因码 → 人话（服务端 deferReason 是唯一来源）。 */
      const DEFER_LABELS = {
        'active-hours': '不在活跃时段',
        ratelimit: '被 GitHub 限流',
        quota: '额度紧张',
        unchanged: '没有变化',
        observe: '仅观察',
        'cap-session': '撞会话上限',
        'cap-watcher': '撞单守上限',
        busy: '会话忙',
      }
      const [snap, setSnap] = useState({ loading: true, error: null, body: null })
      const [tab, setTab] = useState('overview')
      const [notice, setNotice] = useState(null)
      const [form, setForm] = useState({ ...CREATE_WATCHER_VALUES })
      const [token, setToken] = useState('')
      const [goalDraft, setGoalDraft] = useState('')
      const [goalTouched, setGoalTouched] = useState(false)
      const [toastView, notify] = useToast()
      // 三处复制共用同一个成功/失败出口：禁止剪贴板时不能假报「已复制」。
      const copyText = async (text, successMessage) => {
        try {
          await navigator.clipboard.writeText(text)
          notify(successMessage, 'info')
          return true
        } catch (error) {
          notify('复制失败：' + String((error && error.message) || error || '请手动选中文本'), 'error')
          return false
        }
      }
      const [editing, setEditing] = useState(null)
      const [draft, setDraft] = useState({})
      // 审核 M2：记下「这次编辑真的改过哪些字段」—— 保存时只发改过的，
      // 否则全量 PUT 会把别处刚改的值按打开那一刻的旧快照写回（lost update）。
      const touchedRef = useRef(new Set())
      const createTouchedRef = useRef(new Set())
      const createSnapshotRef = useRef({ ...CREATE_WATCHER_VALUES })
      const editSnapshotRef = useRef(null)
      const watcherDraftConfirmedRef = useRef(false)
      // 保存期间仍可输入：旧响应只能清理它提交的那份草稿，不能覆盖 pending 后的新输入。
      const credentialsDraftVersionRef = useRef(0)
      const credentialsSaveSeqRef = useRef(0)
      const goalDraftVersionRef = useRef(0)
      const goalSaveSeqRef = useRef(0)
      const setField = (next) => {
        const opening = editSnapshotRef.current && editSnapshotRef.current.values
        for (const key of Object.keys(next)) {
          if (opening && watcherSameValue(next[key], opening[key])) touchedRef.current.delete(key)
          else if (next[key] !== draft[key]) touchedRef.current.add(key)
        }
        setDraft(next)
      }
      const setCreateField = (key, value) => {
        if (watcherSameValue(value, createSnapshotRef.current[key])) createTouchedRef.current.delete(key)
        else createTouchedRef.current.add(key)
        setForm({ ...form, [key]: value })
      }
      const [search, setSearch] = useState('')
      // 面板弹层共用既有状态槽：术语/规则使用 boolean/string，dirty 编辑确认使用对象。
      const [glossary, setGlossary] = useState(false)
      // 版本检查结果（只读）：面板里直接告诉客户「有没有新版本、该跑什么命令」
      const [update, setUpdate] = useState(null)
      const [checking, setChecking] = useState(false)
      // 设置页用官方的「左栏 + 内容」结构（dt-settings），所以要记当前分区
      const [settingsSection, setSettingsSection] = useState('source')
      // 判定页：点「跳过原因」标签就只看这一类（知识库里记的"还没做"，本轮补上）
      const [reasonFilter, setReasonFilter] = useState(null)
      // 行内徽章可点：点「只读」就只看只读的守（与跳过原因筛选同一套交互）
      const [badgeFilter, setBadgeFilter] = useState(null)
      // 只看有问题的：放在最后追加，避免打乱测试桩按位置取值的队列
      const [problemsOnly, setProblemsOnly] = useState(false)
      // 统计条筛选：点「异常」只看异常。放在 hook 列表末尾，避免打乱既有下标（测试桩按位置取值）
      const [stateFilter, setStateFilter] = useState(null)
      // gh 账号配置目录（只收绝对路径；留空 = 用宿主当前登录的账号）
      const [ghDir, setGhDir] = useState('')
      // Gitee 私人令牌（和 GitHub token 分开存；留空 = 公开仓库也能读）
      const [giteeToken, setGiteeToken] = useState('')
      // 判定页内联追溯结果（官方 Thread 详情最小吸收）：不再只塞进一条 Toast。
      const [traceResult, setTraceResult] = useState(null)
      // 每次追溯独占一个代际：新编号一开始，旧请求的结果/错误就全部作废。
      const traceSeqRef = useRef(0)
      // 安装回执与版本检查分开：POST 失败不能冲掉当前版本和命令回退。
      const [updateResult, setUpdateResult] = useState(null)
      // POST 后 GET 仍以 runningVersion 比较 latest；已装到 latest 且等待重载时，回执优先于 hasUpdate。
      const updateAwaitingReload = Boolean(updateResult && updateResult.needsReload === true
        && update && updateResult.installedVersion === update.latest)
      // 去设置页时必须同时定位分区；只切 Tab 会把人留在上次看的分区（官方 settingsRoute(group) 的最小等价）。
      const openSettings = (section) => {
        setSettingsSection(section)
        setTab('settings')
      }
      // 建议规则弹层：给"叫醒了没回音"一个下一步动作（只提供可复制的规则，不替客户写仓库文件）

      // 「我现在手上有没有能渲染的数据」。判断依据必须是**当前 snap**，不是"以前拿到过一次" ——
      // 会话作用域的 etag 只属于本会话；面板挂载时如果带着一份自己以前留下的 etag（快照已丢），
      // 首帧会 304。旧代码在 304 分支直接 return、从不 setSnap，于是面板永远停在
      // {loading:true, body:null}：统计条全 0、列表空态、底下还挂着「正在读取…」（真机现象：2 条显示 0 条）。
      const hasBodyRef = useRef(false)
      hasBodyRef.current = Boolean(snap.body)

      // 审核 M1：四条路径（轮询 / 工具栏刷新 / 错误卡重试 / run() 成功后）都能并发，
      // 没有序号时「后到的旧响应」会覆盖新数据。序号只认最新的那次。
      const loadSeqRef = useRef(0)
      // 终审 N1：被更新的一次 load 顶掉时，旧的那次不能把「成功」当失败 ——
      // run() 的 requireReload 会因此假报「回读失败，请重试」（数据其实已经取到了）。
      // 记下最新一次的 promise：被顶掉就等它，用最新快照回话；它真失败时仍然按失败返回。
      const latestLoadRef = useRef(null)
      const load = useCallback(() => {
        const seq = ++loadSeqRef.current
        const job = (async () => {
          const statePath = sessionId ? '/state?sessionId=' + encodeURIComponent(String(sessionId)) : '/state'
          const etag = readStateEtag(statePath)
          try {
            const payload = await call(statePath, etag ? { headers: { 'if-none-match': etag } } : undefined)
            // 已经有更新的一次请求在飞：等它的结果，别把自己的成功报成失败
            if (seq !== loadSeqRef.current) return await (latestLoadRef.current || null)
            // 304：内容没变，前提是本地已经有一份能渲染的 body
            if (payload && payload.notModified) {
              // 有本地快照时，304 也是一次成功回读；清掉此前的 500/网络错误。
              // 正常且无旧错时返回原 state，避免每次条件轮询都重渲染。
              if (hasBodyRef.current) {
                setSnap((prev) => prev.error || prev.loading
                  ? { loading: false, error: null, body: prev.body }
                  : prev)
                return payload
              }
              // 本地没有数据：这次 304 是被同一作用域旧 etag 带出来的，清掉它重取一次全量
              writeStateEtag(statePath, null)
              return load()
            }
            const body = payload
            setSnap({ loading: false, error: null, body })
            return body
          } catch (error) {
            // 旧请求的失败同样不盖新数据：等最新那次的结果（它失败就一并按失败返回）
            if (seq !== loadSeqRef.current) return await (latestLoadRef.current || null)
            const message = String((error && error.message) || error)
            // 同文轮询失败复用同一 snap/alert DOM；显式重试成功仍会清错，错误文本变化仍会通知。
            setSnap((prev) => prev.error === message && !prev.loading
              ? prev
              : { loading: false, error: message, body: prev.body })
            return null
          }
        })()
        latestLoadRef.current = job
        return job
      }, [sessionId])

      useEffect(() => { ensurePortedCss() }, [])
      usePoll(load, POLL_MS)

      const body = snap.body || {}
      // 默认值提示从后端读：写死 12/3/6 会在常量变化后变成谎话（JSON Schema 的 default 也是这个用法）
      const DEFAULTS = body.defaults || {}
      // 品牌与默认 ack 都来自 /state（issue #11）；缺省时保持 0.10.0 的文案。
      const BRAND = (typeof body.brandName === 'string' && body.brandName.trim()) || '主理鱼'
      const ACK_DEFAULTS = body.defaultAckLabels || null
      const watchers = body.watchers || []
      // 行内编辑只有这一条入口：clean 维持原直达行为；dirty 先确认，取消绝不重置 draft/touched。
      const applyWatcherEditor = (watcher, nextKey) => {
        const values = watcherValuesFrom(watcher, ACK_DEFAULTS)
        setEditing(nextKey)
        touchedRef.current = new Set()
        editSnapshotRef.current = nextKey ? { key: nextKey, values } : null
        setDraft({
          intervalSec: watcherNumberText(watcher.intervalSec ?? 60),
          labels: (watcher.filters && watcher.filters.labels ? watcher.filters.labels : []).join(','),
          ignoreLabels: (watcher.filters && watcher.filters.ignoreLabels ? watcher.filters.ignoreLabels : []).join(','),
          assignee: (watcher.filters && watcher.filters.assignee) || '',
          activeHours: watcher.activeHours || '',
          // `watcher.goal` 可能是 goalView 对象；输入框只能接收已展开的字符串。
          goal: watcher.note || (typeof watcher.goal === 'string' ? watcher.goal : (watcher.goal && watcher.goal.objective) || ''),
          ackLabel: watcher.ackLabel || ackLabelFallback(watcher, ACK_DEFAULTS),
          onlyAssigned: !!(watcher.filters && watcher.filters.onlyAssigned),
          allowAuthors: (watcher.filters && watcher.filters.allowAuthors ? watcher.filters.allowAuthors : []).join(', '),
          observe: !!watcher.observe,
          isolated: !!watcher.isolated,
          workdir: watcher.workdir || '',
          batchLimit: watcherNumberText(watcher.batchLimit),
          maxWakesPerHour: watcherNumberText(watcher.maxWakesPerHour),
          sessionMaxWakes: watcherNumberText(watcher.sessionMaxWakes),
          isolateKeepDays: watcherNumberText(watcher.isolateKeepDays),
          maxIsolated: watcherNumberText(watcher.maxIsolated),
          priorityReopened: (watcher.priority && watcher.priority.reopened) !== false,
          priorityComments: (watcher.priority && watcher.priority.comments) !== false,
          priorityLabels: (watcher.priority && watcher.priority.labels) !== false,
          priorityUnassigned: Boolean(watcher.priority && watcher.priority.unassigned) === true,
          priorityStale: Boolean(watcher.priority && watcher.priority.stale) === true,
        })
      }
      const editingWatcher = watchers.find((watcher) => watcher.repo + '#' + watcher.sessionId === editing)
      const editingOpening = editSnapshotRef.current && editSnapshotRef.current.key === editing
        ? editSnapshotRef.current.values
        : editingWatcher ? watcherValuesFrom(editingWatcher, ACK_DEFAULTS) : null
      const editingDirty = Boolean(editing && editingOpening && (
        touchedRef.current.size > 0 || watcherDirty({ ...editingOpening, ...draft }, editingOpening)))
      const pendingCollapseKey = glossary && glossary.type === 'watcher-draft' && !glossary.nextKey
        ? glossary.fromKey : null
      // dirty 折叠在确认期间挂起 editing id，但原表单仍挂载；取消/确认之前绝不清它的 draft。
      const displayedEditing = editing || pendingCollapseKey
      const toggleWatcherEditor = (watcher) => {
        const key = watcher.repo + '#' + watcher.sessionId
        const nextKey = editing === key ? null : key
        if (editing && editingDirty) {
          watcherDraftConfirmedRef.current = false
          setGlossary({ type: 'watcher-draft', watcher, nextKey, fromKey: editing })
          if (!nextKey) setEditing(null)
          return
        }
        applyWatcherEditor(watcher, nextKey)
      }
      const dismissWatcherDraft = () => {
        if (glossary && glossary.type === 'watcher-draft' && !glossary.nextKey
          && !watcherDraftConfirmedRef.current) setEditing(glossary.fromKey)
        watcherDraftConfirmedRef.current = false
        setGlossary(false)
      }
      const sessionGoal = body.sessionGoal || null
      const credentials = body.credentials || { token: 'none', sshKeyPath: 'none' }

      useEffect(() => {
        if (goalTouched) return
        // null 也是服务端事实：干净草稿要同步清空；dirty 草稿仍由用户自己决定保存或放弃。
        setGoalDraft(sessionGoal ? sessionGoal.objective || '' : '')
      }, [goalTouched, sessionGoal])

      const busyRef = useRef(false)
      const run = async (work, done, options = {}) => {
        // 审核 T22 B3：上一件事没跑完时点了会**静默丢弃**（界面毫无反应，客户以为按钮坏了）。
        // 至少说一句「还在处理」—— 不说的话没人知道该等。
        if (busyRef.current) {
          const error = new Error('上一件事还在处理，请稍等再点')
          setNotice({ ok: false, text: error.message })
          if (options.rethrow) throw error
          return false
        }
        busyRef.current = true
        setNotice(null)
        try {
          await work()
          if (options.requireReload) {
            const body = await load()
            if (!body) throw new Error(options.reloadError || '操作已提交，但回读失败，请重试')
            if (options.afterReload) options.afterReload(body)
          }
          // 必须回读的保存只有在确认成功后才提示；其它动作保留原有通知/刷新契约。
          if (done) notify(done)
          if (!options.requireReload) await load()
          return true
        } catch (error) {
          const text = String((error && error.message) || error)
          setNotice({ ok: false, text })
          notify(text, 'error')
          // 审核 T22 B3：失败后不回读 —— 批量操作「一半成功一半失败」时 UI 停在旧状态，
          // 客户看到的是「全都失败了」。回读一次，让已经成功的那些如实显示。
          try { await load() } catch { /* 回读失败不影响错误提示本身 */ }
          // WatcherForm 需要原始错误来留在自己的 SaveBar；其它动作保留旧的 false 契约。
          if (options.rethrow) throw error
          return false
        } finally {
          busyRef.current = false
        }
      }

      /** 保存某条 watcher 的编辑（同一 repo+sessionId 即更新，enabled 不传 = 保持原值）。 */
      const saveEdit = (watcher) => run(async () => {
        await call('/watch', {
          method: 'POST',
          // 只发 touched 字段；repo/sessionId 只是现有 /watch 的定位键，不是全量快照。
          body: JSON.stringify((() => {
            const full = {
              repo: watcher.repo,
              sessionId: watcher.sessionId,
              intervalSec: numberOrUndefined(draft.intervalSec) ?? watcher.intervalSec,
              filters: {
                labels: String(draft.labels ?? '').split(',').map((v) => v.trim()).filter(Boolean),
                // filters 整块发送，三个名单都必须带上，否则改标签会顺带清空另外两项。
                ignoreLabels: String(draft.ignoreLabels ?? '').split(',').map((v) => v.trim()).filter(Boolean),
                onlyAssigned: !!draft.onlyAssigned,
                assignee: String(draft.assignee ?? '').trim().replace(/^@/, '') || null,
                allowAuthors: String(draft.allowAuthors ?? '').split(',').map((v) => v.trim().replace(/^@/, '')).filter(Boolean),
              },
              activeHours: draft.activeHours || '',
              goal: draft.goal || '',
              ackLabel: String(draft.ackLabel ?? '').trim() || undefined,
              isolated: !!draft.isolated,
              workdir: draft.workdir || '',
              observe: !!draft.observe,
              isolateKeepDays: numberOrUndefined(draft.isolateKeepDays),
              maxIsolated: numberOrUndefined(draft.maxIsolated),
              priority: {
                reopened: draft.priorityReopened !== false,
                comments: draft.priorityComments !== false,
                labels: draft.priorityLabels !== false,
                unassigned: draft.priorityUnassigned === true,
                stale: draft.priorityStale === true,
              },
              batchLimit: numberOrUndefined(draft.batchLimit),
              maxWakesPerHour: numberOrUndefined(draft.maxWakesPerHour),
              sessionMaxWakes: numberOrUndefined(draft.sessionMaxWakes),
            }
            // 控件名 → API 字段名：一个 filters/priority 子项被触碰时保留整块其它原值。
            const CONTROL_TO_FIELD = {
              labels: "filters",
              ignoreLabels: "filters",
              onlyAssigned: "filters",
              assignee: "filters",
              allowAuthors: "filters",
              priorityReopened: "priority",
              priorityComments: "priority",
              priorityLabels: "priority",
              priorityUnassigned: "priority",
              priorityStale: "priority",
            }
            const wanted = new Set()
            for (const key of touchedRef.current) {
              wanted.add(CONTROL_TO_FIELD[key] || key)
            }
            const out = { repo: full.repo, sessionId: full.sessionId }
            for (const key of wanted) {
              if (Object.prototype.hasOwnProperty.call(full, key)) out[key] = full[key]
            }
            return out
          })()),
        })
        setEditing(null)
        touchedRef.current = new Set()
        editSnapshotRef.current = null
      }, '已保存：' + watcher.repo, { rethrow: true })

      const act = (watcher, action) => run(async () => {
        await call('/watch', { method: 'POST', body: JSON.stringify({ action, id: watcherId(watcher) }) })
      }, action === 'delete' ? '已删除 ' + watcher.repo
        : action === 'poll' ? '已触发一次轮询：' + watcher.repo
        // 审核 T24 W6：暂停/恢复成功以前什么都不说，客户不确定点没点上。
        : action === 'pause' ? '已暂停：' + watcher.repo
        : action === 'resume' ? '已恢复：' + watcher.repo
        : null)

      // 审核 T24 W1：title 写「这一屏」，实现却遍历全部 watchers —— 筛到 1 条点「全部暂停」会发出 2 条 POST。
      // 让按钮对**看见的那些**生效（与它的文案同口径）。
      const actBulk = (action, list = watchers) => run(async () => {
        for (const watcher of list) {
          if (action === 'pause' && !watcher.enabled) continue
          if (action === 'resume' && watcher.enabled) continue
          await call('/watch', { method: 'POST', body: JSON.stringify({ action, id: watcherId(watcher) }) })
        }
      }, action === 'pause' ? '全部已暂停' : '全部已恢复')

      const addWatcher = () => run(async () => {
        const value = { ...CREATE_WATCHER_VALUES, ...form }
        const created = await call('/watch', {
          method: 'POST',
          body: JSON.stringify((() => {
            const valueDirty = (key) => createTouchedRef.current.has(key) || !watcherSameValue(value[key], createSnapshotRef.current[key])
            const payload = { repo: value.repo.trim(), sessionId }
            // 新建也必须只提交实际输入过的配置；服务端负责未提供项的默认值。
            if (valueDirty('intervalSec')) payload.intervalSec = numberOrUndefined(value.intervalSec) ?? 60
            if (valueDirty('goal')) payload.goal = value.goal || ''
            if (['labels', 'ignoreLabels', 'onlyAssigned', 'assignee', 'allowAuthors'].some(valueDirty)) {
              payload.filters = {
                labels: String(value.labels ?? '').split(',').map((v) => v.trim()).filter(Boolean),
                ignoreLabels: String(value.ignoreLabels ?? '').split(',').map((v) => v.trim()).filter(Boolean),
                onlyAssigned: !!value.onlyAssigned,
                assignee: String(value.assignee ?? '').trim().replace(/^@/, '') || null,
                allowAuthors: String(value.allowAuthors ?? '').split(',').map((v) => v.trim().replace(/^@/, '')).filter(Boolean),
              }
            }
            if (valueDirty('activeHours')) payload.activeHours = value.activeHours || ''
            if (valueDirty('ackLabel')) payload.ackLabel = String(value.ackLabel ?? '').trim() || undefined
            if (valueDirty('isolated')) payload.isolated = !!value.isolated
            if (valueDirty('workdir')) payload.workdir = value.workdir || ''
            if (valueDirty('observe')) payload.observe = !!value.observe
            if (['priorityReopened', 'priorityComments', 'priorityLabels', 'priorityUnassigned', 'priorityStale'].some(valueDirty)) {
              payload.priority = {
                reopened: value.priorityReopened !== false,
                comments: value.priorityComments !== false,
                labels: value.priorityLabels !== false,
                unassigned: value.priorityUnassigned === true,
                stale: value.priorityStale === true,
              }
            }
            for (const key of ['batchLimit', 'maxWakesPerHour', 'sessionMaxWakes', 'isolateKeepDays', 'maxIsolated']) {
              if (valueDirty(key)) payload[key] = numberOrUndefined(value[key])
            }
            return payload
          })()),
        })
        // 只重置既定仓库相关字段；限额、隔离策略和时段保留，连续添加不用重填。
        const nextForm = { ...form, repo: '', goal: '', labels: '', onlyAssigned: false, allowAuthors: '' }
        setForm(nextForm)
        // 这些保留下来的值已经由本次 /watch 保存，成为下一次新增的全新基线。
        createSnapshotRef.current = { ...CREATE_WATCHER_VALUES, ...nextForm }
        createTouchedRef.current = new Set()
        const warnings = (created && Array.isArray(created.warnings)) ? created.warnings : []
        if (warnings.length > 0) notify('已添加，但注意：' + warnings.join(' ／ '), 'error')
        else notify('已添加，首轮立即开始轮询', 'info')
        return warnings
      }, null, { rethrow: true })

      const credentialsDirty = Boolean(token.trim() || giteeToken.trim() || ghDir.trim())
      const saveCredentials = () => {
        // 空草稿没有可保存的改动；即使绕过 disabled 直接调用，也不能发 {} 后假报成功。
        if (!credentialsDirty) return Promise.resolve(false)
        const draftVersion = credentialsDraftVersionRef.current
        let saveSeq = 0
        return run(async () => {
          saveSeq = ++credentialsSaveSeqRef.current
          const payload = {}
          if (token.trim()) payload.token = token.trim()
          if (giteeToken.trim()) payload.giteeToken = giteeToken.trim()
          // 审核 T24 M3：这里以前无条件带 "ghConfigDir" —— ghDir 初值恒为空串且从不回填服务端值，
          // 于是「只改 token」也会发 ghConfigDir:""，服务端把它当「显式清空」，
          // 已绑定的 gh 账号目录被静默解绑（评论改回宿主账号）。
          // 现在只有**填了东西**才发；要解绑请用旁边的「清除」按钮（它显式发空串）。
          if (ghDir.trim()) payload.ghConfigDir = ghDir.trim()
          await call('/credentials', { method: 'POST', body: JSON.stringify(payload) })
        }, '凭证已保存', {
          requireReload: true,
          reloadError: '凭证已提交，但回读失败，请重试',
          afterReload: () => {
            if (saveSeq !== credentialsSaveSeqRef.current || draftVersion !== credentialsDraftVersionRef.current) return
            setToken('')
            setGhDir('')
            setGiteeToken('')
          },
        })
      }

      /** 显式解绑 gh 账号目录（T24 M3：不能在「只改 token」时顺带清掉）。 */
      const clearGhDir = () => run(async () => {
        await call('/credentials', { method: 'POST', body: JSON.stringify({ ghConfigDir: '' }) })
      }, '已解绑 gh 账号目录（评论改回宿主账号）')

      const saveGoal = () => {
        // 与按钮/文本域的 disabled 同一道防线：没有会话归属时不能提交或假报成功。
        if (!sessionId) return Promise.resolve(false)
        const draftVersion = goalDraftVersionRef.current
        let saveSeq = 0
        return run(async () => {
          saveSeq = ++goalSaveSeqRef.current
          await call('/goal', { method: 'POST', body: JSON.stringify({ sessionId, objective: goalDraft }) })
        }, 'goal 已保存', {
          requireReload: true,
          reloadError: 'goal 已提交，但回读失败，请重试',
          afterReload: () => {
            if (saveSeq !== goalSaveSeqRef.current || draftVersion !== goalDraftVersionRef.current) return
            setGoalTouched(false)
          },
        })
      }

      // 四个数字必须加得上总数（以前「已暂停」按 enabled 数、「异常」按 lastError 数：
      // 一条已暂停又有错误的守会被数两次，而「时段外」根本没被数）
      const byState = { active: 0, paused: 0, error: 0, dormant: 0 }
      for (const w of watchers) byState[orbitStateOf(w)] += 1
      const running = byState.active
      const broken = byState.error
      const paused = byState.paused
      const dormant = byState.dormant
      // 可暂停的守 = 还在 enabled 的那些。actBulk('pause') 正是按这个条件跳过的，
      // 按钮的可用状态必须跟它同口径 —— 不同口径就会出现「可点、但一条都没动」。
      // 可暂停 = **当前这一屏**里还在轮询的（与 actBulk 的入参同口径）
      const countPausable = (list) => list.filter((w) => w.enabled).length

      const stateTag = (watcher) => {
        const state = orbitStateOf(watcher)
        const tone = state === 'active' ? 'success' : state === 'paused' ? 'warning' : state === 'dormant' ? 'outline' : 'danger'
        return h(Tag, { tone, title: state === 'dormant' ? '不在活跃时段：只攒条目，不唤醒' : undefined }, STATE_TEXT[state])
      }

      // 暂停原因：以前它是卡片第 5 行的一句普通字（14px，和「轮询 10 分钟前」同字号同色），
      // 扫一眼只看到琥珀色的「已暂停」胶囊，看不出这条守为什么停着。
      // 现在它跟在标题后面单独一行、加粗 + 警告色（.dt-ticket-note-pause）；
      // 同时全面板只保留这一处 —— 「需要关注」以前又抄了一遍同一句（同一屏出现两次）。
      // 冒号只留一个：后端句子自带冒号时（「不在活跃时段（09:00-23:00 之外）：只攒条目，不唤醒」），
      // 再拼上「暂停唤醒：」就是两个冒号；句内冒号降成破折号，「之外」也多余（「不在…之外」是重复表达）。
      const pauseNote = (watcher) => {
        const raw = String(watcher.lastDeferred || '').trim()
        const reason = raw
          // 只降全角冒号：时钟 09:00 的冒号是半角，动了就成了「09—— 00」
          ? raw.replace(/（([^）]*?)\s*之外）/, '（$1）').replace(/：/g, '—— ')
          : (watcher.enabled === false ? '已手动停用 —— 不会轮询，也不会唤醒；点「恢复」继续' : '')
        if (!reason) return null
        return h('div', { className: 'dt-ticket-note dt-ticket-note-pause' }, '暂停唤醒：' + reason)
      }

      // 行结构对齐官方 rows.tsx 的 TicketLine：dt-ticket（左侧状态列）+ dt-open 标题 + dt-ticket-actions。
      // 标题本身是按钮 —— 官方就是点标题进详情，我们点标题展开行内编辑，交互更轻但不脱离这套结构。
      const STAGE_STATE = { active: undefined, paused: 'warning', error: 'error', dormant: 'warning' }
      const row = (watcher) => {
        const key = watcher.repo + '#' + watcher.sessionId
        const state = orbitStateOf(watcher)
        const rowBody = h('div', { className: 'dt-ticket-body' },
          h('div', { className: 'dt-row-title' },
            h('button', {
              type: 'button', className: 'dt-open', title: '展开或收起这条' + BRAND + '的设置',
              'aria-expanded': displayedEditing === key, onClick: () => toggleWatcherEditor(watcher),
            }, watcher.repo)),
          pauseNote(watcher),
          h('div', { className: 'dt-row-meta dt-ticket-meta' },
            stateTag(watcher),
            ...repoBadges(watcher.repoAccess).map((badge) => h(Pill, {
              key: 'b-' + badge.text,
              active: badgeFilter === badge.text,
              onClick: () => setBadgeFilter(badgeFilter === badge.text ? null : badge.text),
            }, badge.text)),
            watcher.observe ? h(Tag, { tone: 'warning' }, '仅观察') : null,
            watcher.activeHours ? h(Tag, null, '时段 ' + watcher.activeHours) : null,
            watcher.isolated ? h(Tag, null, isolationText(watcher)) : null,
            filterText(watcher.filters) ? h(Tag, null, filterText(watcher.filters)) : null,
            // ack 标签决定这条是否还会被叫醒，不能只在工具参数里出现。
            h(Tag, { tone: 'quiet' }, '回执 ' + (watcher.ackLabel || ackLabelFallback(watcher, ACK_DEFAULTS))),
            watcher.queuedWakes > 0 ? h(Tag, { tone: 'warning' }, '排队 ' + watcher.queuedWakes) : null,
            backoffText(watcher) ? h(Tag, { tone: 'warning' }, backoffText(watcher)) : null),
          // 三行，各管一件事：① 是谁 ② 节律与最近结果 ③ 目标
          // 原来六段用 ' · ' 串成一行（会话 · 每 60s · 轮询 · 唤醒 · 上次唤醒 · 目标），
          // 出图时那行在浅色主题下折成两行、读起来像一堵墙。
          // 标题行已经写了仓库名；这里只补「会话 + 间隔」，并把最近一次结果并进来
          // （原来这一行和下面那行各占 20px，其实说的是同一件事：这个守在干什么、最近怎么样）
          h('div', { className: 'dt-ticket-note' },
            (watcher.title || '未命名会话') + ' · ' + sessionTag(watcher.sessionId) + ' · 每 ' + (Number(watcher.intervalSec) || 60) + 's'
            + ' · 轮询 ' + ageText(watcher.lastPoll)),
          watcher.lastWake
            ? h('div', { className: 'dt-ticket-note' },
              '唤醒 ' + wakeText(watcher.lastWake) + (historyText(watcher) ? ' · ' + historyText(watcher) : ''))
            : null,
          watcher.note ? h('div', { className: 'dt-ticket-note' }, '目标: ' + String(watcher.note).slice(0, 40)) : null,
          (watcher.otherSessions && watcher.otherSessions.length > 0)
            ? h('div', { className: 'dt-ticket-note' }, '还有 ' + watcher.otherSessions.length + ' 个会话也在守这个仓库：' + watcher.otherSessions.join('、') + '（各自独立唤醒；但 ack 标签是仓库级的 —— 任一方 ack 后两边都会跳过，想让两边各处理各的就给对方设不同标签）')
            : null,
          (() => {
            const fails = Number(watcher.consecutiveFailures) || 0
            if (fails < 1 || !watcher.nextAt) return null
            // 让「为什么很久没动」写在面板上：K8s 的 Job 也是把 backoff 状态暴露在 status 里
            const wait = Math.max(0, Date.now() - Date.parse(watcher.nextAt))
            const left = Date.parse(watcher.nextAt) - Date.now()
            const mins = Math.max(0, Math.round((left > 0 ? left : wait) / 60000))
            // 档位文案从后端给的 retryPolicy 算：写死「2 小时」会在有人改常量后变成谎话
            const policy = (body && body.retryPolicy) || {}
            const budget = Number(policy.budgetAttempts) || 6
            const capMin = Number(policy.slowCapMinutes) || 120
            const capText = capMin >= 60 ? (capMin / 60) + ' 小时' : capMin + ' 分钟'
            const slow = fails >= budget ? '（已放慢到 ' + capText + '档）' : ''
            return h('div', { className: 'dt-hint' },
              '已连续失败 ' + fails + ' 次 · ' + (left > 0 ? '下次重试约 ' + mins + ' 分钟后' : '等待重试') + slow)
          })(),
          watcher.needsReauth ? h('div', { className: 'dt-banner dt-danger' },
            '凭证失效：跑 gh auth login 重新登录 —— 它不会自己恢复，在此之前这条守只会一直失败') : null,
          watcher.lastError ? (() => {
            const parts = splitErrorText(watcher.lastError)
            // 错误光说「为什么」不够，要能点一下就到「下一步」（服务端已给 kind）
            const FIX = {
              auth: { label: '去填 token', onClick: () => openSettings('creds') },
              moved: { label: '改仓库地址', onClick: () => setTab('trigger') },
              notfound: { label: '改仓库地址', onClick: () => setTab('trigger') },
              permission: { label: '去填 token', onClick: () => openSettings('creds') },
              // 本地解析错误：重试没用，只能让人把这段贴出来
              'local-parse': { label: '看诊断', onClick: () => openSettings('diagnose') },
              network: { label: '立刻重试', onClick: () => act(watcher, 'poll') },
              ratelimit: { label: '立刻重试', onClick: () => act(watcher, 'poll') },
              gateway: { label: '立刻重试', onClick: () => act(watcher, 'poll') },
            }
            const fix = FIX[watcher.lastErrorKind] || null
            const errAge = watcher.lastErrorAt ? ageText(watcher.lastErrorAt) : ''
            const staleErr = watcher.lastErrorAt && (Date.now() - Date.parse(watcher.lastErrorAt)) > 3600 * 1000
            return h('div', { className: 'dt-ticket-note dt-ticket-note-error' },
              (errAge && errAge !== '—' ? (staleErr ? '旧记录（' + errAge + '）：' : '(' + errAge + ') ') : '')
              + parts.main.slice(0, 120) + (parts.main.length > 120 ? '…' : ''),
              parts.hint ? h('div', { className: 'dt-hint' }, parts.hint) : null,
              fix ? h('div', { className: 'dt-actions' }, h(Btn, { size: 'sm', variant: 'outline', 'aria-label': fix.label, onClick: fix.onClick }, fix.label)) : null)
          })() : null,
          h('div', { className: 'dt-ticket-actions' },
            h(Btn, { size: 'sm', variant: 'outline', icon: h(Icon, { name: 'refresh' }), onClick: () => act(watcher, 'poll') }, '立刻'),
            h(Btn, {
              size: 'sm', variant: 'ghost',
              icon: h(Icon, { name: watcher.enabled ? 'pause' : 'play' }),
              label: watcher.enabled ? '暂停' : '恢复',
              onClick: () => act(watcher, watcher.enabled ? 'pause' : 'resume'),
            }, watcher.enabled ? '暂停' : '恢复'),
            h(ConfirmButton, {
              size: 'sm', variant: 'ghost', icon: h(Icon, { name: 'trash' }), label: '删除',
              confirmLabel: '确认删除', onConfirm: () => act(watcher, 'delete'),
            })))
        const opening = editSnapshotRef.current && editSnapshotRef.current.key === key
          ? editSnapshotRef.current.values
          : watcherValuesFrom(watcher, ACK_DEFAULTS)
        const editorValue = { ...opening, ...draft }
        const editor = displayedEditing === key
          ? h(WatcherForm, {
            mode: 'edit', idPrefix: 'watcher-edit-' + encodeURIComponent(key),
            value: editorValue, snapshot: opening, defaults: DEFAULTS, sessionId,
            submitLabel: '保存',
            onChange: (name, next) => setField({ ...draft, [name]: next }),
            onSave: () => saveEdit(watcher),
            onDiscard: (snapshot) => {
              setDraft({ ...snapshot })
              touchedRef.current = new Set()
              setNotice(null)
            },
          })
          : null
        // 官方 TicketLine 是 <li class="dt-row dt-link dt-ticket"> 里套「状态列 + 正文列」，
        // 展开的表单跟在正文列里，不破坏两列网格。
        const bodyChildren = [].concat(rowBody.props.children || []).filter(Boolean)
        // 把「立刻/暂停/删除」并进徽章行的右侧：动作原本独占一行（30px + gap ≈ 22% 行高）。
        // 这里用组合的方式改，而不是剪贴 JSX —— 上一版靠文本替换搬这段，括号错位了两次。
        {
          const has = (node, name) => node && node.props && String(node.props.className || '').includes(name)
          const actionsAt = bodyChildren.findIndex((n) => has(n, 'dt-ticket-actions'))
          const metaAt = bodyChildren.findIndex((n) => has(n, 'dt-ticket-meta'))
          if (actionsAt >= 0 && metaAt >= 0) {
            const actions = bodyChildren[actionsAt]
            const meta = bodyChildren[metaAt]
            bodyChildren[metaAt] = {
              ...meta,
              props: { ...meta.props, children: [...(meta.props.children || []), actions] },
            }
            bodyChildren.splice(actionsAt, 1)
          }
        }
        return h('li', {
          key,
          className: 'dt-row dt-link dt-ticket',
          'data-state': state === 'error' ? 'error' : undefined,
        },
          h('div', { className: 'dt-ticket-stage' },
            h('span', { className: 'dt-stage', 'data-state': STAGE_STATE[state] }, STATE_TEXT[state])),
          h('div', { className: 'dt-ticket-body' }, ...bodyChildren, editor))
      }

      const problems = []
      // 按暂停原因筛选时，「需要关注」也一起收窄（否则列表和告警会自相矛盾）
      const deferFilter = reasonFilter && reasonFilter.startsWith('defer:') ? reasonFilter.slice(6) : null
      for (const watcher of watchers) {
        if (deferFilter && watcher.deferReason !== deferFilter) continue
        // 仓库结构问题（关闭 Issues / 是 fork）比轮询失败更早该说：它注定收不到条目
        const structure = repoStructureNote(watcher.repoAccess)
        if (structure) problems.push({ key: 's-' + watcher.repo, text: structure, rank: 3, kind: 'structure' })
        // 只读：以前只在行内显示一个徽章，客户在「需要关注」里看不到，也不知道回写会 403
        const readOnly = accessNoteForPanel(watcher.repoAccess, watcher.repo)
        if (readOnly) problems.push({ key: 'ro-' + watcher.repo, text: readOnly, rank: 4, kind: 'readonly' })
        // 叫醒了但没回音：守在工作、消息也进去了，但会话没动手（规则缺失或被忽略）
        if (Number(watcher.silentWakes) >= 2) {
          problems.push({
            key: 'silent-' + watcher.repo,
            rank: 2, kind: 'silent',
            text: watcher.repo + ' 连续 ' + watcher.silentWakes + ' 次被叫醒后没有动作（没看到 watch_ack 标签）—— 会话可能没按规则处理',
            action: { label: '看建议规则', onClick: () => setGlossary('rules:' + watcher.repo) },
          })
        }
        // 被反复重开的条目：说明"之前判过不做"可能有问题，值得人看一眼
        if (watcher.lastReopened) {
          problems.push({ key: 'r-' + watcher.repo, text: watcher.repo + ' #' + watcher.lastReopened + ' 被重新打开过 —— 之前判"不做"可能不对', rank: 5, kind: 'reopened' })
        }
        if (watcher.lastError) {
          // 同样拆开：现象截断、建议保留（这里原来是整串塞进「需要关注」）
          const parts = splitErrorText(watcher.lastError)
          problems.push({
            key: 'e-' + watcher.repo,
            rank: 1, kind: 'error',
            // 审核 T33 M3：以前一律写「轮询失败」—— 而唤醒失败时轮询是**成功的**，归错了类。
            // `wake:` 前缀由 lib/watch.js 的 planWakeSettlement 打上。
            text: watcher.repo + (isWakeFailure(watcher.lastError) ? ' 唤醒失败：' : ' 轮询失败：')
              + parts.main.replace(/^wake:\s*/, '').slice(0, 120) + (parts.main.length > 120 ? '…' : '') + ' ' + parts.hint,
          })
        }
        // 暂停原因不在「需要关注」里重复：卡片里已经有一行加粗警告色的「暂停唤醒：…」（见 pauseNote），
        // 两处并排会让同一句话在同一屏出现两次；而且「时段外 / 已停用」是配置出来的状态、不是待处理的问题。
      }
      if (body.gh && body.gh.ok === false && credentials.token === 'none') {
        problems.push({ key: 'gh', rank: 0, kind: 'gh', text: 'gh CLI 不可用，且没有配置 token —— 现在所有轮询都会失败。先跑 gh auth login，或在「设置 → 凭证」填一个 token。' })
      }
      if (watchers.length > 0 && credentials.token === 'none') {
        problems.push({ key: 'cred', rank: 7, kind: 'cred', text: '未配置 token，正在复用已登录的 gh CLI（可用 panel 填 token 换成独立凭据）' })
      }

      const needle = search.trim().toLowerCase()
      const matches = (watcher) => !needle
        || watcher.repo.toLowerCase().includes(needle)
        || (watcher.title || '').toLowerCase().includes(needle)
        || (watcher.filteredLog || []).some((entry) => String(entry.number).includes(needle) || String(entry.title || '').toLowerCase().includes(needle))
      const matchesBadge = (watcher) => !badgeFilter
        || repoBadges(watcher.repoAccess).some((badge) => badge.text === badgeFilter)
      const hasProblem = (watcher) => Boolean(watcher.lastError)
        || Boolean(watcher.lastReopened)
        || Number(watcher.silentWakes) >= 2
        || Boolean(repoStructureNote(watcher.repoAccess))
        || repoBadges(watcher.repoAccess).some((badge) => badge.tone === 'warning' || badge.tone === 'danger')
      const stateMatches = (watcher) => {
        const state = orbitStateOf(watcher)
        if (stateFilter === 'active') return state === 'active'
        if (stateFilter === 'paused') return state === 'paused'
        if (stateFilter === 'error') return state === 'error'
        if (stateFilter === 'dormant') return state === 'dormant'
        return true
      }
      const visible = watchers.filter((watcher) => matches(watcher) && matchesBadge(watcher) && stateMatches(watcher)
        && (!problemsOnly || hasProblem(watcher))
        && (!deferFilter || watcher.deferReason === deferFilter))
      const hasActiveFilters = Boolean(needle || badgeFilter || stateFilter || problemsOnly || deferFilter)
      const clearAllFilters = () => {
        setBadgeFilter(null); setStateFilter(null); setSearch(''); setProblemsOnly(false); setReasonFilter(null)
      }

      // T24 W1：「全部暂停/全部恢复」对**这一屏**生效，可用性也按这一屏算（同口径）。
      const pausable = countPausable(visible)
      const resumable = visible.length - pausable

      const filterSummary = (() => {
        // 筛选汇总：叠加时用一行说清「应用了哪几个」，而不是每条各占一行。
        // 只有一个筛选时仍然是一句「只看 X 的守（N 条）：清除」。
          const chips = []
          if (badgeFilter) chips.push({ label: badgeFilter, clear: () => setBadgeFilter(null) })
          if (stateFilter) chips.push({ label: STATE_TEXT[stateFilter], clear: () => setStateFilter(null) })
          if (needle) chips.push({ label: '搜索「' + needle + '」', clear: () => setSearch('') })
          if (problemsOnly) chips.push({ label: '有问题', clear: () => setProblemsOnly(false) })
          if (deferFilter) chips.push({ label: '暂停原因「' + (DEFER_LABELS[deferFilter] || deferFilter) + '」', clear: () => setReasonFilter(null) })
          if (chips.length === 0) return null
          return h('div', { className: 'dt-row-meta', key: 'filter-summary' },
            (() => {
              // 拼成一个文本节点：拆成多段会让文案断言断在标签边界上
              const head = chips.length > 1
                ? '已应用 ' + chips.length + ' 个筛选'
                : '只看 ' + chips.map((c) => c.label).join('、')
              return h('span', null, head + ' 的守（' + visible.length + ' 条）：')
            })(),
            ...chips.map((chip) => h(Pill, { key: chip.label, onClick: chip.clear }, chip.label + ' ✕')),
            chips.length > 1
              ? h(Pill, { key: '__all', onClick: clearAllFilters }, '全部清除')
              : null)
      })()

      // 《为什么做这个产品》的三个成功指标：叫醒之后**有没有动作**。
      // 没有这个数，「不打断」只是信念 —— 你没法知道自己在优化还是在变吵。
      const qualityTotals = watchers.reduce((acc, w) => {
        const q = w.quality
        if (q && typeof q === 'object') {
          acc.woken += Number(q.itemsWoken) || 0
          acc.acked += Number(q.itemsAcked) || 0
        }
        return acc
      }, { woken: 0, acked: 0 })

      // 服务端已限制每守带回的记录数；面板在全局合并后排序，不能按守各切两条再拼接。
      const filteredEntries = watchers
        .flatMap((watcher) => (watcher.filteredLog || []).map((entry, sourceIndex) => ({ watcher, entry, sourceIndex })))
        .sort((a, b) => String(b.entry.at || '').localeCompare(String(a.entry.at || '')))

      const overview = [
        filterSummary,
        h('div', { className: 'dt-stats', key: 'stats' },
          h(Stat, {
            key: 'all', value: watchers.length, unit: '条' + BRAND,
            active: stateFilter === null, onClick: () => setStateFilter(null),
          }),
          h(Stat, {
            key: 'run', value: running, unit: '主理中',
            active: stateFilter === 'active', onClick: () => setStateFilter(stateFilter === 'active' ? null : 'active'),
          }),
          h(Stat, {
            key: 'pause', value: paused, unit: '已暂停',
            active: stateFilter === 'paused', onClick: () => setStateFilter(stateFilter === 'paused' ? null : 'paused'),
          }),
          h(Stat, {
            key: 'err', value: broken, unit: '异常',
            active: stateFilter === 'error', onClick: () => setStateFilter(stateFilter === 'error' ? null : 'error'),
          }),
          // 叫醒之后有动作的比例（有 ack 的条数 / 叫醒过的条数）。
          // 低于 0.7 就是「叫醒的是噪音」—— 这时候该做的是调过滤/优先级，不是叫醒更多。
          qualityTotals.woken > 0 ? h(Stat, {
            key: 'acted', value: qualityTotals.acked + ' / ' + qualityTotals.woken, unit: '有动作（ack）',
          }) : null,
          dormant > 0 ? h(Stat, {
            key: 'dormant', value: dormant, unit: '时段外',
            active: stateFilter === 'dormant', onClick: () => setStateFilter(stateFilter === 'dormant' ? null : 'dormant'),
          }) : null),
        filteredEntries.length > 0
          ? h(Section, { key: 'filtered', title: '最近被过滤' },
            h(CollapsibleList, {
              title: '看被过滤的条目',
              items: filteredEntries.map(({ watcher, entry, sourceIndex }) => {
                const head = watcher.repo + ' · 会话 ' + sessionTag(watcher.sessionId)
                  + ' · #' + String(entry.number ?? '—') + ' ' + (entry.title || '') + ' — ' + entry.reason
                const at = String(entry.at || '')
                return {
                  key: ['filtered', watcher.sessionId, watcher.repo, entry.number, entry.at, entry.reason, sourceIndex]
                    .map((part) => String(part ?? '')).join('|'),
                  summary: head + ' · ' + (at || '时间未知'),
                  content: h(React.Fragment, null, head, ' · ',
                    h('time', { dateTime: at || undefined, title: at || undefined }, at || '时间未知')),
                }
              }),
            }))
          : null,
        // 排序：会丢消息的（失败/没回音）排前面；静态提示（仓库归档、只读）排后面
        (() => {
          // 审核 T34：知识库那条「同一条只出现一次」从 0.9.87 落地起就**没成立过** ——
          // 当时只删掉了循环体内重复 push 的那一行，从未按 key 去重。
          // 而面板拿到的是**全部会话**的守：两个会话守同一仓库，就会出两条一字不差的条目。
          // 排序前按 key 去重（保留第一条：先 push 的 rank 更高）。
          const deduped = [...new Map(problems.map((item) => [item.key, item])).values()]
          problems.length = 0
          problems.push(...deduped)
          problems.sort((a, b) => ((a.rank ?? 9) - (b.rank ?? 9)) || String(a.text).localeCompare(String(b.text)))
          return problems.length > 0
        })()
          // 审核 T24 W4：这里的 count 是**条目数**（一条守可能贡献多条），而「只看有问题的 (N)」是**守的条数**。
        // 两个数字口径不同是对的，但要说清，否则客户会以为算错了。
        ? h(Section, { key: 'problems', title: '需要关注', count: problems.length, subtitle: '共 ' + problems.length + ' 条问题，涉及 ' + watchers.filter(hasProblem).length + ' 条守' },
            // 审核 T24 M4：以前只渲染 item.text，构造好的 item.action（「看建议规则」）没人消费 ——
            // 于是那个按钮永不出现，而 AGENTS.md 建议弹层唯一的入口就是它。
            h('ul', { className: 'dt-plain-list' }, problems.map((item) => h('li', { key: item.key, className: 'dt-danger' },
              item.text,
              item.action ? h(Btn, { variant: 'link', size: 'sm', 'aria-label': item.action.label, onClick: item.action.onClick }, item.action.label) : null))))
          : null,
        // 中间态：已经有守、但一次都没唤醒过 —— 这不是"没配置"，是"在等"
        (watchers.length > 0 && watchers.every((w) => !w.lastWake))
          ? h('div', { className: 'sa-dock-foot', key: 'waiting' },
            '还没有唤醒过：守会在仓库出现新条目时叫醒这个会话。'
            + '想先试一次，点行里的「立刻」；如果一直没动静，看上面「需要关注」里有没有被过滤的记录。'
            + '叫醒之后：会话先读仓库 AGENTS.md → 决定「修 / 回 / 待定」→ 做完写回并 watch_ack 打标签；'
            + '不做也要说明理由并 ack。每次只处理叫醒消息里列出的那几条，不会扩大范围。')
          : null,
        // 需要重启：这是"功能看起来没生效"最常见的根因，必须显眼
        (body.installedVersion && body.runningVersion && body.installedVersion !== body.runningVersion)
          ? h('div', { className: 'dt-banner dt-danger', key: 'restart-needed' },
            '代码已更新到 ' + body.installedVersion + '，但宿主还在跑 ' + body.runningVersion
            + ' —— 服务端改动需要重启宿主才生效（客户端部分已生效，所以界面可能看起来是新的）。')
          : null,
        h(Section, { key: 'list', title: needle ? '搜索结果' : BRAND, count: visible.length },
          watchers.length === 0
            ? h(Onboarding, { onStart: () => setTab('trigger') })
            : visible.length === 0
              ? h('div', { className: 'dt-empty' },
                h(Hint, null, needle
                  ? '没有匹配「' + search.trim() + '」的' + BRAND + '。'
                  : hasActiveFilters
                    ? '没有符合当前筛选条件的' + BRAND + '。'
                    : '当前没有可显示的' + BRAND + '。'),
                hasActiveFilters
                  ? h('div', { className: 'dt-actions' },
                    h(Btn, { size: 'sm', variant: 'outline', onClick: clearAllFilters }, '清除全部筛选'))
                  : null)
              : h('ul', { className: 'dt-list' }, visible.map(row))),
      ]

      // 判定日志（对应官方 eval 页）：每轮的唤醒结果、跳过原因、被过滤记录，一次看全。
      // 结构对齐官方 rows.tsx DecisionLine：ul.dt-list > li.dt-row.dt-ticket；
      // ol.dt-timeline 只保留给追溯与运行时事件，不再把不同语义混成一种列表。
      // 跳过原因分布：把「为什么没叫醒」做成可读的统计（客户最常问的问题）
      const reasonTotals = (() => {
        const merged = {}
        for (const w of watchers) for (const [reason, n] of Object.entries(w.filterCounts || {})) {
          merged[reason] = (Number(merged[reason]) || 0) + (Number(n) || 0)
        }
        return Object.entries(merged).sort((a, b) => b[1] - a[1]).slice(0, 8)
      })()
      const judge = h(Section, {
        title: '判定日志（最近 5 次唤醒）',
        count: watchers.reduce((n, w) => n + (w.history || []).length, 0),
        action: h(InfoTip, { text: '每轮判定都会记一条：唤醒、仅观察，或跳过及其原因。只存最近若干条。' }),
      },
        (() => {
          const needle = search.trim()
          // 审核 T40 M3：以前只认纯数字 —— Gitee 的编号是 IKSQJH 这种字符串，
          // 于是**连追溯按钮都不生成**。而「这条为什么没叫醒我」正是这个产品的核心诉求。
          const asNumber = /^#?[A-Za-z0-9_-]+$/.test(needle) ? needle.replace(/^#/, '') : ''
          const onlyRepo = watchers.length === 1 ? watchers[0].repo : ''
          return h(Section, { key: 'trace', title: '追溯一条', hint: '在右上角搜索框输入编号（GitHub 如 42，Gitee 如 IKSQJH），就能查它经历了什么：被过滤、已唤醒，还是根本没轮到' },
            asNumber
              ? h('div', { className: 'dt-row-meta' },
                h('button', {
                  className: 'dt-textlink',
                  type: 'button',
                  onClick: async () => {
                    const seq = ++traceSeqRef.current
                    setTraceResult({ loading: true, number: asNumber })
                    try {
                      const query = '/trace?number=' + encodeURIComponent(asNumber) + (onlyRepo ? '&repo=' + encodeURIComponent(onlyRepo) : '')
                      const payload = await call(query)
                      if (seq !== traceSeqRef.current) return
                      setTraceResult(payload || { text: '没有结果', traces: [] })
                    } catch (error) {
                      if (seq !== traceSeqRef.current) return
                      setTraceResult({ error: String((error && error.message) || error), number: asNumber })
                    }
                  },
                }, '追溯 #' + asNumber),
                h('span', { className: 'dt-row-meta' }, onlyRepo ? '（在 ' + onlyRepo + ' 里查）' : '（在所有守里查）'))
              : null,
            traceResult ? h(TraceTimeline, { payload: traceResult }) : null)
        })(),
        (() => {
          const counts = new Map()
          for (const w of watchers) if (w.deferReason) counts.set(w.deferReason, (counts.get(w.deferReason) || 0) + 1)
          if (counts.size === 0) return null
          return h('div', { className: 'dt-row-meta', key: 'defer-totals' },
            h('span', null, '暂停原因：'),
            [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([code, n]) => h(Pill, {
              key: code,
              active: deferFilter === code,
              onClick: () => setReasonFilter(deferFilter === code ? null : 'defer:' + code),
            }, (DEFER_LABELS[code] || code) + ' × ' + n)),
            deferFilter ? h(Pill, { key: 'clear-defer', onClick: () => setReasonFilter(null) }, '清除') : null)
        })(),
        reasonTotals.length > 0
          ? h('div', { className: 'dt-row-meta', key: 'reason-totals' },
            h('span', null, '跳过原因累计：'),
            reasonTotals.map(([reason, n]) => h(Pill, {
              key: reason,
              active: reasonFilter === reason,
              onClick: () => setReasonFilter(reasonFilter === reason ? null : reason),
            }, reason + ' × ' + n)),
            reasonFilter ? h(Pill, { key: 'clear', onClick: () => setReasonFilter(null) }, '清除筛选') : null)
          : null,
        reasonFilter
          ? h('ul', { className: 'dt-plain-list', key: 'reason-entries' },
            watchers.flatMap((w) => (w.filteredLog || [])
              .filter((entry) => entry.reason === reasonFilter)
              .map((entry) => h('li', { key: w.repo + '#' + entry.number + entry.at },
                w.repo + ' #' + entry.number + ' ' + String(entry.title || '').slice(0, 60)))))
          : null,
        watchers.length === 0
          // 判定页的空态也要给路径：说清「加一条之后这里会出现什么」，而不是一句「还没有主理鱼」
          ? h('div', { className: 'dt-empty' },
            h('p', { className: 'dt-step-title' }, '还没有可判定的记录'),
            h('p', { className: 'dt-step-detail' }, '添加一条' + BRAND + '之后，每轮判定都会记在这里：唤醒、仅观察，或跳过及其原因。'),
            h('div', { className: 'dt-actions' },
              h(Btn, { size: 'sm', variant: 'primary', onClick: () => setTab('trigger') }, '去添加第一条')))
          : (() => {
            // 跨守按时间倒序合并，再按「今天 / 昨天 / 具体日期」分组：
            // 每天独立 Section + ul，日期标题不是事件 li，列表语义不再混用。
            const merged = watchers
              .flatMap((watcher) => (watcher.history || []).slice(-6).map((entry) => ({ watcher, entry })))
              .sort((a, b) => String(b.entry.at || '').localeCompare(String(a.entry.at || '')))
            const groups = []
            merged.forEach(({ watcher, entry }, index) => {
              const day = String(entry.at || '').slice(0, 10)
              let group = groups[groups.length - 1]
              if (!group || group.day !== day) {
                group = { day, label: dayLabel(entry.at), items: [] }
                groups.push(group)
              }
              group.items.push({ watcher, entry, index })
            })
            return groups.map((group) => h(Section, {
              key: 'day-' + group.day,
              title: h('span', { className: 'dt-timeline-day' }, group.label),
            }, h('ul', { className: 'dt-list' },
              group.items.map(({ watcher, entry, index }) => eventRow(watcher, entry, index)))))
          })())

      /** 时间线分组标题：今天 / 昨天 / 具体日期（本地时区）。 */
      function dayLabel(iso) {
        const date = new Date(iso)
        if (Number.isNaN(date.getTime())) return '更早'
        const today = new Date()
        const day = (d) => d.getFullYear() + '-' + d.getMonth() + '-' + d.getDate()
        if (day(date) === day(today)) return '今天'
        const yesterday = new Date(today.getTime() - 86400000)
        if (day(date) === day(yesterday)) return '昨天'
        return (date.getMonth() + 1) + ' 月 ' + date.getDate() + ' 日'
      }

      function eventRow(watcher, entry, index) {
        const label = entry.ok ? '已唤醒' : entry.reason === 'observe' ? '仅观察（未唤醒）' : '跳过：' + (entry.reason || '未知')
        const mark = entry.ok
          ? { state: 'done', icon: 'decision', label: '已唤醒' }
          : entry.reason === 'observe'
            ? { state: 'idle', icon: 'clock', label: '仅观察' }
            : { state: 'warning', icon: 'alert', label: '跳过' }
        const reason = entry.ok ? '判定通过' : entry.reason === 'observe' ? '仅观察（未唤醒）' : '原因：' + (entry.reason || '未知')
        const numbers = (entry.numbers || []).map((n) => '#' + n).join(' ')
        return h('li', {
          key: watcher.repo + '#' + watcher.sessionId + entry.at + index,
          className: 'dt-row dt-link dt-ticket',
          'data-state': mark.state,
        },
          h('div', { className: 'dt-ticket-stage' },
            h('span', { className: 'dt-stage', 'data-state': mark.state },
              h(Icon, { name: mark.icon, size: 14 }), mark.label)),
          h('div', { className: 'dt-ticket-body' },
            // dt-event-head 仅为旧主题兼容保留；语义与布局均以 DecisionLine 的 dt-row-title 为准。
            h('div', { className: 'dt-row-title' },
              h('div', { className: 'dt-event-head' },
                h('strong', { className: 'dt-event-actor' }, watcher.repo))),
            h('div', { className: 'dt-row-quote dt-line' },
              label, ' · ', h('span', { className: 'dt-event-nums' }, numbers || '无编号')),
            h('div', { className: 'dt-row-meta dt-line' },
              h('span', { className: 'dt-line-text' }, reason),
              h('time', { className: 'dt-event-time', dateTime: entry.at, title: entry.at }, ageText(entry.at)))))
      }

      // 迷你时间线：回答"为什么这一轮没叫醒我"。先全局合并排序，再兑现标题的最近 20 条。
      const runtimeEvents = watchers
        .flatMap((watcher) => (watcher.events || []).map((event, sourceIndex) => ({ watcher, event, sourceIndex })))
        .sort((a, b) => String(b.event.at || '').localeCompare(String(a.event.at || ''))
          // 同守同时间戳仍保持原先 slice().reverse() 的后进先出；跨守同刻交给稳定排序。
          || (a.watcher === b.watcher ? b.sourceIndex - a.sourceIndex : 0))
        .slice(0, 20)
      const timeline = h(Section, {
        key: 'timeline',
        title: '运行时事件（最近 20 条）',
        // 和上面的「判定日志」区分开：那条记的是**每轮判定结果**，这条记的是**每次轮询做了什么**
        hint: '每次轮询做了什么：唤醒、攒着、限流、回收。上面那条只记判定结果。',
      },
        watchers.length === 0
          ? h(Hint, null, '还没有守。添加一条之后，这里会记录每次唤醒、攒着、限流、回收。')
          : runtimeEvents.length === 0
            ? h(Hint, null, '还没有运行时事件；完成一次轮询后，这里会记录唤醒、攒着、限流、回收。')
            : h('ol', { className: 'dt-timeline' }, runtimeEvents.map(({ watcher, event, sourceIndex }) => h('li', {
              key: ['event', watcher.sessionId, watcher.repo,
                event.number ?? (Array.isArray(event.numbers) ? event.numbers.join(',') : ''),
                event.kind, event.at, sourceIndex].map((part) => String(part ?? '')).join('|'),
              className: 'dt-event',
            },
              h('div', { className: 'dt-event-head' },
                h(Tag, { tone: event.kind === 'wake' ? 'success' : event.kind === 'wake-failed' ? 'danger' : 'outline' },
                  event.kind + (Number(event.count) > 1 ? ' ×' + event.count : '')),
                (Number(event.count) > 1 && event.since)
                  ? h('span', { className: 'dt-row-meta' }, '持续 ' + durationText(event.since, event.at))
                  : null,
                h('span', { className: 'dt-row-meta' }, '会话 ' + sessionTag(watcher.sessionId)),
                h('span', { className: 'dt-row-meta' }, ageText(event.at))),
              h('div', { className: 'dt-event-text' }, watcher.repo + ' · ' + event.text),
              h('div', { className: 'dt-event-foot' }, event.at)))))

      // 按仓库汇总：守多个仓库时先看哪个健康，再看明细（渐进披露）
      const problemCount = watchers.filter(hasProblem).length
      const repoSummary = watchers.length === 0 ? null : h(Section, { key: 'repo-summary', title: '按仓库', hint: '点仓库名只看它（再点取消）' },
        h('div', { className: 'dt-row-meta' },
          h(Pill, { active: problemsOnly, onClick: () => setProblemsOnly(!problemsOnly) }, '只看有问题的' + (problemCount > 0 ? ' (' + problemCount + ')' : '')),
          problemsOnly ? h(Pill, { onClick: () => setProblemsOnly(false) }, '全部') : null),
        body.sessionUsage
          ? h('div', { className: 'dt-row-meta' },
            // 和统计条同一套官方素材：数值强（dt-stat-value）+ 单位弱
            h(Stat, { value: body.sessionUsage.used + ' / ' + body.sessionUsage.cap, unit: '本会话近一小时唤醒' }),
            body.sessionUsage.used >= body.sessionUsage.cap
              ? h('span', { className: 'dt-error-text' },
                '已到上限：新条目会攒到下一小时' +
                (body.sessionUsage.suggest ? '；建议把「这条会话每小时最多被唤醒」调到 ' + body.sessionUsage.suggest : '；可以把该上限调大'))
              : null,
            body.sessionUsage.capped > 0
              ? h('span', { className: 'dt-error-text' }, body.sessionUsage.capped + ' 条守正因上限暂停')
              : null,
            (() => {
              // 过去几天累计（数据来自每条守的 daily，几十字节）
              const days = watchers.reduce((n, w) => n + (Number(w.daily && w.daily.days) || 0), 0)
              const total = watchers.reduce((n, w) => n + (Number(w.daily && w.daily.total) || 0), 0)
              if (total === 0) return null
              const span = Math.max(...watchers.map((w) => Number(w.daily && w.daily.days) || 0))
              return [
                h('span', { key: 'hist', className: 'dt-row-meta' }, '过去 ' + span + ' 天：共 ' + total + ' 次（日均 ' + (Math.round((total / Math.max(1, span)) * 10) / 10) + '）'),
                watchers.some((w) => w.busy)
                  ? h('span', { key: 'busy', className: 'dt-error-text' }, '今天明显比平时忙 —— 上限可能不够用')
                  : null,
              ]
            })())
          : null,
        visible.length === 0 && watchers.length > 0
          ? h(Hint, null, '按仓库汇总也为空；请用上方「清除全部筛选」恢复。')
          : null,
        h('div', { className: 'dt-rows' }, ...summarizeByRepo(visible).map((group) => h('div', { key: 'repo-' + group.repo, className: 'dt-row' },
          h('div', { className: 'dt-row-main' },
            h('div', { className: 'dt-row-title' },
              h(StateDot, { state: group.errors > 0 ? 'error' : group.silentWakes >= 2 ? 'warning' : 'done' }),
              h('button', {
                className: 'dt-textlink dt-row-name',
                type: 'button',
                onClick: () => setSearch(search === group.repo ? '' : group.repo),
                title: '点一下只看这个仓库（再点取消）',
              }, group.repo),
              ...group.badges.map((badge) => h(Tag, { key: 'rb-' + badge.text, tone: badge.tone }, badge.text))),
            h('div', { className: 'dt-row-meta' },
              h('span', null, group.count + ' 条守（启用 ' + group.enabled + '）'),
              h('span', null, group.lastWake ? '最近唤醒 ' + ageText(group.lastWake) : '还没唤醒过'),
              group.silentWakes > 0 ? h('span', { className: 'dt-error-text' }, '未回音 ' + group.silentWakes + ' 次') : null,
              group.errors > 0 ? h('span', { className: 'dt-error-text' }, group.errors + ' 条报错') : null))))))

      const trigger = h(Section, { title: '新增' + BRAND },
        h(WatcherForm, {
          mode: 'create', idPrefix: 'watcher-new', value: { ...CREATE_WATCHER_VALUES, ...form },
          snapshot: createSnapshotRef.current, defaults: DEFAULTS, disabled: !sessionId, sessionId,
          submitLabel: sessionId ? '添加到当前会话' : '先打开一个会话',
          onChange: setCreateField,
          onSave: addWatcher,
          onDiscard: () => {
            setForm({ ...createSnapshotRef.current })
            createTouchedRef.current = new Set()
            setNotice(null)
          },
        }))

      // 「仓库」行：先给条数，再看名字；超过 3 条截断成「… 等」。
      // 原先用 ", " 把三个 owner/name 粘成一行（实测 518px / 可用 541px），
      // 读者数不清有几条守，窄屏还会劈在 owner/name 中间；改用顿号后换行只发生在条目之间。
      const repoListText = (list) => list.length + ' 条：'
        + list.slice(0, 3).map((w) => w.repo).join('、')
        + (list.length > 3 ? ' 等' : '')

      // 项目层写到 <repo>/.superagent/gates.json：有隔离工作目录时它就是真实仓库路径；
      // 没有就交给服务端按默认目录来，面板把服务端回的路径原样显示出来。
      const gateRepo = (watchers.find((item) => typeof item.workdir === 'string' && item.workdir.trim()) || {}).workdir || ''

      const settingsSections = [
        h(Section, { key: 'source', title: '来源' },
          h(KeyValues, { rows: [
            ['仓库', watchers.length === 0 ? '未绑定' : repoListText(watchers)],
            // 审核 T26 M1：`gh.ok` 是**三态**（true / false / null=「这次没探测，因为不需要」）。
            // 以前当二态用，null 落进 else —— 新客户（零守）和全 Gitee 客户都被要求去跑 gh auth login。
            ['gh CLI', !body.gh
              ? '检测中…'
              : body.gh.ok === null || body.gh.ok === undefined
                ? '不需要（没有 GitHub 主理鱼）' + (body.gh.skipped ? ' · ' + String(body.gh.skipped) : '')
                : body.gh.ok
                  ? '可用 · ' + (body.gh.version || '')
                  : '不可用 —— 先跑 gh auth login（或装 gh CLI）；也可以去「凭证」填 token · ' + String(body.gh.error || '').slice(0, 80)],
            // 标签要能和左栏「凭证」分区区分开；值要写出「面板 token / gh 登录态」二选一。
            ['GitHub 凭证', credentials.token === 'none' ? '回退到已登录的 gh CLI' : '用面板 token（不再需要 gh login）'],
            ['权限范围', (() => {
              const info = body.tokenScopes
              if (!info) return '正在探测…'
              if (!info.ok) return '探测失败：' + String(info.error || '').slice(0, 60)
              if (!info.scopes.length) return '细粒度 token（无 OAuth scope）'
              return info.scopes.join(', ')
            })()],
            (body.tokenScopes && body.tokenScopes.broad && body.tokenScopes.broad.length > 0)
              ? ['提示', '这个 token 带 ' + body.tokenScopes.broad.join(', ') + ' —— 我们的功能只需要「读 issue/PR + 写评论和标签」，建议换细粒度 token 只授 Issues 读写']
              : null,
            // 只有一个隔离根时「合计」恒等于前面那个数 —— 同一个数字写两遍；只有多根时才补合计。
            ['隔离占用', (() => {
              const roots = (body.isolation && body.isolation.roots) || []
              if (roots.length === 0) return '没有开隔离执行'
              const each = roots.map((r) => r.count + ' 个隔离根 · ' + bytesText(r.bytes)).join('；')
              return roots.length > 1 ? each + '（合计 ' + bytesText(body.isolation.totalBytes) + '）' : each
            })()],
          ] })),
        h(Section, { key: 'creds', title: '凭证' },
          h(KeyValues, { rows: [['GitHub token', credentials.token === 'none' ? '未配置（回退到已登录的 gh CLI）' : '已配置'], ['SSH 私钥', credentials.sshKeyPath === 'none' ? '未配置（当前版本不使用）' : '已配置'],
            ['gh 账号', credentials.ghConfigDir === 'configured' ? '已绑定独立配置目录（评论挂智能体账号）' : '宿主当前登录的账号'],
            ['Gitee 令牌', credentials.giteeToken === 'configured' ? '已配置' : '未配置（公开仓库也能读，私有仓库需要）']] }),
          field('Token', h(Input, { type: 'password', value: token, placeholder: 'ghp_… / github_pat_…（留空不改）', onChange: (e) => { credentialsDraftVersionRef.current += 1; setToken(e.target.value) } })),
          field('Gitee 令牌', h(Input, { type: 'password', value: giteeToken, placeholder: '私人令牌（留空不改）；只盯 GitHub 可以不管' , onChange: (e) => { credentialsDraftVersionRef.current += 1; setGiteeToken(e.target.value) } })),
          field('gh 账号目录', h(Input, { value: ghDir, placeholder: '绝对路径（留空 = 用宿主账号）', onChange: (e) => { credentialsDraftVersionRef.current += 1; setGhDir(e.target.value) } })),
          // 解绑要**显式**：以前只改 token 时会顺带把已绑定的目录清掉，客户不知道评论为什么换回了宿主账号。
          credentials.ghConfigDir === 'configured'
            ? h('div', { className: 'dt-actions' }, h(Btn, { variant: 'ghost', size: 'sm', onClick: clearGhDir }, '解绑 gh 账号目录'))
            : null,
          h('div', { className: 'dt-actions' }, h(Btn, { variant: 'primary', onClick: saveCredentials, disabled: !credentialsDirty }, '保存凭证')),
          h(Hint, null, '不填也能用：插件直接复用已登录的 gh CLI。填了 gh 账号目录，插件就用那个目录里的账号发言（git push 仍用会话自己的身份，唤醒消息里会提醒会话）。')),
        h(GateSettings, { key: 'gate', repo: gateRepo, notify }),
        h(Section, { key: 'update', title: '版本与更新' },
          h(KeyValues, { rows: [
            ['当前版本', (update && update.current) || CLIENT_VERSION],
            ['npm 最新', !update ? '检测中…' : update.error
              ? h('span', { className: 'dt-error', role: 'alert', 'aria-atomic': 'true' }, '检测失败 · ' + update.error)
              : (update.latest || '未知')],
            ['状态', updateResult && updateResult.resultUnknown
              ? UPDATE_UNKNOWN_HINT
              : updateAwaitingReload
                ? '已安装，等待重载'
                : updateResult
                  ? (updateResult.needsReload ? '安装完成，需要重载才生效' : '安装完成')
                  : !update ? '…' : update.error ? '检查失败' : update.hasUpdate ? '有新版本，可以升级' : '已是最新'],
            updateResult ? ['安装副本（installedVersion）', updateResult.resultUnknown ? '未知 · 请重新检查' : updateResult.installedVersion] : null,
            updateResult ? ['运行版本（runningVersion）', updateResult.resultUnknown ? '未知 · 请重新检查' : updateResult.runningVersion] : null,
            updateResult ? ['重载要求（needsReload）', updateResult.resultUnknown ? '未知 · 请重新检查' : updateResult.needsReload ? 'true · 需要重载才生效' : 'false · 当前无需重载'] : null,
          ] }),
          update && update.hasUpdate
            ? h('div', null,
                h('code', { className: 'dt-code' }, update.command),
                h('div', { className: 'dt-actions' },
                  !updateAwaitingReload ? h(ConfirmButton, {
                    label: '安装更新',
                    confirmLabel: '确认安装',
                    title: '安装更新',
                    message: '将安装 ' + update.latest + '（当前 ' + (update.current || CLIENT_VERSION) + '）。这会修改安装副本；安装完成后需要重载宿主才生效，不会自动重启。',
                    variant: 'primary',
                    size: 'sm',
                    ButtonComponent: UpdateButton,
                    danger: false,
                    preventDismissOnBusy: true,
                    onConfirm: installUpdate,
                  }) : null,
                  h(Btn, {
                    size: 'sm',
                    // 审核 T24 W2：以前点了零反馈，writeText 的 rejection 也没人接 —— 客户不知道复制没成。
                    // 对照旁边「复制诊断信息」：await + notify 才是正例。
                    onClick: () => copyText(update.command, '升级命令已复制'),
                  }, '复制命令')))
            : null,
          h('div', { className: 'dt-actions' },
            h(Btn, { size: 'sm', variant: 'ghost', onClick: () => checkUpdate(true), disabled: checking }, checking ? '检查中…' : '重新检查')),
          h(Hint, null, '安装会修改当前安装副本，不会自动重启；安装成功后需要重载宿主才生效。复制命令保留作手动回退。'),
          h(Hint, null, '反馈问题时把下面这份诊断贴给对方：里面有版本、挂载状态和每条守的最近错误（凭证已脱敏）。'),
          h('div', { className: 'dt-actions' },
            h(Btn, {
              size: 'sm',
              variant: 'outline',
              onClick: async () => {
                try {
                  const payload = await call('/diagnose')
                  const text = String((payload && payload.text) || '')
                  await copyText(text, '诊断信息已复制（' + text.split(String.fromCharCode(10)).length + ' 行）')
                } catch (error) {
                  notify('生成诊断失败：' + String((error && error.message) || error), 'error')
                }
              },
            }, '复制诊断信息'))),
        h(Section, { key: 'diagnose', title: '分诊信号与取舍' },
          h(Hint, null, '一次只处理上限条数之内，这些信号的加权决定先后。事实默认展示，判断默认关闭。'),
          // 这张表要「竖着看是两列」：左列名字 + 两个标签成组，右列 `+N 分` 定宽贴右（数字等宽）。
          // 只挂局部类 dt-signal-*，全局 `.dt-row-title` 一行不动 —— 别的列表还在用它。
          h('div', { className: 'dt-rows' }, (body.triageSignals || []).map((sig) => h('div', { key: sig.key, className: 'dt-row' },
            h('div', { className: 'dt-row-main' },
              h('div', { className: 'dt-row-title dt-signal-title' },
                h('div', { className: 'dt-signal-lead' },
                  h('span', { className: 'dt-row-name' }, sig.label),
                  h(Tag, { tone: sig.kind === 'fact' ? 'success' : 'warning' }, sig.kind === 'fact' ? '事实' : '判断'),
                  h(Tag, { tone: sig.default ? 'outline' : 'quiet' }, sig.default ? '默认开' : '默认关')),
                h('span', { className: 'dt-row-meta dt-signal-meta' }, '+' + sig.weight + ' 分')),
              h('div', { className: 'dt-hint' }, sig.why)))))),

        h(Section, { key: 'goal', title: '会话 goal' },
          h('textarea', {
            className: 'dt-textarea', rows: 3, value: goalDraft,
            'aria-label': '会话 goal', disabled: !sessionId,
            placeholder: '这个会话要达成什么（会写进 /goal）',
            onChange: (e) => { goalDraftVersionRef.current += 1; setGoalTouched(true); setGoalDraft(e.target.value) },
          }),
          !sessionId ? h(Hint, null, '没有会话上下文，无法编辑或保存 goal；请先在会话里打开本面板。') : null,
          h('div', { className: 'dt-actions' }, h(Btn, { variant: 'primary', onClick: saveGoal, disabled: !sessionId }, '保存 goal'))),
      ]


      // 官方设置页是「左栏导航 + 右侧内容」的网格（dt-settings / dt-settings-nav / dt-settings-body）。
      // 我们之前是单列堆叠，四个分区一起铺开，深链路里翻起来很累。
      const SETTINGS_SECTIONS = [
        { key: 'source', title: '来源' },
        { key: 'creds', title: '凭证' },
        { key: 'gate', title: '门禁' },
        { key: 'update', title: '版本与更新' },
        { key: 'diagnose', title: '分诊信号' },   // 与卡片标题「分诊信号与取舍」同名：导航与标题不同名会让人以为是两块东西
        { key: 'goal', title: '会话 goal' },
      ]
      // 审核（客户实拍）：**React 会把 `key` 从 props 里剥掉** —— `node.props.key` 在真机上是
      // undefined，于是 find 永远落空、回退到第一个分区。表现就是：点左栏导航高亮会动，
      // 右边内容永远是「来源」。按**顺序**取（settingsSections 与 SETTINGS_SECTIONS 同序）。
      const settingsIndex = Math.max(0, SETTINGS_SECTIONS.findIndex((item) => item.key === settingsSection))
      const settingsBody = settingsSections[settingsIndex] || settingsSections.find(Boolean) || null
      const settings = h('div', { className: 'dt-settings' },
        h('nav', { className: 'dt-settings-nav', 'aria-label': '设置分区' },
          SETTINGS_SECTIONS.map((item) => h('button', {
            key: item.key, type: 'button', className: 'dt-settings-link',
            'aria-current': settingsSection === item.key ? 'page' : undefined,
            onClick: () => setSettingsSection(item.key),
          }, item.title))),
        h('div', { className: 'dt-settings-body' }, settingsBody))

      const checkUpdate = useCallback(async (force, preserveResult = false) => {
        if (!preserveResult) setUpdateResult(null)
        setChecking(true)
        try {
          const value = await call('/update' + (force ? '?refresh=1' : ''))
          setUpdate(value)
          return true
        } catch (error) {
          setUpdate({ current: CLIENT_VERSION, latest: null, hasUpdate: false, error: String(error && error.message || error) })
          return false
        } finally {
          setChecking(false)
        }
      }, [])

      async function installUpdate() {
        const version = update && update.latest
        if (!version) {
          const text = '安装更新失败：缺少目标版本，请先重新检查'
          setNotice({ ok: false, text })
          notify(text, 'error')
          return false
        }
        setNotice(null)
        try {
          const result = await call('/update', {
            method: 'POST',
            body: JSON.stringify({ version }),
          })
          if (!result || result.ok === false) throw new Error((result && result.error) || '服务端没有返回安装结果')
          const receipt = {
            installedVersion: String(result.installedVersion || '未知'),
            runningVersion: String(result.runningVersion || '未知'),
            needsReload: result.needsReload === true,
          }
          setUpdateResult(receipt)
          notify('安装完成：installedVersion ' + receipt.installedVersion
            + '，runningVersion ' + receipt.runningVersion
            + (receipt.needsReload ? '，需要重载才生效' : ''), 'success')
          // 安装只改副本；重新检查由 GET 完成，不触发重启，也不推断 runningVersion 已变化。
          await checkUpdate(true, true)
          return true
        } catch (error) {
          const detail = String((error && error.message) || error)
          const resultUnknown = Boolean(error && error.updateResultUnknown)
          if (resultUnknown) {
            setUpdateResult({
              installedVersion: '未知',
              runningVersion: '未知',
              needsReload: null,
              resultUnknown: true,
            })
          }
          const text = resultUnknown
            ? '安装更新' + UPDATE_UNKNOWN_HINT + '：' + detail
            : '安装更新失败：' + detail
          setNotice({ ok: false, text })
          notify(text, 'error')
          return false
        }
      }

      useEffect(() => {
        // 初始检查与手动「重新检查」共用一条错误路径：失败要显示「检查失败」，不能吞掉后永远检测中。
        checkUpdate(false)
      }, [])


      /**
       * 面板自己遇到的错误也要说人话。
       *
       * 服务端那边的 gh 错误已经由 classifyGhError 归类（并附了 hint）；
       * 但**面板拉自己接口失败**（宿主正在重启、端口变了、fetch 被拒）不会经过那套，
       * 以前界面上就是一行 "Failed to fetch" —— 客户看不懂，也不知道要做什么。
       */
      function humanizeError(text) {
        const raw = String(text == null ? '' : text)
        if (!raw) return ''
        if (/Failed to fetch|NetworkError|Load failed/i.test(raw)) {
          return '连不上宿主：DSH 可能正在重启，或插件刚被重载。稍等几秒会自动重试；一直这样就去「设置 → 诊断」复制信息。'
        }
        if (/HTTP 401|HTTP 403/i.test(raw)) return '宿主拒绝了请求（401/403）：面板接口可能需要重新挂载，重启宿主即可。'
        if (/HTTP 5\d\d/.test(raw)) return '宿主内部报错（5xx）：先把「设置 → 诊断」里的信息复制出来，再重启宿主试试。'
        if (/HTTP 404/.test(raw)) return '接口不存在（404）：多半还在跑旧版本代码，重启宿主。'
        if (/timeout|timed out|ETIMEDOUT/i.test(raw)) return '请求超时：宿主可能正忙（比如同时在建隔离 worktree），稍后自动重试。'
        return raw
      }

      /** 建议追加到仓库 AGENTS.md 的处理规则（只给文本，绝不代写文件）。 */
      const AGENTS_SNIPPET = [
        '## 主理鱼（dsh-session-superagent）处理规则',
        '',
        '- 收到「【主理鱼】<owner/name> 有 N 条新动态」时，先读完整条目再决定做不做。',
        '- 做：按【完成标准】三件事写清（改了什么 / 怎么验证 / 回写到哪），改完推分支开 PR。',
        '- 不做：也要说明理由（例如"已有 PR 在做"），然后照样 ack。',
        '- **无论做还是不做，最后都要 watch_ack 打标签**，否则下一轮会被再次唤醒。',
      ].join(String.fromCharCode(10))

      /** 按仓库汇总（与服务端 watch.js:summarizeByRepo 同语义）。 */
      function summarizeByRepo(list) {
        const groups = new Map()
        for (const watcher of Array.isArray(list) ? list : []) {
          if (!watcher || !watcher.repo) continue
          const repo = String(watcher.repo)
          const group = groups.get(repo) || { repo, count: 0, lastWake: null, silentWakes: 0, errors: 0, enabled: 0, badges: [] }
          group.count += 1
          if (watcher.enabled !== false) group.enabled += 1
          if (watcher.lastWake && (!group.lastWake || String(watcher.lastWake) > String(group.lastWake))) group.lastWake = watcher.lastWake
          group.silentWakes = Math.max(group.silentWakes, Number(watcher.silentWakes) || 0)
          if (watcher.lastError) group.errors += 1
          for (const badge of repoBadges(watcher.repoAccess) || []) {
            if (!group.badges.some((b) => b.text === badge.text)) group.badges.push(badge)
          }
          groups.set(repo, group)
        }
        const weight = (g) => (g.errors > 0 ? 0 : g.silentWakes >= 2 ? 1 : 2)
        return [...groups.values()].sort((a, b) => (weight(a) - weight(b)) || a.repo.localeCompare(b.repo))
      }

      /** 持续时长（与服务端 watch.js:durationText 同语义）。 */
      function durationText(sinceIso, untilIso) {
        const from = Date.parse(String(sinceIso || ''))
        const to = Date.parse(String(untilIso || ''))
        if (!Number.isFinite(from) || !Number.isFinite(to) || to < from) return ''
        const minutes = Math.floor((to - from) / 60000)
        if (minutes < 1) return '不到 1 分钟'
        if (minutes < 60) return minutes + ' 分钟'
        const hours = Math.floor(minutes / 60)
        const rest = minutes % 60
        return rest > 0 ? hours + ' 小时 ' + rest + ' 分' : hours + ' 小时'
      }

      /** 只读仓库的提示（与服务端 gh-format.js:accessNote 同语义；面板要独立一份）。 */
      function accessNoteForPanel(access, repo) {
        if (!access || typeof access !== 'object') return ''
        if (access.push === true) return ''
        if (access.push !== false) return ''
        return '你对 ' + String(access.repo || repo || '这个仓库') + ' 只有读权限：能看、能被叫醒，但没法回写评论或打标签 —— ' +
          '要么换有写权限的 token，要么把结论写在本地/PR 描述里。'
      }

      /** 仓库能力徽章（与服务端 watch.js:repoBadges 同语义）。 */
      function repoBadges(access) {
        if (!access || typeof access !== 'object') return []
        const badges = []
        if (access.archived === true) badges.push({ text: '已归档', tone: 'warning' })
        if (access.disabled === true) badges.push({ text: '已禁用', tone: 'danger' })
        if (access.hasIssues === false) badges.push({ text: '无 Issues', tone: 'warning' })
        if (access.fork === true && access.parent && access.hasIssues !== true) badges.push({ text: 'fork · 上游 ' + String(access.parent), tone: 'outline' })
        if (access.push === false) badges.push({ text: '只读', tone: 'warning' })
        if (access.private === true) badges.push({ text: '私有', tone: 'outline' })
        return badges
      }

      /** 仓库结构提示（与服务端 watch.js:repoStructureNote 同语义；客户端要独立一份，不能跨包 import）。 */
      function repoStructureNote(access) {
        if (!access || typeof access !== 'object') return ''
        const repo = String(access.repo || '这个仓库')
        // 顺序与服务端 gh-format.js:repoStructureNote 保持一致：归档/禁用优先
        if (access.archived === true) return repo + ' 已归档：GitHub 上归档仓库是只读的，评论和打标签都会失败 —— 守它只能看、不能回写。'
        if (access.disabled === true) return repo + ' 已被禁用：这个仓库当前不可用，主理鱼不会拿到任何条目。'
        if (access.hasIssues === false) return repo + ' 关闭了 Issues —— 主理鱼只认 issue / PR，这里不会有任何条目可接。'
        if (access.fork === true && access.parent && access.hasIssues !== true) {
          return repo + ' 是 ' + String(access.parent) + ' 的 fork。fork 默认不带 Issues，issue 大多落在上游 —— 确认你要守的是哪一个。'
        }
        return ''
      }

      /** 人话字节数：客户看的是"3.2 MB"，不是 3355443。 */
      function bytesText(n) {
        const value = Number(n) || 0
        if (value < 1024) return value + ' B'
        if (value < 1024 * 1024) return (value / 1024).toFixed(1) + ' KB'
        if (value < 1024 * 1024 * 1024) return (value / 1024 / 1024).toFixed(1) + ' MB'
        return (value / 1024 / 1024 / 1024).toFixed(2) + ' GB'
      }

      // 设置页的旧调用面保留，但 DOM 也统一走共享 Field，避免再生成嵌套 label。
      function field(label, control, hint, note) {
        return h(Field, { key: label, label, hint, note }, control)
      }

      return h('div', { className: 'dt-page' },
        h('div', { className: 'dt-page-head' },
          h('div', null,
            h('h1', { className: 'dt-page-title' }, BRAND),
            h('span', { className: 'dt-hint' },
              (body.sessionTitle ? '会话：' + body.sessionTitle + ' · ' : '')
              + (sessionId ? '盯办 + 主动办：仓库有动静就叫醒本会话' : '（在会话里打开本面板，才能把仓库绑到会话上）'))),
          h('div', { className: 'dt-toolbar' },
            h(Input, { value: search, placeholder: '搜索仓库 / 编号 / 标题', onChange: (e) => setSearch(e.target.value), 'aria-label': '搜索' }),
            h(Btn, { size: 'sm', variant: 'ghost', icon: h(Icon, { name: 'question' }), onClick: () => setGlossary(true) }, '术语'),
            h(Btn, { size: 'sm', icon: h(Icon, { name: 'refresh' }), onClick: () => load() }, '刷新'),
            // 停用态下这个按钮以前是个空操作：actBulk('pause') 对 !enabled 的守直接 continue，
            // 按钮却照常可点，点完什么也没发生 —— 客户会以为坏了。没有可暂停的守时置灰并说明原因
            // （保留按钮而不是隐藏：工具栏不会随状态跳来跳去，位置也一直在）。
            watchers.length > 0 ? h(Btn, {
              size: 'sm',
              disabled: pausable === 0,
              title: pausable === 0
                ? '没有可暂停的主理鱼：当前 ' + watchers.length + ' 条守都已停用 —— 要一起恢复请点「全部恢复」'
                : '暂停这一屏显示的 主理鱼' + (visible.length !== watchers.length ? '（已筛选：' + visible.length + '/' + watchers.length + '）' : '') + '，共 ' + pausable + ' 条可暂停',
              'aria-label': pausable === 0 ? '全部暂停（不可用：没有在轮询的主理鱼）' : '全部暂停',
              onClick: () => actBulk('pause', visible),
            }, '全部暂停') : null,
            // 与「全部暂停」同口径：没有可恢复的守时置灰并说明（actBulk 对 enabled 的守直接 continue）
            watchers.length > 0 ? h(Btn, {
              size: 'sm',
              disabled: resumable === 0,
              title: resumable === 0
                ? '没有可恢复的主理鱼：当前 ' + watchers.length + ' 条守都在轮询 —— 要一起停下请点「全部暂停」'
                : '恢复这一屏显示的已暂停主理鱼' + (visible.length !== watchers.length ? '（已筛选：' + visible.length + '/' + watchers.length + '）' : '') + '，共 ' + resumable + ' 条可恢复',
              'aria-label': resumable === 0 ? '全部恢复（不可用：没有已暂停的主理鱼）' : '全部恢复',
              onClick: () => actBulk('resume', visible),
            }, '全部恢复') : null)),
        h(SegTabs, {
          items: [
            { value: 'overview', label: '总览', id: 'sa-main-tab-overview', panelId: 'sa-main-panel-overview' },
            { value: 'judge', label: '判定', id: 'sa-main-tab-judge', panelId: 'sa-main-panel-judge' },
            { value: 'trigger', label: '触发', id: 'sa-main-tab-trigger', panelId: 'sa-main-panel-trigger' },
            { value: 'settings', label: '设置', id: 'sa-main-tab-settings', panelId: 'sa-main-panel-settings' },
          ],
          value: tab, onChange: setTab, label: '主理鱼分区',
        }),
        notice ? h('div', { className: 'dt-banner dt-danger', role: 'alert', 'aria-atomic': 'true' }, notice.text) : null,
        // 有新版本时在顶部提示一次；已装到 latest 且等待重载时完全隐藏，避免重复安装入口。
        update && update.hasUpdate && !updateAwaitingReload
          ? h('div', { className: 'dt-banner dt-banner-actions' },
              h('span', { className: 'dt-banner-label' }, '有新版本 ' + update.latest + '（当前 ' + update.current + '）'),
              h('button', { className: 'dt-btn dt-btn-sm dt-btn-outline', type: 'button', onClick: () => openSettings('update') }, '去更新'))
          : null,
        toastView,
        snap.error
          ? h('div', { className: 'dt-card dt-error-card', key: 'read-error' },
            h('div', { className: 'dt-card-head' }, h('span', { className: 'dt-card-title' }, '读不到面板数据')),
            h('p', { className: 'dt-error', role: 'alert', 'aria-atomic': 'true' }, humanizeError(snap.error)),
            h('div', { className: 'dt-actions' }, h(Btn, { size: 'sm', variant: 'primary', onClick: () => load() }, '重试')))
          : null,
        snap.loading && !snap.body ? h(Hint, null, '正在读取…') : null,
        snap.body && tab === 'overview' ? h('div', {
          id: 'sa-main-panel-overview', role: 'tabpanel', 'aria-labelledby': 'sa-main-tab-overview', className: 'dt-tabpanel',
        }, overview, repoSummary) : null,
        snap.body && tab === 'judge' ? h('div', {
          id: 'sa-main-panel-judge', role: 'tabpanel', 'aria-labelledby': 'sa-main-tab-judge', className: 'dt-tabpanel',
        }, judge, timeline) : null,
        snap.body && tab === 'trigger' ? h('div', {
          id: 'sa-main-panel-trigger', role: 'tabpanel', 'aria-labelledby': 'sa-main-tab-trigger', className: 'dt-tabpanel',
        }, trigger) : null,
        snap.body && tab === 'settings' ? h('div', {
          id: 'sa-main-panel-settings', role: 'tabpanel', 'aria-labelledby': 'sa-main-tab-settings', className: 'dt-tabpanel',
        }, settings) : null,
        glossary && glossary.type === 'watcher-draft' ? h(ConfirmDialog, {
          open: true,
          label: '放弃未保存的修改',
          title: '放弃未保存的修改？',
          confirmLabel: glossary.nextKey ? '放弃并切换' : '放弃并收起',
          message: glossary.nextKey
            ? '当前 watcher 有未保存的修改。切换到 ' + glossary.watcher.repo + ' 会丢弃这些修改；取消会继续编辑当前草稿。'
            : '当前 watcher 有未保存的修改。收起会丢弃这些修改；取消会继续编辑当前草稿。',
          onClose: dismissWatcherDraft,
          onConfirm: () => {
            watcherDraftConfirmedRef.current = true
            applyWatcherEditor(glossary.watcher, glossary.nextKey)
            return true
          },
        }) : null,
        typeof glossary === 'string' ? h(PanelModal, {
          open: true,
          title: '建议追加到 ' + String(glossary).replace('rules:', '') + ' 的 AGENTS.md',
          onClose: () => setGlossary(false),
        },
          h('p', { className: 'dt-hint' }, '问题通常不是"没叫醒"，而是"叫醒了但没动手"。这段规则会明确要求：无论做不做，最后都要 ack。'),
          h('pre', { className: 'dt-pre' }, AGENTS_SNIPPET),
          h('div', { className: 'dt-actions' },
            h(Btn, {
              size: 'sm',
              variant: 'primary',
              onClick: () => copyText(AGENTS_SNIPPET, '规则已复制，粘贴到 AGENTS.md 末尾即可'),
            }, '复制这段规则'),
            h(Btn, { size: 'sm', variant: 'ghost', onClick: () => setGlossary(false) }, '关闭'))) : null,
        h(PanelModal, { open: glossary === true, title: BRAND + '术语表', onClose: () => setGlossary(false) },
          h(KeyValues, { rows: [
            [BRAND, '一条「盯办」绑定：某个仓库 → 某个会话'],
            ['盯', '轮询 issue/PR、按水位去重、冷却期吸收自己的操作'],
            ['办', '唤醒会话后按仓库 AGENTS.md 主动处理，完成后 watch_ack 打标签'],
            ['唤醒', '只在会话空闲（idle）时注入一条【' + BRAND + '】消息'],
            ['过滤', '标签白名单/黑名单、只看已指派、跳过机器人/草稿/已关闭'],
            ['时段', 'activeHours，比如 09:00-23:00，时段外只攒不唤醒'],
            ['隔离', 'isolated：每次唤醒开一次性 git worktree，改副本不碰主工作区'],
            ['ack', '处理完给条目打「' + (ACK_DEFAULTS && ACK_DEFAULTS.github ? ACK_DEFAULTS.github : '主理鱼:已处理') + '」标签，轮询直接跳过'],
            ['goal', '会话目标（ctx.goals），和 /goal 命令同一份状态'],
            ['凭证', 'token 存在服务端 secrets.json，面板只显示是否已配置'],
            ['上限', '两种：单条守每小时最多 6 次、整条会话 12 次（面板里可调）；和 GitHub 的限流不是一回事'],
            ['没有动作', '叫醒后会话没有调用 watch_ack（既没改也没回），连续 2 次就会在「需要关注」里提示'],
            ['临时副本', '隔离执行开的一次性 git worktree：只在这里改，用完回收，不碰主工作区'],
            ['盯办', '把「某个仓库」和「某个会话」绑在一起：仓库有动静就叫醒会话'],
            ['轮询', '每隔一段时间问一次 GitHub「有没有新变化」（问不到会退避重试，不用管）'],
            ['限流', '单位时间内唤醒次数撞到上限：会话每小时 12 次、单条守每小时 6 次。撞上不会丢消息，条目会攒到下一轮'],
          ] })))
    }


    return {
      inject: ['slots'],
      apply(ctx) {
        try { noteOnce('client-version', CLIENT_VERSION) } catch {}
        try { note('apply', 'start') } catch {}
        // 样式表在这里注入一次就够了：任何槽位（面板 / 圆环 / 将来的入口）都用得上。
        // 之前只在面板组件挂载时注入，没打开过面板就直接点圆环 → 弹层没有样式（0.9.159 才补上那次调用）。
        try { ensurePortedCss() } catch {}

        // ① the 主理鱼 tab in the centre-column view ring
        ctx.effect(() => {
          try {
            return ctx.slots.inject('conversation.view', () => {
              const dispose = ctx.slots.register({
                name: 'conversation.view',
                id: 'session-superagent-desk',
                order: 30,
                label: '主理鱼',
              }, ZhuliPanel)
              note('desk-registered', 'ok')
              return dispose
            })
          } catch (error) {
            note('desk-fail', String((error && error.message) || error))
            return undefined
          }
        })

        // ② the machine-wide orbit in the composer dock row — the same row as
        //    轮/步 · token 用量 · 上下文环. One ring, one satellite per live
        //    session-level superagent; click opens the per-agent detail list.
        ctx.effect(() => {
          try {
            return ctx.slots.inject('conversation.composer.dock', () => {
              const dispose = ctx.slots.register({
                name: 'conversation.composer.dock',
                id: 'session-superagent-orbit',
                order: -10, // 最左端：activity/usage 是 0/1，上下文环在槽位之外

              }, StatusOrbit)
              note('orbit-registered', 'ok')
              return dispose
            })
          } catch (error) {
            note('orbit-fail', String((error && error.message) || error))
            return undefined
          }
        })

        note('apply', 'done')
      },
    }
  },
})
