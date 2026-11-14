/**
 * @99galaxy/dsh-plugin-market browser half.
 *
 * A DSH client bundle: the shell's module loader runs this file, which only
 * registers a factory. The factory returns a Cordis plugin (`inject` + `apply`)
 * whose `apply` registers a left-sidebar entry and its matching main view.
 *
 * Differences from the dynamic-plugin form of this UI, all forced by the bundle
 * contract:
 *   - React arrives through `require("react")`, not as an injected global.
 *   - The slot service arrives as a Cordis service (`ctx.slots`), not `ctx.get`.
 *   - There is no package-private `host.call`; the host half is reached over its
 *     three HTTP routes instead (see index.js for why).
 */
window.__ModuleLoader__.load({
  id: '@99galaxy/dsh-plugin-market',
  factory: (require) => {
    var module = { exports: {} }
    var exports = module.exports
    Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' })

    const React = require('react')

    const API_LIST = '/dsh-plugin-market/list'
    const API_INSTALL = '/dsh-plugin-market/install'
    const API_RELOAD = '/dsh-plugin-market/reload'
    const PAGE = 100
    const LIST_URL = 'https://github.com/awesome-dsh-plugin/awesome-dsh-plugin'
    const SRC_URL = 'https://awesome-dsh-plugin.com/plugins.json'

    const CSS = [
      '.dsxpm-page{height:100%;min-height:0;min-width:0;display:flex;flex-direction:column;background:var(--dsw-alias-bg-layer-1);color:var(--dsw-alias-label-primary)}',
      '.dsxpm-page-scroll{flex:1;min-height:0;overflow:auto;overscroll-behavior:contain}',
      '.dsxpm-page-content{box-sizing:border-box;width:100%;max-width:1200px;margin:0 auto;padding:28px clamp(16px,3vw,40px) 40px}',
      '.dsxpm-page-title{font-size:22px;line-height:1.4;font-weight:600;margin:0 0 20px}',
      '.dsxpm-root{display:flex;flex-direction:column;gap:10px;padding:4px 2px 20px;color:var(--dsw-alias-label-primary);font-size:13px}',
      '.dsxpm-tabs{display:flex;gap:2px;border-bottom:1px solid var(--dsw-alias-border-l1)}',
      '.dsxpm-tab{cursor:pointer;border:none;background:none;color:var(--dsw-alias-label-secondary);font-family:inherit;font-size:13px;line-height:1.4;padding:7px 14px;border-bottom:2px solid transparent;margin-bottom:-1px}',
      '.dsxpm-tab:hover{color:var(--dsw-alias-label-primary)}',
      '.dsxpm-tab-on{color:var(--dsw-alias-brand-primary);border-bottom-color:var(--dsw-alias-brand-primary);font-weight:600}',
      '.dsxpm-tab-n{color:inherit;opacity:.75;margin-left:4px}',
      '.dsxpm-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center}',
      '.dsxpm-btn{cursor:pointer;border:1px solid var(--dsw-alias-border-l2);background:var(--dsw-alias-bg-layer-2);color:var(--dsw-alias-label-primary);border-radius:6px;padding:5px 11px;font-size:12px;line-height:1.4}',
      '.dsxpm-btn:hover:not(:disabled){border-color:var(--dsw-alias-brand-primary);color:var(--dsw-alias-brand-primary)}',
      '.dsxpm-btn:disabled{opacity:.5;cursor:not-allowed}',
      '.dsxpm-btn-on{border-color:var(--dsw-alias-brand-primary);color:var(--dsw-alias-brand-primary)}',
      '.dsxpm-input{flex:1;min-width:150px;background:var(--dsw-alias-bg-layer-2);border:1px solid var(--dsw-alias-border-l1);border-radius:6px;padding:5px 10px;color:var(--dsw-alias-label-primary);font-size:12px;outline:none}',
      '.dsxpm-input:focus{border-color:var(--dsw-alias-brand-primary)}',
      '.dsxpm-select{background:var(--dsw-alias-bg-layer-2);border:1px solid var(--dsw-alias-border-l1);border-radius:6px;padding:5px 8px;color:var(--dsw-alias-label-primary);font-size:12px;outline:none}',
      '.dsxpm-meta{color:var(--dsw-alias-label-secondary);font-size:12px;line-height:1.6}',
      '.dsxpm-list{display:flex;flex-direction:column;gap:8px}',
      '.dsxpm-card{border:1px solid var(--dsw-alias-border-l1);border-radius:8px;padding:8px 12px;background:var(--dsw-alias-bg-layer-1);display:flex;gap:12px;align-items:flex-start}',
      '.dsxpm-card:hover{border-color:var(--dsw-alias-border-l2)}',
      '.dsxpm-card-on{border-color:var(--dsw-alias-state-success-primary)}',
      '.dsxpm-info{flex:1;min-width:0;display:flex;flex-direction:column;gap:4px}',
      '.dsxpm-name{font-weight:600;font-size:13px;word-break:break-all}',
      '.dsxpm-desc{color:var(--dsw-alias-label-secondary);line-height:1.5;word-break:break-word}',
      '.dsxpm-tags{display:flex;flex-wrap:wrap;gap:8px;color:var(--dsw-alias-label-secondary);font-size:11px}',
      '.dsxpm-chip{border:1px solid var(--dsw-alias-border-l1);border-radius:10px;padding:1px 7px}',
      '.dsxpm-cat{color:var(--dsw-alias-brand-primary);border-color:var(--dsw-alias-brand-primary)}',
      '.dsxpm-installed{color:var(--dsw-alias-state-success-primary);border-color:var(--dsw-alias-state-success-primary);font-weight:600}',
      '.dsxpm-update{color:var(--dsw-alias-state-warn-primary);border-color:var(--dsw-alias-state-warn-primary);font-weight:600}',
      '.dsxpm-link{color:var(--dsw-alias-brand-primary);text-decoration:none;cursor:pointer;font-size:11px}',
      '.dsxpm-log{white-space:pre-wrap;word-break:break-all;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:11px;line-height:1.5;background:var(--dsw-alias-bg-layer-2);border:1px solid var(--dsw-alias-border-l1);border-radius:6px;padding:8px;max-height:220px;overflow:auto;margin-top:6px}',
    ].join('\n')

    function insertCss () {
      if (typeof document === 'undefined') return
      const id = '@99galaxy/dsh-plugin-market/styles.css'
      if (document.querySelector('style[data-plugin-css=' + JSON.stringify(id) + ']') !== null) return
      const tag = document.createElement('style')
      tag.dataset.plugin = '@99galaxy/dsh-plugin-market'
      tag.dataset.pluginCss = id
      tag.textContent = CSS
      document.head.appendChild(tag)
    }

    const el = (type, props, children) => React.createElement.apply(null, [type, props].concat(children === undefined ? [] : children))

    function fmt (n) {
      const v = typeof n === 'number' ? n : Number(n)
      if (!isFinite(v) || v <= 0) return '0'
      if (v >= 1000000) return (v / 1000000).toFixed(1).replace(/\.0$/, '') + 'M'
      if (v >= 1000) return (v / 1000).toFixed(1).replace(/\.0$/, '') + 'k'
      return String(v)
    }

    function age (ts) {
      if (!ts) return ''
      const mins = Math.floor((Date.now() - Number(ts)) / 60000)
      if (!isFinite(mins) || mins < 0) return ''
      if (mins < 1) return '刚刚'
      if (mins < 60) return mins + ' 分钟前'
      const h = Math.floor(mins / 60)
      if (h < 24) return h + ' 小时前'
      return Math.floor(h / 24) + ' 天前'
    }

    const textOf = (v) => (v === null || v === undefined ? '' : typeof v === 'string' ? v : String(v))

    async function getList (query) {
      const params = new URLSearchParams({
        category: query.category,
        q: query.q,
        sort: query.sort,
        only: query.only,
        limit: String(PAGE),
        offset: String(query.offset),
      })
      if (query.bypass) params.set('bypass', '1')
      const res = await fetch(API_LIST + '?' + params.toString(), { headers: { accept: 'application/json' } })
      const body = await res.json().catch(() => null)
      if (body === null) return { ok: false, message: 'Host 返回了无法解析的内容（HTTP ' + res.status + '）' }
      return body
    }

    async function postInstall (spec, name, version, update) {
      const res = await fetch(API_INSTALL, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ install: spec, name, version, update: update === true }),
      })
      const body = await res.json().catch(() => null)
      if (body === null) return { ok: false, message: 'Host 返回了无法解析的内容（HTTP ' + res.status + '）' }
      return body
    }

    async function postReload () {
      const res = await fetch(API_RELOAD, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: '{}',
      })
      const body = await res.json().catch(() => null)
      if (body === null) return { ok: false, message: 'Host 返回了无法解析的内容（HTTP ' + res.status + '）' }
      return body
    }

    // The three catalog ranges are tabs inside one sidebar page.
    function MarketPage () {
      const [tab, setTab] = React.useState('all')
      const installedMode = tab === 'installed'
      const updatableMode = tab === 'updatable'
      // Every tab but the first pins the range it shows; writing `only` here is what
      // makes the query effect below fire with the new value.
      const tabOnly = installedMode ? 'installed' : updatableMode ? 'updatable' : 'all'
      const [list, setList] = React.useState([])
      const [cats, setCats] = React.useState([])
      const [loading, setLoading] = React.useState(false)
      const [more, setMore] = React.useState(false)
      const [error, setError] = React.useState('')
      const [detail, setDetail] = React.useState('')
      const [meta, setMeta] = React.useState('')
      const [srcNote, setSrcNote] = React.useState('')
      const [saveAge, setSaveAge] = React.useState('')
      const [search, setSearch] = React.useState('')
      const [appliedSearch, setAppliedSearch] = React.useState('')
      const [category, setCategory] = React.useState('')
      const [sort, setSort] = React.useState('stars')
      const [only, setOnly] = React.useState('all')
      const [zh, setZh] = React.useState(true)
      const [total, setTotal] = React.useState(0)
      const [installedTotal, setInstalledTotal] = React.useState(0)
      const [updatableTotal, setUpdatableTotal] = React.useState(0)
      const [unknownTotal, setUnknownTotal] = React.useState(0)
      const [adv, setAdv] = React.useState(false)
      const [busy, setBusy] = React.useState('')
      const [job, setJob] = React.useState(null)
      const [reloading, setReloading] = React.useState(false)
      const [reloadNote, setReloadNote] = React.useState('')

      // Latest filter values, read by reference. The dynamic-plugin form of this UI
      // had to hand-roll this because reads went through a stale render closure and
      // two in-flight requests could resolve out of order; with real React hooks a
      // ref is the idiomatic answer to the same two hazards.
      const latest = React.useRef({ category: '', q: '', sort: 'stars', only: 'all', seq: 0 })

      // Switching tabs changes the effective range. Writing `only` here is what makes
      // the query effect below fire with the new value.
      function switchTab (next) {
        if (next === tab) return
        setTab(next)
        const nextOnly = next === 'installed' ? 'installed' : next === 'updatable' ? 'updatable' : 'all'
        latest.current.only = nextOnly
        setOnly(nextOnly)
      }

      function setFilter (patch) {
        if (patch.category !== undefined) { latest.current.category = patch.category; setCategory(patch.category) }
        if (patch.q !== undefined) { latest.current.q = patch.q.trim(); setSearch(patch.q); setAppliedSearch(patch.q.trim()) }
        if (patch.sort !== undefined) { latest.current.sort = patch.sort; setSort(patch.sort) }
        if (patch.only !== undefined) { latest.current.only = patch.only; setOnly(patch.only) }
      }

      const query = React.useCallback(function (offset, append, bypass) {
        const cur = latest.current
        cur.seq += 1
        const myId = cur.seq
        const requestedOnly = cur.only
        if (append) setMore(true)
        else setLoading(true)
        setError('')
        setDetail('')
        getList({
          category: cur.category,
          q: String(cur.q).trim(),
          sort: cur.sort,
          only: requestedOnly,
          offset,
          bypass: bypass === true,
        }).then(function (res) {
          // Only the newest request may write state.
          if (myId !== latest.current.seq) return
          setLoading(false)
          setMore(false)
          if (res && res.ok === true && res.payload) {
            const p = res.payload
            const items = p.items || []
            // Functional update: keeps this callback free of a `list` dependency, so
            // it never reads a stale page.
            if (append) setList(function (prev) { return prev.concat(items) })
            else setList(items)
            setCats(p.categories || [])
            setTotal(p.matched === undefined ? items.length : p.matched)
            setInstalledTotal(p.installedTotal === undefined ? 0 : p.installedTotal)
            setUpdatableTotal(p.updatableTotal === undefined ? 0 : p.updatableTotal)
            setUnknownTotal(p.unknownTotal === undefined ? 0 : p.unknownTotal)
            setSaveAge(p.fetchedAt ? ('本地目录保存于 ' + age(p.fetchedAt) + '（超 1 小时自动重新获取）') : '')
            setMeta(requestedOnly === 'installed'
              ? ('已安装 ' + String(p.installedTotal === undefined ? 0 : p.installedTotal) + ' 个（目录共 ' + String(p.total) + ' 个）')
              : requestedOnly === 'updatable'
                ? ('可更新 ' + String(p.updatableTotal === undefined ? 0 : p.updatableTotal) +
                   ' 个（已安装 ' + String(p.installedTotal === undefined ? 0 : p.installedTotal) + ' 个，目录共 ' + String(p.total) + ' 个）')
                : ('目录 ' + String(p.total) + ' 个' + (p.catalogUpdated ? '（数据 ' + String(p.catalogUpdated) + '）' : '')))
            setSrcNote(p.note ? String(p.note) : (p.source === 'mirror' ? '官方域名不可达，已走镜像通道' : ''))
          } else {
            if (!append) setList([])
            setError(res && res.message ? String(res.message) : '获取目录失败')
            setDetail(res && res.detail ? String(res.detail) : '')
            if (!append) { setMeta(''); setSrcNote(''); setSaveAge('') }
          }
        }, function (e) {
          if (myId !== latest.current.seq) return
          setLoading(false)
          setMore(false)
          setError('请求 Host 失败：' + String((e && e.message) || e))
        })
      }, [])

      // Filters are applied on the host, so every change must re-query immediately
      // with the new values rather than wait for a later render.
      //
      // This effect also covers the MOUNT. A separate `useEffect(…, [])` alongside it
      // fired on the first render too, issuing the identical request a second time —
      // two host round trips per page open, and nothing to show for the first one.
      React.useEffect(function () { query(0, false, false) }, [category, sort, only, appliedSearch])

      function submitSearch () {
        setFilter({ q: search })
        // Changed keywords are queried by the effect; submitting the same keyword
        // still explicitly refreshes its first page.
        if (search.trim() === appliedSearch) query(0, false, false)
      }

      function install (item) {
        const spec = String(item.name || '')
        setBusy(spec)
        setJob(null)
        setError('')
        setDetail('')
        postInstall(String(item.install || ''), spec, String(item.version || ''), item.update === true).then(function (res) {
          setBusy('')
          if (res && res.ok === true) {
            setJob({ spec: spec, kind: item.update ? '更新' : '安装', seconds: res.seconds, log: res.log || '' })
            query(0, false, false)
          } else {
            setError(res && res.message ? String(res.message) : '安装失败')
            setDetail(res && res.log ? String(res.log) : '')
          }
        }, function (e) {
          setBusy('')
          setError('请求 Host 失败：' + String((e && e.message) || e))
        })
      }

      function reloadProfile () {
        if (reloading) return
        setReloading(true)
        setError('')
        setDetail('')
        setReloadNote('')
        postReload().then(function (res) {
          setReloading(false)
          if (!res || res.ok !== true) {
            setError(res && res.message ? String(res.message) : '重载 profile 失败')
            if (res && res.detail) setDetail(String(res.detail))
            return
          }
          const pending = Array.isArray(res.pending) ? res.pending.length : 0
          if (res.restartReason || pending > 0) {
            setReloadNote('profile 配置已重载，但有 ' + (pending > 0 ? String(pending) + ' 个模块' : '部分模块') + '需要重启 DSH 才能生效。')
            query(0, false, false)
            return
          }
          setReloadNote('profile 已重载，正在刷新页面…')
          setTimeout(function () {
            try { window.location.reload() } catch (e) { /* keep the note if the shell has no location */ }
          }, 800)
        }, function (e) {
          setReloading(false)
          setError('请求 Host 失败：' + String((e && e.message) || e))
        })
      }

      function resetFilters () {
        // `only` is pinned by whichever tab is open, so clearing the other filters
        // must not switch the range back to "all".
        setFilter({ q: '', category: '', sort: 'stars', only: tabOnly })
      }

      const filtersActive = search.trim() !== '' || appliedSearch !== '' || category !== '' || (tabOnly === 'all' && only !== 'all')
      const catLabels = {}
      for (const c of cats) catLabels[c.id] = c.zh || c.en || c.id

      // In-page tabs. Each count rides on its label so the tab itself says how many
      // there are, without opening it.
      const tabBar = el('div', { key: 'tabs', className: 'dsxpm-tabs', role: 'tablist' }, [
        el('button', {
          key: 't-all',
          type: 'button',
          role: 'tab',
          'aria-selected': tab === 'all' ? 'true' : 'false',
          className: 'dsxpm-tab' + (tab === 'all' ? ' dsxpm-tab-on' : ''),
          onClick: function () { switchTab('all') },
        }, ['全部插件']),
        el('button', {
          key: 't-installed',
          type: 'button',
          role: 'tab',
          'aria-selected': installedMode ? 'true' : 'false',
          className: 'dsxpm-tab' + (installedMode ? ' dsxpm-tab-on' : ''),
          onClick: function () { switchTab('installed') },
        }, ['已安装', el('span', { key: 'n', className: 'dsxpm-tab-n' }, [String(installedTotal)])]),
        el('button', {
          key: 't-updatable',
          type: 'button',
          role: 'tab',
          'aria-selected': updatableMode ? 'true' : 'false',
          className: 'dsxpm-tab' + (updatableMode ? ' dsxpm-tab-on' : ''),
          onClick: function () { switchTab('updatable') },
        }, ['可更新', el('span', { key: 'n', className: 'dsxpm-tab-n' }, [String(updatableTotal)])]),
      ])

      const bar = []
      bar.push(el('button', { key: 'r', className: 'dsxpm-btn', disabled: loading, onClick: function () { query(0, false, true) } }, [loading ? '获取中…' : '强制重新获取']))
      bar.push(el('button', {
        key: 'reload', className: 'dsxpm-btn', disabled: reloading || busy !== '',
        title: '重新组合当前 profile，让刚安装的插件生效',
        onClick: reloadProfile,
      }, [reloading ? '重载中…' : '重载 profile']))
      const catOpts = [el('option', { key: 'all', value: '' }, ['全部分类'])]
      cats.forEach(function (c) { catOpts.push(el('option', { key: c.id, value: c.id }, [String(c.zh || c.id) + '（' + String(c.count) + '）'])) })
      bar.push(el('select', { key: 'c', className: 'dsxpm-select', value: category, onChange: function (e) { setFilter({ category: e.target.value }) } }, catOpts))
      bar.push(el('select', { key: 's', className: 'dsxpm-select', value: sort, onChange: function (e) { setFilter({ sort: e.target.value }) } }, [
        el('option', { key: 'st', value: 'stars' }, ['按 Star']),
        el('option', { key: 'dl', value: 'downloads' }, ['按下载量']),
        el('option', { key: 'nm', value: 'name' }, ['按名称']),
        el('option', { key: 'ad', value: 'added' }, ['按收录时间']),
      ]))
      // The installed and updatable tabs are already scoped, so the range selector
      // would be a no-op there; showing it would only invite confusion.
      if (tabOnly === 'all') {
        bar.push(el('select', { key: 'o', className: 'dsxpm-select', value: only, onChange: function (e) { setFilter({ only: e.target.value }) } }, [
          el('option', { key: 'a', value: 'all' }, ['全部']),
          el('option', { key: 'i', value: 'installed' }, ['仅已安装']),
          el('option', { key: 'n', value: 'notinstalled' }, ['仅未安装']),
        ]))
      }
      bar.push(el('input', {
        key: 'q', className: 'dsxpm-input', placeholder: '搜索插件名或简介（回车）…', value: search,
        onChange: function (e) { setSearch(e.target.value) },
        onKeyDown: function (e) { if (e.key === 'Enter') submitSearch() },
      }))
      bar.push(el('button', { key: 'go', className: 'dsxpm-btn', disabled: loading, onClick: submitSearch }, ['搜索']))
      if (filtersActive) bar.push(el('button', { key: 'cl', className: 'dsxpm-btn', disabled: loading, onClick: resetFilters }, ['清除筛选']))
      bar.push(el('button', { key: 'z', className: 'dsxpm-btn', onClick: function () { setZh(!zh) } }, [zh ? '中文' : 'EN']))
      bar.push(el('button', { key: 'a', className: 'dsxpm-btn', onClick: function () { setAdv(!adv) } }, [adv ? '收起设置' : '设置']))

      const kids = [tabBar, el('div', { key: 'bar', className: 'dsxpm-bar' }, bar)]

      if (adv) {
        kids.push(el('div', { key: 'adv', className: 'dsxpm-bar' }, [
          el('span', { key: 'l', className: 'dsxpm-meta' }, ['目录地址：' + SRC_URL]),
          el('a', { key: 'g', className: 'dsxpm-btn', href: LIST_URL, target: '_blank', rel: 'noreferrer', style: { textDecoration: 'none' } }, ['awesome 列表 ↗']),
        ]))
      }

      kids.push(el('div', { key: 'meta', className: 'dsxpm-meta' }, [meta === '' ? '数据源：' + SRC_URL + '（保存到本地，超过 1 小时自动重新获取）' : meta]))
      // State the blind spot rather than hiding it: a plugin the catalog carries no
      // version for — or has never heard of at all — cannot be judged either way, so
      // an empty list here must not be read as "everything is current".
      if (updatableMode && unknownTotal > 0) {
        kids.push(el('div', { key: 'unk', className: 'dsxpm-meta' }, ['另有 ' + String(unknownTotal) + ' 个已安装插件无法判断（目录里没有版本信息，或没有收录）']))
      }
      if (saveAge !== '') kids.push(el('div', { key: 'age', className: 'dsxpm-meta' }, [saveAge]))
      if (srcNote !== '') kids.push(el('div', { key: 'src', className: 'dsxpm-meta', style: { color: 'var(--dsw-alias-state-warn-primary)' } }, [srcNote]))
      if (reloadNote !== '') kids.push(el('div', { key: 'reload-note', className: 'dsxpm-meta' }, [reloadNote]))
      if (error !== '') kids.push(el('div', { key: 'err', className: 'dsxpm-meta', style: { color: 'var(--dsw-alias-state-error-primary)' } }, [error]))
      if (detail !== '') kids.push(el('div', { key: 'det', className: 'dsxpm-log' }, [detail]))

      if (job !== null) {
        const box = [el('div', { key: 't', style: { fontWeight: 600 } }, [(job.kind || '安装') + '完成：' + job.spec + '（' + String(job.seconds) + ' 秒）'])]
        box.push(el('div', { key: 'n', className: 'dsxpm-meta' }, ['请点上方「重载 profile」让刚安装的插件生效；没有 HMR 时仍需重启 DSH。']))
        if (job.log) box.push(el('div', { key: 'l', className: 'dsxpm-log' }, [String(job.log)]))
        kids.push(el('div', { key: 'job', className: 'dsxpm-card' }, [el('div', { className: 'dsxpm-info' }, box)]))
      }

      if (!loading && list.length === 0) {
        const filtered = appliedSearch !== '' || category !== ''
        kids.push(el('div', { key: 'empty', className: 'dsxpm-meta' }, [installedMode
          ? (filtered ? '已安装的插件里没有匹配项。' : '还没有安装任何插件。去「全部插件」标签挑一个吧。')
          : updatableMode
            ? (filtered ? '可更新的插件里没有匹配项。' : '没有可更新的插件。')
            : (only === 'installed' ? '没有已安装的插件。' : '暂无数据或没有匹配的插件。')]))
      }

      // Cards are keyed by name AND index: plugin names are not unique in the
      // catalog (one 100-item page repeated a name four times), and a name-only key
      // makes React reuse the wrong nodes.
      const cards = list.map(function (it, idx) {
        const spec = String(it.name || '')
        const isBusy = busy === spec
        const tags = []
        if (it.installed) tags.push(el('span', { key: 'in', className: 'dsxpm-chip dsxpm-installed' }, ['已安装' + (it.installedAs && it.installedAs !== spec ? '：' + String(it.installedAs) : '')]))
        tags.push(el('span', { key: 'cat', className: 'dsxpm-chip dsxpm-cat' }, [String(catLabels[it.category] || it.category || '未分类')]))
        if (it.stars) tags.push(el('span', { key: 'st', className: 'dsxpm-chip' }, ['★ ' + fmt(it.stars)]))
        if (it.downloads) tags.push(el('span', { key: 'dl', className: 'dsxpm-chip' }, ['↓ ' + fmt(it.downloads)]))
        // One version chip, not two: when there is an update it carries both sides.
        if (it.update) {
          tags.push(el('span', { key: 'v', className: 'dsxpm-chip dsxpm-update' }, [
            '可更新 ' + String(it.version) + (it.installedVersion ? '（当前 ' + String(it.installedVersion) + '）' : ''),
          ]))
        } else if (it.version) {
          tags.push(el('span', { key: 'v', className: 'dsxpm-chip' }, ['v' + String(it.version)]))
        }
        if (it.added) tags.push(el('span', { key: 'ad', className: 'dsxpm-chip' }, ['收录 ' + String(it.added)]))
        tags.push(el('a', { key: 'lk', className: 'dsxpm-link', href: String(it.page || it.url || LIST_URL), target: '_blank', rel: 'noreferrer' }, [it.page ? '详情页 ↗' : 'GitHub ↗']))
        const desc = zh ? (it.zh || it.en || '（无简介）') : (it.en || it.zh || '(no description)')
        return el('div', { key: spec + '#' + String(idx), className: 'dsxpm-card' + (it.installed ? ' dsxpm-card-on' : '') }, [
          el('div', { className: 'dsxpm-info' }, [
            el('div', { className: 'dsxpm-name' }, [spec]),
            el('div', { className: 'dsxpm-desc' }, [String(desc)]),
            el('div', { className: 'dsxpm-tags' }, tags),
          ]),
          el('button', {
            className: 'dsxpm-btn' + (it.installed && !it.update ? '' : ' dsxpm-btn-on'),
            disabled: busy !== '' || reloading,
            onClick: function () { install(it) },
          }, [isBusy
            ? (it.update ? '更新中…' : '安装中…')
            : (it.update ? '更新' : it.installed ? '重新安装' : '安装')]),
        ])
      })
      kids.push(el('div', { key: 'list', className: 'dsxpm-list' }, cards))

      if (!loading && list.length < total) {
        kids.push(el('div', { key: 'morebar', className: 'dsxpm-bar' }, [
          el('button', { key: 'more', className: 'dsxpm-btn', disabled: more, onClick: function () { query(list.length, true, false) } }, [more ? '加载中…' : ('加载更多（还剩 ' + String(total - list.length) + ' 个）')]),
        ]))
      }

      return el('div', { className: 'dsxpm-root' }, kids)
    }

    const inject = ['slots']

    function MarketView () {
      return el('section', { className: 'dsxpm-page', 'aria-label': 'DSH 插件市场' }, [
        el('div', { className: 'dsxpm-page-scroll' }, [
          el('div', { className: 'dsxpm-page-content' }, [
            el('h1', { className: 'dsxpm-page-title', 'data-window-drag': true }, ['DSH 插件市场']),
            el(MarketPage),
          ]),
        ]),
      ])
    }

    function MarketIcon (props) {
      const size = props && typeof props.size === 'number' ? props.size : 20
      return el('svg', { width: size, height: size, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinecap: 'round', strokeLinejoin: 'round', 'aria-hidden': true }, [
        el('path', { d: 'M6 7h12l2 14H4L6 7Z' }),
        el('path', { d: 'M9 7V6a3 3 0 0 1 6 0v1' }),
      ])
    }

    function apply (ctx) {
      ctx.effect(() => insertCss(), '@99galaxy/dsh-plugin-market: styles')
      // The host selects main views by the matching sidebar entry id / view key.
      ctx.slots.inject('main', () => ctx.slots.register({
        name: 'main',
        key: 'dsh-plugin-market',
      }, MarketView))
      ctx.slots.inject('sidebar.panellist', () => ctx.slots.register({
        name: 'sidebar.panellist',
        id: 'dsh-plugin-market',
        order: 25,
        label: '插件市场',
      }, MarketIcon))
    }

    exports.apply = apply
    exports.inject = inject
    return module.exports
  },
})
