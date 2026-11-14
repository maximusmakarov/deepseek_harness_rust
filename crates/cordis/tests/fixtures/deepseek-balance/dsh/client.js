// Browser half of the deepseek-balance dsh plugin: a real-API balance card
// pinned to the sidebar footer (above the Settings trigger). Clicking the card
// opens a detail popup with the account balance AND the current session's usage
// (turns/steps, wall times, input/output/cache tokens from dsh's own
// sessionStats + tokenUsage projections).
//
// ALL data is real:
//   - balance       from GET /dsh-balance/deepseek (host proxy to
//                   api.deepseek.com/user/balance)
//   - today's spend from GET /dsh-usage/sessions (host aggregate over sessions
//                   created today, costed with the official pricing table)
// No mock data, no localStorage, no local balance mutation. The 充值 button
// opens the official DeepSeek top-up page (platform.deepseek.com/top_up) in a
// new window — the only real recharge path.
//
// Registers a 'sidebar.footer.action' entry (list slot, root scope). Keys stay
// server-side, the loopback dsh web server answers same-origin — no CORS, no
// key in the browser. Auto-refreshes every 60s and on demand.
//
// Hand-written in the lazy-CJS bundle protocol (window.__ModuleLoader__.load
// with a factory requiring 'react'), so no build step.
window.__ModuleLoader__.load({
  id: '@kt11/dsh-deepseek-balance',
  factory: (require) => {
    var module = { exports: {} }
    var exports = module.exports

    var React = require('react')
    var useState = React.useState
    var useEffect = React.useEffect
    var useCallback = React.useCallback
    var useRef = React.useRef
    var ReactDOM = require('react-dom')

    var REFRESH_INTERVAL_MS = 60000
    var RECHARGE_URL = 'https://platform.deepseek.com/top_up'
    var CONFIG = {
      // 余额 ≥ sufficient 充足；[danger, sufficient) 不足；< danger 危险
      statusThresholds: { sufficient: 50, danger: 10 },
    }
    var STATUS_COLOR = {
      sufficient: 'var(--dsw-alias-state-success-primary, #22c55e)',
      insufficient: 'var(--dsw-alias-state-warn-primary, #f59e0b)',
      danger: 'var(--dsw-alias-state-error-primary, #ef4444)',
    }

    // ---- injected CSS (spin + smooth hover/active feedback) ---------------
    var cssTagId = '@kt11/dsh-deepseek-balance/card.css'
    if (typeof document !== 'undefined' && document.querySelector('style[data-plugin-css=' + JSON.stringify(cssTagId) + ']') === null) {
      var cssTag = document.createElement('style')
      cssTag.dataset.plugin = '@kt11/dsh-deepseek-balance'
      cssTag.dataset.pluginCss = cssTagId
      cssTag.textContent = [
        '@keyframes dshb-spin{to{transform:rotate(360deg)}}',
        '.dshb-spin{animation:dshb-spin .8s linear infinite}',
        '.dshb-refresh:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.14));color:var(--dsw-alias-label-primary)}',
        '.dshb-refresh:active{transform:scale(.88)}',
        '.dshb-rail:hover{transform:scale(1.06)}',
        '.dshb-rail:active{transform:scale(.94)}',
        '.dshb-recharge:hover{background:rgba(110,139,255,.28)}',
        '.dshb-recharge:active{transform:scale(.97)}',
      ].join('\n')
      document.head.appendChild(cssTag)
    }

    // ---- helpers -----------------------------------------------------------
    function statusOf(balance) {
      if (typeof balance !== 'number' || !Number.isFinite(balance)) return { key: 'danger', label: '余额未知' }
      if (balance >= CONFIG.statusThresholds.sufficient) return { key: 'sufficient', label: '余额充足' }
      if (balance >= CONFIG.statusThresholds.danger) return { key: 'insufficient', label: '余额不足' }
      return { key: 'danger', label: '余额危险' }
    }
    function fmtTokens(n) {
      if (typeof n !== 'number' || !Number.isFinite(n)) return '--'
      if (n >= 1e6) return (n / 1e6).toFixed(1).replace(/\.0$/, '') + 'M'
      if (n >= 1e3) return String(Math.round(n / 1e3)) + 'K'
      return String(n)
    }
    function fmtDuration(ms) {
      if (typeof ms !== 'number' || !Number.isFinite(ms)) return '--'
      var s = Math.round(ms / 1000)
      var h = Math.floor(s / 3600)
      var m = Math.floor((s % 3600) / 60)
      var sec = s % 60
      if (h > 0) return h + 'h' + String(m).padStart(2, '0') + 'm'
      if (m > 0) return m + 'm' + String(sec).padStart(2, '0') + 's'
      return sec + 's'
    }
    function fmtAvg(ms) {
      if (typeof ms !== 'number' || !Number.isFinite(ms)) return '--'
      if (ms >= 1000) return (ms / 1000).toFixed(1) + 's'
      return String(Math.round(ms)) + 'ms'
    }
    function fmtPerSec(tok) {
      if (typeof tok !== 'number' || !Number.isFinite(tok)) return '--'
      return String(Math.round(tok)) + ' tok/s'
    }
    function fmtCny(cny) {
      if (typeof cny !== 'number' || !Number.isFinite(cny)) return '¥0.00'
      var s = String(parseFloat(cny.toFixed(4)))
      return '¥' + (s === '0' ? '0.00' : s)
    }
    function fmtYuan(n) {
      if (typeof n !== 'number' || !Number.isFinite(n)) return '¥0.00'
      return '¥' + n.toFixed(2)
    }
    function fmtShortDate(epochMs) {
      if (typeof epochMs !== 'number') return ''
      var d = new Date(epochMs)
      var pad = function (n) { return String(n).padStart(2, '0') }
      return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
    }
    function fmtClock(epochMs) {
      if (typeof epochMs !== 'number') return ''
      var d = new Date(epochMs)
      var pad = function (n) { return String(n).padStart(2, '0') }
      return pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds())
    }
    function fmtDateTime(epochMs) {
      if (typeof epochMs !== 'number') return ''
      var d = new Date(epochMs)
      var pad = function (n) { return String(n).padStart(2, '0') }
      return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds())
    }
    // Browser timezone offset in seconds-east, matching the server's `tz` query
    // param (getTimezoneOffset returns minutes-west; negating gives sec-east).
    function browserTz() {
      try { return new Date().getTimezoneOffset() * -60 } catch (error) { return 0 }
    }

    // ---- shared card styles ------------------------------------------------
    function cardWrapStyle() {
      return { position: 'relative', width: '100%', flex: '0 0 auto', minWidth: 0 }
    }
    function cardStyle(wide, statusKey) {
      if (!wide) {
        return { display: 'flex', alignItems: 'center', justifyContent: 'center', width: '100%', boxSizing: 'border-box', padding: '2px 0', cursor: 'pointer' }
      }
      // Frosted-glass surface on top of the desktop theme: the theme's layer
      // color is carried at ~58% opacity with a backdrop blur, so the card
      // reads as translucent "雾面" glass while staying token-driven.
      return {
        display: 'flex', flexDirection: 'column', alignItems: 'stretch', gap: '3px',
        width: '100%', boxSizing: 'border-box', minHeight: '44px', padding: '7px 10px',
        justifyContent: 'center', borderRadius: '10px',
        border: '1px solid var(--dsw-alias-border-l2)',
        background: 'color-mix(in srgb, var(--dsw-alias-bg-layer-2, rgba(127,127,127,.5)) 58%, transparent)',
        WebkitBackdropFilter: 'blur(12px) saturate(150%)',
        backdropFilter: 'blur(12px) saturate(150%)',
        boxShadow: 'inset 0 1px 0 rgba(255,255,255,.06)',
        fontFamily: 'inherit', color: 'var(--dsw-alias-label-primary)', cursor: 'pointer',
        transition: 'background .15s var(--ds-ease-in-out, ease), border-color .15s var(--ds-ease-in-out, ease)',
      }
    }
    var headStyle = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px' }
    var spacerStyle = { flex: 1, minWidth: 0 }
    var labelStyle = {
      margin: 0, fontSize: '12px', fontWeight: 500, color: 'var(--dsw-alias-label-secondary)',
      whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', minWidth: 0,
    }
    var refreshButtonStyle = {
      font: 'inherit', fontSize: '11px', color: 'var(--dsw-alias-label-secondary)',
      background: 'none', border: 'none', padding: '2px 4px', cursor: 'pointer', flex: 'none',
      borderRadius: '6px', transition: 'background .12s var(--ds-ease-in-out, ease), color .12s var(--ds-ease-in-out, ease), transform .12s var(--ds-ease-in-out, ease)',
    }
    var amountRowStyle = { display: 'flex', alignItems: 'baseline', gap: '4px', minWidth: 0 }
    var currencyStyle = { fontSize: '13px', fontWeight: 500, color: 'var(--dsw-alias-label-secondary)', whiteSpace: 'nowrap' }
    function amountStyle() {
      return { margin: 0, fontSize: '20px', fontWeight: 700, lineHeight: 1.2, letterSpacing: '-.01em', fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap', color: 'var(--dsw-alias-label-primary)' }
    }
    function amountToneStyle(key) {
      return (key === 'danger' || key === 'insufficient') ? { color: STATUS_COLOR[key] } : null
    }
    var todayRowStyle = {
      display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: '8px', minWidth: 0,
      borderTop: '1px dashed var(--dsw-alias-border-l1, rgba(127,127,127,.18))', paddingTop: '4px',
    }
    var todayLabelStyle = { margin: 0, fontSize: '11px', color: 'var(--dsw-alias-label-secondary)', whiteSpace: 'nowrap' }
    var todayValueStyle = {
      margin: 0, fontSize: '12px', fontWeight: 600, color: 'var(--dsw-alias-label-primary)',
      fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap',
    }
    var footRowStyle = { display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }
    var updatedStyle = {
      margin: 0, fontSize: '10px', color: 'var(--dsw-alias-label-tertiary, var(--dsw-alias-label-secondary))',
      opacity: '.85', whiteSpace: 'nowrap', fontVariantNumeric: 'tabular-nums',
    }
    var errorStyle = { margin: 0, fontSize: '11px', color: 'var(--dsw-alias-label-error)', wordBreak: 'break-all' }
    function pillStyle(key) {
      return {
        display: 'inline-flex', alignItems: 'center', gap: '5px', flex: 'none',
        padding: '2px 8px', borderRadius: '999px', fontSize: '10px', fontWeight: 600, whiteSpace: 'nowrap',
        color: STATUS_COLOR[key], background: 'color-mix(in srgb, ' + STATUS_COLOR[key] + ' 14%, transparent)',
      }
    }
    function pillDotStyle(color) {
      return { width: '6px', height: '6px', borderRadius: '50%', background: color, flex: 'none' }
    }
    var railWrapStyle = { display: 'flex', alignItems: 'center', justifyContent: 'center', width: '100%', padding: '4px 0 6px' }
    function railButtonStyle(key) {
      var bg = key === 'danger'
        ? 'linear-gradient(135deg,var(--dsw-alias-state-error-primary,#ef4444),#f97316)'
        : key === 'insufficient'
          ? 'linear-gradient(135deg,var(--dsw-alias-state-warn-primary,#f59e0b),#f97316)'
          : 'linear-gradient(135deg,#4d6bfe,#8b5cf6)'
      return {
        width: '34px', height: '34px', borderRadius: '50%', border: 'none', cursor: 'pointer', padding: 0,
        display: 'flex', alignItems: 'center', justifyContent: 'center', background: bg,
        color: '#fff', fontSize: '10px', fontWeight: 700, fontVariantNumeric: 'tabular-nums',
        boxShadow: '0 1px 4px rgba(77,107,254,.35)', fontFamily: 'inherit',
        transition: 'transform .12s var(--ds-ease-in-out, ease), box-shadow .12s var(--ds-ease-in-out, ease)',
      }
    }
    function railDotStyle(key) {
      return {
        position: 'absolute', right: '-1px', bottom: '-1px', width: '7px', height: '7px',
        borderRadius: '50%', background: STATUS_COLOR[key],
        border: '1.5px solid var(--dsw-specific-sidebar-fill, #fff)', boxSizing: 'border-box',
      }
    }
    var rechargeRowStyle = { display: 'flex', minWidth: 0, marginTop: '1px' }
    var rechargeButtonStyle = {
      width: '100%', fontFamily: 'inherit', fontSize: '11px', fontWeight: 600, letterSpacing: '.03em',
      padding: '5px 0', borderRadius: '7px', border: 'none', cursor: 'pointer',
      background: 'rgba(110, 139, 255, .16)', color: 'var(--dsw-alias-label-primary)',
      display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '5px',
      transition: 'background .12s var(--ds-ease-in-out, ease), transform .08s var(--ds-ease-in-out, ease)',
    }

    // ---- popup styles -------------------------------------------------------
    var popupStyle = function (pos) {
      return {
        position: 'fixed', left: pos.left + 'px', bottom: pos.bottom + 'px', zIndex: 100,
        width: 'min(300px, calc(100vw - 24px))', boxSizing: 'border-box',
        border: '1px solid var(--dsw-alias-border-inverted)',
        background: 'var(--dsw-specific-menu)', boxShadow: 'var(--dsw-shadow-lv3)',
        color: 'var(--dsw-alias-label-primary)', borderRadius: '12px', padding: '12px',
        fontSize: '12px', lineHeight: '20px', display: 'flex', flexDirection: 'column', gap: '10px',
        fontFamily: 'inherit',
      }
    }
    var sectionStyle = { display: 'flex', flexDirection: 'column', gap: '2px' }
    var sectionTitleStyle = { margin: 0, fontSize: '11px', fontWeight: 600, color: 'var(--dsw-alias-label-secondary)', marginBottom: '2px' }
    var rowStyle = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px' }
    var rowLabelStyle = { color: 'var(--dsw-alias-label-secondary)', whiteSpace: 'nowrap' }
    var rowValueStyle = { color: 'var(--dsw-alias-label-primary)', fontVariantNumeric: 'tabular-nums', textAlign: 'right' }
    var popupHintStyle = { margin: 0, fontSize: '10px', color: 'var(--dsw-alias-label-tertiary)' }
    var dividerStyle = { margin: '2px 0', border: 'none', borderTop: '1px solid var(--dsw-alias-border-l2)' }
    var sessionsListStyle = { display: 'flex', flexDirection: 'column', gap: '6px', maxHeight: '200px', overflowY: 'auto' }
    var sessionRowStyle = { display: 'flex', flexDirection: 'column', gap: '1px', padding: '4px 6px', borderRadius: '8px', background: 'var(--dsw-alias-bg-layer-3, transparent)' }
    var sessionRowMainStyle = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '10px' }
    var sessionRowTitleStyle = { color: 'var(--dsw-alias-label-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', minWidth: 0 }
    var sessionRowMetaStyle = { color: 'var(--dsw-alias-label-tertiary)', flex: 'none' }
    var sessionRowCostStyle = { color: 'var(--dsw-alias-label-primary)', fontWeight: 600, flex: 'none', fontVariantNumeric: 'tabular-nums' }
    var totalsRowStyle = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderTop: '1px solid var(--dsw-alias-border-l2)', paddingTop: '6px', color: 'var(--dsw-alias-label-secondary)', fontWeight: 600 }

    function Row(props) {
      return React.createElement('div', { style: rowStyle },
        React.createElement('span', { style: rowLabelStyle }, props.label),
        React.createElement('span', { style: rowValueStyle }, props.value),
      )
    }

    // ---- card fragments -----------------------------------------------------
    function BalancePill(st) {
      return React.createElement('span', {
        style: pillStyle(st.key),
        title: '阈值：余额 ≥ ¥' + CONFIG.statusThresholds.sufficient + ' 为充足；低于 ¥' + CONFIG.statusThresholds.danger + ' 为危险',
      },
        React.createElement('span', { style: pillDotStyle(STATUS_COLOR[st.key]) }),
        React.createElement('span', null, st.label),
      )
    }
    function CardHead(busy, onRefresh) {
      return React.createElement('div', { style: headStyle },
        React.createElement('p', { style: labelStyle }, 'API 余额'),
        React.createElement('button', {
          type: 'button', style: refreshButtonStyle,
          className: 'dshb-refresh' + (busy ? ' dshb-spin' : ''),
          onClick: onRefresh, title: '刷新余额',
        }, '↻'),
      )
    }
    function RailButton(text, key, busy, onClick, title) {
      return React.createElement('div', { style: railWrapStyle },
        React.createElement('span', { style: { position: 'relative', display: 'inline-block' } },
          React.createElement('button', {
            type: 'button', style: railButtonStyle(key), className: busy ? 'dshb-rail dshb-spin' : 'dshb-rail',
            onClick: onClick, title: title,
          }, text),
          React.createElement('span', { style: railDotStyle(key) }),
        ),
      )
    }

    function BalanceCard(props, sessions) {
      var wide = !!(props && props.wide)

      var snapshotRef = useState({ status: 'loading', data: null, error: null, at: 0 })
      var snapshot = snapshotRef[0]
      var setSnapshot = snapshotRef[1]
      var todayRef = useState({ status: 'loading', costCny: null, count: 0, date: '', source: 'estimate', platformTokens: null, platformRequests: null, error: null })
      var todayState = todayRef[0]
      var setToday = todayRef[1]
      var busyRef = useState(0)
      var busyCount = busyRef[0]
      var setBusyCount = busyRef[1]
      // busy is a count of in-flight balance requests, so overlapping calls
      // (the 60s timer + a manual refresh) never clear the spinner early.
      var busy = busyCount > 0
      var beginBusy = useCallback(function () { setBusyCount(function (c) { return c + 1 }) }, [])
      var endBusy = useCallback(function () { setBusyCount(function (c) { return Math.max(0, c - 1) }) }, [])

      var loadBalance = useCallback(function () {
        beginBusy()
        fetch('/dsh-balance/deepseek', { cache: 'no-store' })
          .then(function (res) { return res.json().catch(function () { return {} }) })
          .then(function (body) {
            if (!body || body.ok !== true) throw new Error((body && body.error) || '查询余额失败')
            setSnapshot({ status: 'ready', data: body, error: null, at: Date.now() })
          })
          .catch(function (err) {
            setSnapshot({ status: 'error', data: null, error: String(err && err.message ? err.message : err), at: Date.now() })
          })
          .finally(endBusy)
      }, [beginBusy, endBusy])

      var loadToday = useCallback(function () {
        fetch('/dsh-usage/sessions?tz=' + browserTz(), { cache: 'no-store' })
          .then(function (res) { return res.json().catch(function () { return {} }) })
          .then(function (body) {
            if (!body || body.ok !== true) throw new Error((body && body.error) || '查询今日消耗失败')
            var t = body.today || { costCny: 0, count: 0, date: '' }
            setToday({
              status: 'ready',
              costCny: typeof t.costCny === 'number' ? t.costCny : 0,
              count: typeof t.count === 'number' ? t.count : 0,
              date: typeof t.date === 'string' ? t.date : '',
              source: t.source === 'platform' ? 'platform' : 'estimate',
              platformTokens: typeof t.platformTokens === 'number' ? t.platformTokens : null,
              platformRequests: typeof t.platformRequests === 'number' ? t.platformRequests : null,
              error: null,
            })
          })
          .catch(function (err) {
            setToday({ status: 'error', costCny: null, count: 0, date: '', error: String(err && err.message ? err.message : err) })
          })
      }, [])

      var loadAll = useCallback(function () { loadBalance(); loadToday() }, [loadBalance, loadToday])

      useEffect(function () {
        loadAll()
        var timer = setInterval(loadAll, REFRESH_INTERVAL_MS)
        return function () { clearInterval(timer) }
      }, [loadAll])

      // --- detail popup ---
      var openRef = useState(false)
      var open = openRef[0]
      var setOpen = openRef[1]
      var posRef = useState(null)
      var popupPos = posRef[0]
      var setPopupPos = posRef[1]
      var detailsRef = useState(null)
      var details = detailsRef[0]
      var setDetails = detailsRef[1]
      var sessionsRef = useState(null)
      var sessionsData = sessionsRef[0]
      var setSessionsData = sessionsRef[1]
      var rootRef = useRef(null)
      var popupRef = useRef(null)

      var currentSessionId = useCallback(function () {
        try {
          var info = sessions && sessions.currentProvideInfo ? sessions.currentProvideInfo.getSnapshot() : null
          return info && info.sessionId ? info.sessionId : ''
        } catch (error) {
          return ''
        }
      }, [])

      var fetchDetails = useCallback(function () {
        var sessionId = currentSessionId()
        setDetails({ status: 'loading', data: null, error: null })
        setSessionsData(null)
        Promise.all([
          fetch('/dsh-usage/details?session=' + encodeURIComponent(sessionId), { cache: 'no-store' }).then(function (res) { return res.json().catch(function () { return {} }) }),
          fetch('/dsh-usage/sessions?tz=' + browserTz(), { cache: 'no-store' }).then(function (res) { return res.json().catch(function () { return {} }) }),
        ]).then(function (results) {
          var body = results[0]
          var sess = results[1]
          if (!body || body.ok !== true) throw new Error((body && body.error) || '查询用量失败')
          setDetails({ status: 'ready', data: body, error: null })
          setSessionsData(sess && sess.ok === true ? sess : null)
        }).catch(function (err) {
          setDetails({ status: 'error', data: null, error: String(err && err.message ? err.message : err) })
        })
      }, [currentSessionId])

      var positionPopup = useCallback(function () {
        var el = rootRef.current
        if (el) {
          var rect = el.getBoundingClientRect()
          setPopupPos({
            left: Math.max(8, Math.min(rect.left, window.innerWidth - 316)),
            bottom: Math.max(8, window.innerHeight - rect.top + 8),
          })
        }
      }, [])

      var toggle = useCallback(function () {
        setOpen(function (prev) {
          var next = !prev
          if (next) {
            positionPopup()
            fetchDetails()
          }
          return next
        })
      }, [fetchDetails, positionPopup])

      useEffect(function () {
        if (!open) return
        var onDown = function (event) {
          if (rootRef.current && !rootRef.current.contains(event.target) && (!popupRef.current || !popupRef.current.contains(event.target))) setOpen(false)
        }
        var onKey = function (event) {
          if (event.key === 'Escape') setOpen(false)
        }
        var onReposition = function () { positionPopup() }
        document.addEventListener('mousedown', onDown)
        document.addEventListener('keydown', onKey)
        // Keep the popup anchored to the card as the window scrolls/resizes.
        window.addEventListener('scroll', onReposition, true)
        window.addEventListener('resize', onReposition)
        return function () {
          document.removeEventListener('mousedown', onDown)
          document.removeEventListener('keydown', onKey)
          window.removeEventListener('scroll', onReposition, true)
          window.removeEventListener('resize', onReposition)
        }
      }, [open, positionPopup])

      var todayText = todayState.status === 'ready' ? fmtYuan(todayState.costCny) : (todayState.status === 'error' ? '--' : '…')
      var todayLabelText = '今日已消耗'
      var todayTitle = ''
      if (todayState.status === 'ready') {
        todayLabelText += todayState.source === 'platform' ? ' · 平台' : ' · 估算'
        todayTitle = todayState.source === 'platform'
          ? 'DeepSeek 平台今日实际花费' + (todayState.platformRequests != null ? ' · ' + todayState.platformRequests + ' 次请求' : '') + (todayState.platformTokens != null ? ' · ' + todayState.platformTokens + ' tokens' : '') + (todayState.date ? '（' + todayState.date + '）' : '')
          : '本地估算：今日有消耗的会话 ' + todayState.count + ' 个 · 按官方价估算' + (todayState.date ? '（' + todayState.date + '）' : '')
      }

      var body
      if (snapshot.status === 'ready' && snapshot.data) {
        var data = snapshot.data
        var info = (data.balance_infos && data.balance_infos[0]) || {}
        var currency = info.currency === 'CNY' ? '¥' : (info.currency || '')
        var totalNum = info.total_balance != null ? Number(info.total_balance) : NaN
        var totalText = Number.isFinite(totalNum) ? totalNum.toFixed(2) : '--'
        var available = data.is_available === true
        var st = statusOf(totalNum)
        var dot = available ? STATUS_COLOR[st.key] : '#ef4444'
        body = React.createElement('div', { style: cardWrapStyle(), ref: rootRef },
          React.createElement('div', {
            style: cardStyle(wide, st.key),
            title: 'DeepSeek API 余额 · 更新于 ' + fmtClock(data.fetchedAt) + ' · 点击查看详情',
            onClick: toggle,
          },
            wide
              ? React.createElement(React.Fragment, null,
                  CardHead(busy, function (event) { event.stopPropagation(); loadAll() }),
                  React.createElement('div', { style: amountRowStyle, title: available ? '余额可用' : '余额不可用' },
                    React.createElement('span', { style: currencyStyle }, currency),
                    React.createElement('p', { style: Object.assign({}, amountStyle(), amountToneStyle(st.key)) }, totalText),
                  ),
                  React.createElement('div', { style: todayRowStyle, title: todayTitle },
                    React.createElement('p', { style: todayLabelStyle }, todayLabelText),
                    React.createElement('p', { style: todayValueStyle }, todayText),
                  ),
                  React.createElement('div', { style: footRowStyle },
                    BalancePill(st),
                    React.createElement('span', { style: spacerStyle }),
                    React.createElement('p', { style: updatedStyle }, '更新于 ' + fmtClock(data.fetchedAt)),
                  ),
                  React.createElement('div', { style: rechargeRowStyle },
                    React.createElement('button', {
                      type: 'button', className: 'dshb-recharge', style: rechargeButtonStyle,
                      onClick: function (event) {
                        event.stopPropagation()
                        window.open(RECHARGE_URL, '_blank', 'noopener,noreferrer')
                      },
                      title: '前往 DeepSeek 官网充值',
                    }, '充值 ↗'),
                  ),
                )
              : RailButton(totalText, st.key, busy, toggle, 'API 余额：' + currency + totalText + ' · ' + st.label),
          ),
          open && popupPos ? ReactDOM.createPortal(React.createElement('div', { style: popupStyle(popupPos), ref: popupRef }, renderDetails(details, sessionsData, currency)), document.body) : null,
        )
      } else if (snapshot.status === 'error') {
        body = React.createElement('div', { style: cardWrapStyle(), ref: rootRef },
          React.createElement('div', { style: cardStyle(wide, 'danger'), title: snapshot.error, onClick: toggle },
            wide
              ? React.createElement(React.Fragment, null,
                  CardHead(busy, function (event) { event.stopPropagation(); loadAll() }),
                  React.createElement('p', { style: Object.assign({}, amountStyle(), { color: 'var(--dsw-alias-label-error)' }) }, '获取失败'),
                  React.createElement('p', { style: errorStyle }, snapshot.error || '余额查询失败'),
                  React.createElement('div', { style: footRowStyle },
                    BalancePill(statusOf(NaN)),
                    React.createElement('span', { style: spacerStyle }),
                    React.createElement('p', { style: updatedStyle }, snapshot.at > 0 ? '更新于 ' + fmtClock(snapshot.at) : ''),
                  ),
                )
              : RailButton('—', 'danger', busy, toggle, '余额查询失败'),
          ),
          open && popupPos ? ReactDOM.createPortal(React.createElement('div', { style: popupStyle(popupPos), ref: popupRef }, renderDetails(details, sessionsData, '¥')), document.body) : null,
        )
      } else {
        body = React.createElement('div', { style: cardWrapStyle(), ref: rootRef },
          React.createElement('div', { style: cardStyle(wide, 'ok'), title: '正在查询余额…', onClick: toggle },
            wide
              ? React.createElement(React.Fragment, null,
                  CardHead(true, function (event) { event.stopPropagation(); loadAll() }),
                  React.createElement('div', { style: amountRowStyle },
                    React.createElement('span', { style: currencyStyle }, '¥'),
                    React.createElement('p', { style: amountStyle() }, '…'),
                  ),
                  React.createElement('div', { style: todayRowStyle, title: todayTitle },
                    React.createElement('p', { style: todayLabelStyle }, todayLabelText),
                    React.createElement('p', { style: todayValueStyle }, todayText),
                  ),
                  React.createElement('div', { style: footRowStyle },
                    React.createElement('p', { style: updatedStyle }, '查询中…'),
                  ),
                )
              : RailButton('…', 'ok', true, toggle, '正在查询余额…'),
          ),
          open && popupPos ? ReactDOM.createPortal(React.createElement('div', { style: popupStyle(popupPos), ref: popupRef }, renderDetails(details, sessionsData, '¥')), document.body) : null,
        )
      }
      return body
    }

    // ---- popup content ------------------------------------------------------
    function renderSessions(sessionsData) {
      if (!sessionsData) {
        return React.createElement('p', { style: popupHintStyle }, '—')
      }
      var list = sessionsData.sessions || []
      var totals = sessionsData.totals || {}
      var children = []
      for (var i = 0; i < list.length; i++) {
        var s = list[i]
        var title = (s.title && s.title.trim() !== '') ? s.title : s.sessionId
        children.push(
          React.createElement('div', { key: s.sessionId, style: sessionRowStyle },
            React.createElement('div', { style: sessionRowMainStyle },
              React.createElement('span', { style: sessionRowTitleStyle, title: title }, title),
              React.createElement('span', { style: sessionRowMetaStyle }, fmtShortDate(s.createdAt)),
            ),
            React.createElement('div', { style: sessionRowMainStyle },
              React.createElement('span', { style: sessionRowMetaStyle }, '输入 ' + fmtTokens(s.billedInput) + ' · 输出 ' + fmtTokens(s.output)),
              React.createElement('span', { style: sessionRowCostStyle }, fmtCny(s.costCny)),
            ),
          ),
        )
      }
      if (children.length === 0) {
        return React.createElement('p', { style: popupHintStyle }, '暂无有消耗的会话')
      }
      children.push(
        React.createElement('div', { key: 'totals', style: totalsRowStyle },
          React.createElement('span', null, '合计 ' + list.length + ' 个会话'),
          React.createElement('span', { style: sessionRowCostStyle }, fmtCny(totals.costCny)),
        ),
      )
      return React.createElement('div', { style: sessionsListStyle }, children)
    }

    function renderDetails(details, sessionsData, currency) {
      if (!details || details.status === 'loading') {
        return React.createElement(React.Fragment, null,
          React.createElement('p', { style: sectionTitleStyle }, '正在查询详情…'),
          React.createElement('p', { style: popupHintStyle }, '余额与用量'),
        )
      }
      if (details.status === 'error') {
        return React.createElement(React.Fragment, null,
          React.createElement('p', { style: sectionTitleStyle }, '查询失败'),
          React.createElement('p', { style: errorStyle }, details.error),
        )
      }
      var d = details.data
      var bal = (d.balance && d.balance.balance_infos && d.balance.balance_infos[0]) || {}
      var cur = bal.currency === 'CNY' ? '¥' : (bal.currency || '')
      var stats = d.sessionStats || {}
      var usage = d.tokenUsage || {}
      var billedInput = (typeof usage.uncachedInputTokens === 'number' ? usage.uncachedInputTokens : 0) +
        (typeof usage.cacheReadTokens === 'number' ? usage.cacheReadTokens : 0) +
        (typeof usage.cacheWriteTokens === 'number' ? usage.cacheWriteTokens : 0)
      var cacheHit = billedInput > 0 && typeof usage.cacheReadTokens === 'number'
        ? Math.round(usage.cacheReadTokens / billedInput * 100) + '%'
        : '--'
      var tokPerSec = stats.decodeMs > 0 && typeof stats.decodeTokens === 'number'
        ? fmtPerSec(stats.decodeTokens / (stats.decodeMs / 1000))
        : '--'
      var ttft = stats.ttftSteps > 0 ? fmtAvg(stats.ttftMs / stats.ttftSteps) : '--'
      var avail = d.balance && d.balance.is_available === true
      return React.createElement(React.Fragment, null,
        React.createElement('div', { style: sectionStyle },
          React.createElement('p', { style: sectionTitleStyle }, 'DeepSeek 余额'),
          React.createElement(Row, { label: '总余额', value: cur + (bal.total_balance != null ? bal.total_balance : '--') }),
          React.createElement(Row, { label: '充值余额', value: cur + (bal.topped_up_balance != null ? bal.topped_up_balance : '--') }),
          React.createElement(Row, { label: '赠送余额', value: cur + (bal.granted_balance != null ? bal.granted_balance : '--') }),
          React.createElement(Row, { label: '状态', value: avail ? '可用' : '不可用' }),
        ),
        React.createElement('hr', { style: dividerStyle }),
        React.createElement('div', { style: sectionStyle },
          React.createElement('p', { style: sectionTitleStyle }, '本会话用量'),
          React.createElement(Row, { label: '轮次 · 步数', value: (stats.turns ?? 0) + ' · ' + (stats.steps ?? 0) }),
          React.createElement(Row, { label: 'LLM 耗时', value: fmtDuration(stats.llmMs) }),
          React.createElement(Row, { label: '工具耗时', value: fmtDuration(stats.toolMs) }),
          React.createElement(Row, { label: '首 token 平均', value: ttft }),
          React.createElement(Row, { label: '生成速度', value: tokPerSec }),
        ),
        React.createElement('hr', { style: dividerStyle }),
        React.createElement('div', { style: sectionStyle },
          React.createElement('p', { style: sectionTitleStyle }, 'Token 消耗'),
          React.createElement(Row, { label: '输入（合计）', value: fmtTokens(billedInput) }),
          React.createElement(Row, { label: '  未缓存', value: fmtTokens(usage.uncachedInputTokens) }),
          React.createElement(Row, { label: '  缓存读取', value: fmtTokens(usage.cacheReadTokens) }),
          React.createElement(Row, { label: '  缓存写入', value: fmtTokens(usage.cacheWriteTokens) }),
          React.createElement(Row, { label: '输出', value: fmtTokens(usage.outputTokens) }),
          React.createElement(Row, { label: '缓存命中率', value: cacheHit }),
          React.createElement(Row, { label: '预计费用', value: fmtCny(d.cost && typeof d.cost.cny === 'number' ? d.cost.cny : 0) }),
        ),

        React.createElement('hr', { style: dividerStyle }),
        React.createElement('div', { style: sectionStyle },
          React.createElement('p', { style: sectionTitleStyle }, '历史会话用量'),
          renderSessions(sessionsData),
        ),
        React.createElement('p', { style: popupHintStyle }, '更新于 ' + fmtDateTime(d.fetchedAt) + ' · 点击外部关闭'),
      )
    }

    // The balance card renders directly in the footer.action list slot (order 0),
    // right above the session-manager card (order 1); the shell places
    // sidebar.settings after the footer actions, so the two cards sit together
    // at the bottom with Settings at the very bottom.
    function apply(ctx) {
      var sessions = null
      try {
        sessions = ctx.get('sessions')
      } catch (error) {
        sessions = null
      }
      ctx.slots.inject('sidebar.footer.action', function () {
        return ctx.slots.register({
          name: 'sidebar.footer.action',
          id: 'deepseek-balance',
          order: 1,
        }, function BalanceCardEntry(props) {
          return BalanceCard(props, sessions)
        })
      })
    }

    exports.apply = apply
    // 'sessions' is intentionally optional: the card renders without it, and
    // apply resolves it defensively via ctx.get('sessions') so the details
    // popup degrades gracefully to "no current session" instead of the whole
    // card failing to mount when the service is absent.
    exports.inject = ['slots']
    return module.exports
  },
})
