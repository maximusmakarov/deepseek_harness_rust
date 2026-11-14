/**
 * Browser half of the DeepSeek balance badge.
 *
 * One occupant of the frame-wide `shell.overlay` slot (`@deepseek-ai/dsh-client-ui-layout`
 * declares it): a draggable pill showing the remaining balance of the credential
 * the harness itself uses for model calls. The host half owns the credential and
 * the upstream request; this half only reads the JSON snapshot it serves.
 *
 * Hand-written in the client module system's lazy-CJS factory format, so no
 * build step is involved. `id` must equal the package name. React comes from the
 * browser module table via `require`.
 *
 * Every visible string lives in this plugin's own locale namespace and reaches
 * the component through the `t` seat the slot framework injects for a
 * registration that names a `locale` namespace, so the badge follows the active
 * language instead of being Chinese-only.
 *
 * The factory itself stays side-effect free: the stylesheet, the dictionaries,
 * the interval and the slot registration are all created inside `apply` and
 * owned by that fiber.
 */
window.__ModuleLoader__.load({
  id: '@ruoyang/dsh-plugin-deepseek-balance',
  factory(require) {
    const React = require('react')

    /** Snapshot route served by this package's host half. */
    const API_PATH = '/deepseek-balance/api'
    /** Auto-refresh cadence, matching the host half's own cache window. */
    const REFRESH_MS = 60000
    /** Display symbol per reported currency; anything else is shown with its code. */
    const SYMBOLS = { CNY: '¥', USD: '$' }
    /** Id of the one style node this fiber owns. */
    const STYLE_ID = 'dsh-balance-styles'
    /**
     * Locale namespace this plugin owns. It is also the namespace named on the
     * slot registration, which is what makes the framework inject `t`.
     */
    const NS = 'deepseek-balance'

    /**
     * Dictionaries for every built-in locale. Registration requires the complete
     * set, and both dictionaries carry the same keys.
     */
    const DICTS = {
      en: {
        'pill.loading': 'Balance …',
        'pill.unavailable': 'Balance unavailable',
        'pill.title': 'DeepSeek balance · click to refresh · drag to move',
        'panel.totalIn': '{currency} total',
        'panel.grantSplit': '  granted / topped up',
        'panel.balance': 'Balance',
        'panel.noData': 'no data',
        'panel.callable': 'Callable',
        'panel.yes': 'yes',
        'panel.no': 'no',
        'panel.updated': 'Updated',
        'panel.state': 'State',
        'panel.querying': 'Querying …',
        'panel.error': 'Error',
        'panel.credential': 'Credential',
        'hint': 'Click to refresh · drag to move',
        'error.fallback': 'Balance temporarily unavailable',
        'error.NO_KEY': 'No DEEPSEEK_API_KEY credential — configure it under Settings → Models',
        'error.TRANSPORT': 'The host could not reach api.deepseek.com',
        'error.BAD_JSON': 'The balance response was not valid JSON',
        'error.TIMEOUT': 'The balance query timed out',
        'error.NO_OUTPUT': 'The balance query returned nothing parseable',
        'error.HTTP_401': 'The API key was rejected',
        'error.HTTP_403': 'This API key is not permitted',
        'error.HTTP_429': 'Rate limited — try again shortly',
      },
      zh: {
        'pill.loading': '余额 …',
        'pill.unavailable': '余额不可用',
        'pill.title': 'DeepSeek 余额 · 点击刷新 · 拖动移动',
        'panel.totalIn': '{currency} 总余额',
        'panel.grantSplit': '  赠金 / 充值',
        'panel.balance': '余额',
        'panel.noData': '无数据',
        'panel.callable': '可调用',
        'panel.yes': '是',
        'panel.no': '否',
        'panel.updated': '更新于',
        'panel.state': '状态',
        'panel.querying': '查询中 …',
        'panel.error': '错误',
        'panel.credential': '凭据',
        'hint': '点击刷新 · 按住拖动可移动位置',
        'error.fallback': '余额暂不可用',
        'error.NO_KEY': '未找到 DEEPSEEK_API_KEY 凭据 —— 请在 设置 → Models 里配置',
        'error.TRANSPORT': '宿主机连不上 api.deepseek.com',
        'error.BAD_JSON': '余额响应不是合法 JSON',
        'error.TIMEOUT': '余额查询超时',
        'error.NO_OUTPUT': '余额查询没有返回可解析的结果',
        'error.HTTP_401': 'API key 被拒绝',
        'error.HTTP_403': '这个 API key 没有权限',
        'error.HTTP_429': '被限流了 —— 稍后再试',
      },
    }

    const CSS = [
      '.dsh-balance-root{position:fixed;z-index:60;pointer-events:auto;user-select:none;',
      'font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI","Microsoft YaHei",sans-serif;',
      'font-size:12px;line-height:1.45;}',
      '.dsh-balance-pill{display:flex;align-items:center;gap:6px;padding:6px 11px;border-radius:999px;',
      'background:var(--dsw-alias-bg-overlay,rgba(24,24,27,.94));',
      'color:var(--dsw-alias-label-primary,#fafafa);',
      'border:1px solid var(--dsw-alias-border-l2,rgba(255,255,255,.16));',
      'box-shadow:0 6px 20px rgba(0,0,0,.28);cursor:grab;touch-action:none;backdrop-filter:blur(8px);}',
      '.dsh-balance-pill:active{cursor:grabbing;}',
      '.dsh-balance-dot{width:7px;height:7px;border-radius:50%;flex:none;',
      'background:var(--dsw-alias-label-secondary,#a1a1aa);}',
      '.dsh-balance-tone-ok .dsh-balance-dot{background:var(--dsw-alias-state-success-primary,#22c55e);}',
      '.dsh-balance-tone-warn .dsh-balance-dot{background:var(--dsw-alias-state-warn-primary,#f59e0b);}',
      '.dsh-balance-tone-error .dsh-balance-dot{background:var(--dsw-alias-state-error-primary,#ef4444);}',
      '.dsh-balance-label{font-weight:600;font-variant-numeric:tabular-nums;letter-spacing:.01em;white-space:nowrap;}',
      '.dsh-balance-spin{opacity:.65;animation:dsh-balance-spin .9s linear infinite;}',
      '@keyframes dsh-balance-spin{to{transform:rotate(360deg)}}',
      '.dsh-balance-panel{display:none;margin-bottom:8px;padding:10px 12px;min-width:190px;border-radius:12px;',
      'background:var(--dsw-alias-bg-overlay,rgba(24,24,27,.96));',
      'color:var(--dsw-alias-label-primary,#fafafa);',
      'border:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.12));',
      'box-shadow:0 10px 30px rgba(0,0,0,.32);backdrop-filter:blur(10px);}',
      '.dsh-balance-root:hover .dsh-balance-panel{display:block;}',
      '.dsh-balance-row{display:flex;justify-content:space-between;gap:16px;padding:2px 0;}',
      '.dsh-balance-key{color:var(--dsw-alias-label-secondary,#a1a1aa);white-space:pre;}',
      '.dsh-balance-val{font-variant-numeric:tabular-nums;white-space:nowrap;}',
      '.dsh-balance-err{color:var(--dsw-alias-state-error-primary,#ef4444);max-width:230px;',
      'white-space:normal;word-break:break-word;text-align:right;}',
      '.dsh-balance-hint{margin-top:7px;padding-top:6px;font-size:11px;',
      'border-top:1px solid var(--dsw-alias-border-l1,rgba(255,255,255,.1));',
      'color:var(--dsw-alias-label-secondary,#a1a1aa);}',
    ].join('')

    /**
     * Read one snapshot from this package's host half.
     *
     * The host half answers structured failures with HTTP 200, so a rejected
     * promise here means the route itself was unreachable.
     *
     * @param force - bypass the host half's cache.
     * @returns the snapshot payload.
     */
    function requestSnapshot(force) {
      return fetch(API_PATH + (force === true ? '?force=1' : ''), { headers: { accept: 'application/json' } })
        .then((response) => response.json())
    }

    /** Render a thrown value as one short line. */
    function describe(error) {
      return String(error !== null && typeof error === 'object' && 'message' in error ? error.message : error)
    }

    /** Display symbol for a reported currency. */
    function symbolOf(currency) {
      return SYMBOLS[currency] !== undefined ? SYMBOLS[currency] : currency + ' '
    }

    /** `HH:MM:SS` for a snapshot timestamp. */
    function formatTime(ms) {
      const date = new Date(ms)
      const pad = (value) => (value < 10 ? '0' + String(value) : String(value))
      return pad(date.getHours()) + ':' + pad(date.getMinutes()) + ':' + pad(date.getSeconds())
    }

    /**
     * Stand-in used only when the slot framework injects no `t` seat.
     *
     * The framework throws rather than omitting the seat for a registration that
     * names a locale namespace, so this is a guard against a future change on a
     * harness this plugin deliberately declares no version dependency on: a
     * missing seat degrades the wording instead of blanking the badge.
     */
    function fallbackTranslate(key) {
      return DICTS.zh[key] !== undefined ? DICTS.zh[key] : key
    }

    /** Take the injected translate seat, falling back when it is absent. */
    function translatorOf(props) {
      if (props !== null && typeof props === 'object' && typeof props.t === 'function') return props.t
      return fallbackTranslate
    }

    /**
     * Wording for a failed snapshot.
     *
     * The host half already answers a machine-readable `code`; translating that
     * keeps the badge in the active language, while an unrecognised code falls
     * back to the host's own message so a novel failure is still legible. An
     * unknown key resolves to the key itself, which is how the miss is detected.
     */
    function failureText(t, payload) {
      if (payload !== null && typeof payload === 'object' && typeof payload.code === 'string' && payload.code !== '') {
        const key = 'error.' + payload.code
        const text = t(key)
        if (text !== key) return text
      }
      if (payload !== null && typeof payload === 'object' && typeof payload.message === 'string' && payload.message !== '') {
        return payload.message
      }
      return t('error.fallback')
    }

    /** One key/value line of the hover panel. */
    function row(key, value, extraClass) {
      return React.createElement(
        'div',
        { className: 'dsh-balance-row', key },
        React.createElement('span', { className: 'dsh-balance-key' }, key),
        React.createElement(
          'span',
          { className: 'dsh-balance-val' + (extraClass === undefined ? '' : ' ' + extraClass) },
          value,
        ),
      )
    }

    /**
     * Build the hover panel: one block per reported currency, then availability
     * and freshness. The failure branch names the credential so a missing key is
     * obvious without opening logs.
     */
    function panelRows(t, data, failure, phase) {
      const rows = []
      if (data !== null) {
        if (data.infos.length === 0) {
          rows.push(row(t('panel.balance'), t('panel.noData')))
        } else {
          for (const info of data.infos) {
            const symbol = symbolOf(info.currency)
            rows.push(row(t('panel.totalIn', { currency: info.currency }), symbol + info.total))
            rows.push(row(t('panel.grantSplit'), symbol + info.granted + ' / ' + symbol + info.toppedUp))
          }
        }
        rows.push(row(t('panel.callable'), data.isAvailable === true ? t('panel.yes') : t('panel.no')))
        rows.push(row(t('panel.updated'), formatTime(data.at)))
      } else if (phase === 'loading') {
        rows.push(row(t('panel.state'), t('panel.querying')))
        rows.push(row(t('panel.credential'), 'DEEPSEEK_API_KEY'))
      } else {
        rows.push(row(t('panel.error'), failure === null ? t('error.fallback') : failure, 'dsh-balance-err'))
        rows.push(row(t('panel.credential'), 'DEEPSEEK_API_KEY'))
      }
      rows.push(React.createElement('div', { className: 'dsh-balance-hint', key: 'hint' }, t('hint')))
      return rows
    }

    return {
      inject: ['slots', 'timer', 'locale'],
      apply(ctx) {
        ctx.effect(() => {
          if (document.getElementById(STYLE_ID) === null) {
            const node = document.createElement('style')
            node.id = STYLE_ID
            node.textContent = CSS
            document.head.append(node)
          }
          return () => { document.getElementById(STYLE_ID)?.remove() }
        }, 'deepseek-balance: styles')

        ctx.effect(() => ctx.locale.register(NS, DICTS), 'deepseek-balance: dictionaries')

        // Defined inside apply so the component closes over this fiber's context
        // and can own its own interval. One mount per client run keeps the
        // component identity stable across re-renders.
        function BalanceBadge(props) {
          const t = translatorOf(props)
          const [snapshot, setSnapshot] = React.useState({ phase: 'loading' })
          const [busy, setBusy] = React.useState(false)
          const [position, setPosition] = React.useState({ right: 22, bottom: 22 })
          const drag = React.useRef(null)

          React.useEffect(() => {
            let alive = true
            const pull = (force) => {
              requestSnapshot(force).then(
                (data) => { if (alive) setSnapshot({ phase: 'ready', data }) },
                (error) => { if (alive) setSnapshot({ phase: 'error', message: describe(error) }) },
              )
            }
            pull(false)
            const stop = ctx.interval(() => { pull(true) }, REFRESH_MS)
            return () => { alive = false; stop() }
          }, [])

          const refresh = () => {
            setBusy(true)
            requestSnapshot(true)
              .then(
                (data) => setSnapshot({ phase: 'ready', data }),
                (error) => setSnapshot({ phase: 'error', message: describe(error) }),
              )
              .then(() => setBusy(false))
          }

          const onPointerDown = (event) => {
            if (event.button !== 0) return
            try { event.currentTarget.setPointerCapture(event.pointerId) } catch (error) { /* still draggable */ }
            drag.current = {
              x: event.clientX,
              y: event.clientY,
              right: position.right,
              bottom: position.bottom,
            }
          }

          const onPointerMove = (event) => {
            const started = drag.current
            if (started === null) return
            let right = started.right - (event.clientX - started.x)
            let bottom = started.bottom - (event.clientY - started.y)
            right = Math.max(4, Math.min(right, window.innerWidth - 60))
            bottom = Math.max(4, Math.min(bottom, window.innerHeight - 32))
            setPosition({ right, bottom })
          }

          const onPointerUp = (event) => {
            const started = drag.current
            drag.current = null
            try { event.currentTarget.releasePointerCapture(event.pointerId) } catch (error) { /* nothing captured */ }
            if (started === null) return
            const moved = Math.abs(event.clientX - started.x) + Math.abs(event.clientY - started.y)
            if (moved < 5) refresh()
          }

          const data = snapshot.phase === 'ready' && snapshot.data !== null && snapshot.data.ok === true
            ? snapshot.data
            : null
          const failed = snapshot.phase === 'error'
            ? { message: snapshot.message }
            : (snapshot.phase === 'ready' && snapshot.data !== null && snapshot.data.ok !== true ? snapshot.data : null)
          const failure = failed === null ? null : failureText(t, failed)

          let tone = 'idle'
          let label = t('pill.loading')
          if (failure !== null) {
            tone = 'error'
            label = t('pill.unavailable')
          } else if (data !== null) {
            tone = data.isAvailable === true ? 'ok' : 'warn'
            label = symbolOf(data.currency) + data.total
          }

          return React.createElement(
            'div',
            {
              className: 'dsh-balance-root dsh-balance-tone-' + tone,
              style: { right: position.right + 'px', bottom: position.bottom + 'px' },
            },
            React.createElement('div', { className: 'dsh-balance-panel' }, panelRows(t, data, failure, snapshot.phase)),
            React.createElement(
              'div',
              {
                className: 'dsh-balance-pill',
                title: t('pill.title'),
                onPointerDown,
                onPointerMove,
                onPointerUp,
                onPointerCancel: onPointerUp,
              },
              React.createElement('span', { className: 'dsh-balance-dot' }),
              React.createElement('span', { className: 'dsh-balance-label' }, label),
              busy ? React.createElement('span', { className: 'dsh-balance-spin' }, '↻') : null,
            ),
          )
        }

        ctx.effect(() => ctx.slots.inject('shell.overlay', () => ctx.slots.register(
          // Naming the locale namespace is what makes the framework inject the
          // `t` seat (and re-render every outlet when the language changes).
          { name: 'shell.overlay', id: 'deepseek-balance', order: 200, locale: NS },
          BalanceBadge,
        )), 'deepseek-balance: overlay badge')
      },
    }
  },
})
