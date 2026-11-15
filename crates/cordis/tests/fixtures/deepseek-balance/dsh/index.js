// DeepSeek Harness (dsh) web plugin: a DeepSeek API balance card.
//
// Host half: registers two routes on the dsh web server.
//   GET /dsh-balance/deepseek        - DeepSeek account balance
//   GET /dsh-usage/details?session=  - balance + current session usage
//     (sessionStats + tokenUsage projections from dsh's own session services)
// The API key never leaves this process: it is read from
// ~/.dsh/.credentials.yaml only to build the Authorization header of the
// outbound call to https://api.deepseek.com/user/balance. Zero runtime
// dependencies (node builtins + global fetch only).
//
// Loaded via the cordis.patch.yml row '@kt11/dsh-deepseek-balance' (see the
// package.json dsh.bundle manifest); the browser half is dsh/client.js.

import { readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'

export const name = 'deepseek-balance'
export const inject = []

const BALANCE_URL = 'https://api.deepseek.com/user/balance'
const REQUEST_TIMEOUT_MS = 15_000

// DeepSeek official pricing (api-docs.deepseek.com/quick_start/pricing), CNY
// per 1M tokens, standard tier. Prices differ per model: this harness ships
// deepseek-v4-flash and deepseek-v4-pro. Cache writes are billed at the
// cache-hit tier; DeepSeek's wire usage has no separate cache-write bucket and
// the dsh adapter never reports one, so that rate is only a safety net.
const PRICING = {
  flash: { uncachedInputPerM: 1, cacheReadPerM: 0.02, cacheWritePerM: 0.02, outputPerM: 2 },
  pro: { uncachedInputPerM: 3, cacheReadPerM: 0.025, cacheWritePerM: 0.025, outputPerM: 6 },
}

function pricingOf(model) {
  if (typeof model === 'string' && /pro/i.test(model)) return PRICING.pro
  return PRICING.flash
}

function estimateCost(usage, model) {
  const u = usage ?? {}
  const num = (v) => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? v : 0)
  const uncached = num(u.uncachedInputTokens)
  const cacheRead = num(u.cacheReadTokens)
  const cacheWrite = num(u.cacheWriteTokens)
  const output = num(u.outputTokens)
  const p = pricingOf(model)
  const cny =
    (uncached / 1e6) * p.uncachedInputPerM +
    (cacheRead / 1e6) * p.cacheReadPerM +
    (cacheWrite / 1e6) * p.cacheWritePerM +
    (output / 1e6) * p.outputPerM
  return { cny, uncached, cacheRead, cacheWrite, output }
}

// Model in effect at the end of a session log: the last `request/header` (or
// `request/context`) record wins. request/header is appended on the first
// request and whenever the config changes, so scanning the whole log gives the
// model that priced each later step.
function lastModelOf(log) {
  if (!Array.isArray(log)) return ''
  let model = ''
  for (let i = 0; i < log.length; i++) {
    const ev = log[i]
    if (!ev || typeof ev.data !== 'object' || ev.data === null) continue
    if (ev.type === 'request/header' && typeof ev.data.header?.config?.model === 'string') model = ev.data.header.config.model
    else if (ev.type === 'request/context' && typeof ev.data.model === 'string') model = ev.data.model
  }
  return model
}

// Fold the token-meter semantics over only the log events that occurred inside
// [startMs, endMs] (one local calendar day). The tokenUsage projection is
// lifetime-only with no timestamps, so this re-walks the log to attribute
// usage to the day it actually happened: a session created yesterday but used
// today now counts, and a session's earlier usage never leaks into today.
// Replace-per-(turn,step) mirrors the projection, so a usage chunk and the
// final assistant/message sample of the same step never double count — the
// last in-window sample for a step wins. Each step is priced with the model in
// effect at that point of the log.
function foldUsageInWindow(log, startMs, endMs) {
  const steps = new Map() // "turn:step" -> { uncached, cacheRead, cacheWrite, output, model }
  let currentModel = ''
  if (Array.isArray(log)) {
    for (let i = 0; i < log.length; i++) {
      const ev = log[i]
      if (!ev || typeof ev.time !== 'number' || typeof ev.data !== 'object' || ev.data === null) continue
      if (ev.type === 'request/header' && typeof ev.data.header?.config?.model === 'string') {
        currentModel = ev.data.header.config.model
        continue
      }
      if (ev.type === 'request/context' && typeof ev.data.model === 'string') {
        currentModel = ev.data.model
        continue
      }
      if (ev.time < startMs || ev.time > endMs) continue
      let turn
      let step
      let usage
      if (ev.type === 'assistant/chunk' && ev.data.chunk?.type === 'usage') {
        turn = ev.data.turn
        step = ev.data.step
        usage = ev.data.chunk.usage
      } else if (ev.type === 'assistant/message' && ev.data.usage !== undefined) {
        turn = ev.data.turn
        step = ev.data.step
        usage = ev.data.usage
      } else {
        continue
      }
      const num = (v) => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? v : 0)
      steps.set(turn + ':' + step, {
        uncached: num(usage?.inputTokens),
        cacheRead: num(usage?.cacheReadTokens),
        cacheWrite: num(usage?.cacheWriteTokens),
        output: num(usage?.outputTokens),
        model: currentModel,
      })
    }
  }
  const totals = { uncachedInputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0, outputTokens: 0 }
  let costCny = 0
  for (const entry of steps.values()) {
    totals.uncachedInputTokens += entry.uncached
    totals.cacheReadTokens += entry.cacheRead
    totals.cacheWriteTokens += entry.cacheWrite
    totals.outputTokens += entry.output
    costCny += estimateCost(
      {
        uncachedInputTokens: entry.uncached,
        cacheReadTokens: entry.cacheRead,
        cacheWriteTokens: entry.cacheWrite,
        outputTokens: entry.output,
      },
      entry.model,
    ).cny
  }
  return { totals, costCny, steps: steps.size }
}

// Local calendar day key (YYYY-MM-DD) for one epoch-ms timestamp.
function localDayKey(epochMs) {
  const d = new Date(epochMs)
  const pad = (n) => (n < 10 ? '0' : '') + n
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

// One pass over a session log to pick up BOTH the effective model (same rules
// as lastModelOf) and the latest title — avoids walking the same log twice for
// a single session row. A single event has exactly one type, so the if/else-if
// chain cannot drop a model change when a title arrives (they are distinct
// event types).
function scanLogMeta(log) {
  let model = ''
  let title = ''
  if (Array.isArray(log)) {
    for (let i = 0; i < log.length; i++) {
      const ev = log[i]
      if (!ev || typeof ev.data !== 'object' || ev.data === null) continue
      if (ev.type === 'request/header' && typeof ev.data.header?.config?.model === 'string') {
        model = ev.data.header.config.model
      } else if (ev.type === 'request/context' && typeof ev.data.model === 'string') {
        model = ev.data.model
      } else if (ev.type === 'session/title' && typeof ev.data.title === 'string' && ev.data.title.trim() !== '') {
        title = ev.data.title.trim()
      }
    }
  }
  return { model, title }
}

// A structured "not configured" error so the client can render a clear
// guidance state instead of a generic failure message.
function notConfiguredError() {
  const error = new Error('DEEPSEEK_API_KEY 未配置')
  error.code = 'not-configured'
  error.hint = '在 DSH Models 设置页配置 DEEPSEEK_API_KEY，或设置环境变量 DEEPSEEK_API_KEY'
  return error
}

// Last-resort read of the API key from ~/.dsh/.credentials.yaml. Kept only as
// a fallback: DSH's own credential seam (ctx.credentials.resolve) and the
// ambient/process env are preferred, because the launch environment can carry
// the key (or the credentials service can own the storage) without this file.
function readDeepseekApiKeyFromYaml() {
  const credsPath = join(homedir(), '.dsh', '.credentials.yaml')
  let text
  try {
    text = readFileSync(credsPath, 'utf8')
  } catch {
    return ''
  }
  const match = text.match(/^DEEPSEEK_API_KEY:\s*["']?([^"'\r\n]+)/m)
  return match ? match[1].trim() : ''
}

// Resolve the DeepSeek API key the way the harness does, most-authoritative
// first: DSH's credentials service, then the process environment, then the
// yaml fallback. Returns '' when every source is absent.
async function resolveApiKey(ctx) {
  try {
    const credentials = typeof ctx?.get === 'function' ? ctx.get('credentials') : undefined
    if (credentials !== undefined && typeof credentials.resolve === 'function') {
      const hit = await credentials.resolve('DEEPSEEK_API_KEY')
      if (hit !== undefined && hit !== null && typeof hit.value === 'string' && hit.value.trim() !== '') {
        return hit.value.trim()
      }
    }
  } catch (error) {
    // credentials service unavailable or rejected; fall through to env / yaml.
  }
  const envApiKey = process.env.DEEPSEEK_API_KEY
  if (typeof envApiKey === 'string' && envApiKey.trim() !== '') return envApiKey.trim()
  return readDeepseekApiKeyFromYaml()
}

async function queryDeepseekBalance(ctx) {
  const apiKey = await resolveApiKey(ctx)
  if (!apiKey) {
    throw notConfiguredError()
  }
  let res
  try {
    res = await fetch(BALANCE_URL, {
      headers: { Authorization: `Bearer ${apiKey}` },
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    })
  } catch (error) {
    throw new Error(`DeepSeek balance request failed: ${error?.message ?? error}`)
  }
  if (!res.ok) {
    throw new Error(`DeepSeek balance API responded ${res.status}`)
  }
  return res.json()
}

// ---------------------------------------------------------------------------
// Real platform usage ("今日已消耗" from DeepSeek's own billing).
//
// The API key can only read /user/balance; daily usage lives on the private
// platform dashboard endpoints, which require the browser session token
// (localStorage `userToken` of platform.deepseek.com). Without it the card
// falls back to the local token×price estimate. The token never leaves this
// process and is only used to build the Authorization header of the outbound
// GETs to platform.deepseek.com.
//
// To match the platform.deepseek.com usage page exactly, we replay the page's
// own queries: GET /api/v0/usage/by_api_key/cost and .../amount with an
// explicit time window in the SELECTED timezone (the browser's offset, passed
// by the client as `tz`). The page totals only the CURRENT (valid) API key —
// revoked keys are hidden — so series are filtered to the configured
// DEEPSEEK_API_KEY via its masked sensitive_id (7-char prefix + 4-char
// suffix), falling back to valid series, then to everything.
// ---------------------------------------------------------------------------
const PLATFORM_USAGE_COST_URL = 'https://platform.deepseek.com/api/v0/usage/by_api_key/cost'
const PLATFORM_USAGE_AMOUNT_URL = 'https://platform.deepseek.com/api/v0/usage/by_api_key/amount'
const PLATFORM_TOKEN_REF = 'DEEPSEEK_PLATFORM_TOKEN'
const PLATFORM_CACHE_TTL_MS = 60_000
const DEFAULT_TZ_SEC = 8 * 3600 // Beijing; the client passes the browser offset

let platformCache = { key: '', at: 0, value: null }

// Per-session memo of "today's spend" (costCny + steps) keyed by session id.
// The fold window's day-start is stable within a local day and its log is
// append-only, so a cached entry stays valid for a session as long as its
// local day key is unchanged AND its log length has not grown. Reading the
// memo avoids re-walking every unchanged session's log on each 60s poll; when
// it is stale (new events or a day roll), we recompute the fold and overwrite.
// Pruned to live sessions on every request so it never grows without bound.
const todayWindowCache = new Map()

// DeepSeek stores its platform session token as `{"value":"<token>","__version":"0"}`
// in localStorage; extract the inner `value` so either the raw string or the
// wrapped JSON works in DEEPSEEK_PLATFORM_TOKEN.
function normalizePlatformToken(raw) {
  if (typeof raw !== 'string') return ''
  const value = raw.trim()
  if (!value) return ''
  if (value.startsWith('{')) {
    try {
      const obj = JSON.parse(value)
      if (obj && typeof obj.value === 'string') return obj.value.trim()
    } catch {
      // not JSON — fall through and use the raw string
    }
  }
  return value
}

function readPlatformToken() {
  if (process.env[PLATFORM_TOKEN_REF]) return normalizePlatformToken(process.env[PLATFORM_TOKEN_REF])
  try {
    const raw = readFileSync(join(homedir(), '.dsh', '.credentials.yaml'), 'utf8')
    const m = new RegExp(`^${PLATFORM_TOKEN_REF}:\\s*["']?([^"'\r\n]+)`, 'm').exec(raw)
    return normalizePlatformToken(m ? m[1].trim() : '')
  } catch {
    return ''
  }
}

// Start-of-day epoch-seconds for "today" in a timezone offset (seconds east).
function startOfDaySec(tzSec) {
  const nowSec = Math.floor(Date.now() / 1000)
  const offset = ((nowSec + tzSec) % 86400 + 86400) % 86400
  return nowSec - offset
}

// "YYYY-MM-DD" (UTC fields) for an epoch-second rendered in a timezone offset.
function dateKeyInTz(sec, tzSec) {
  const d = new Date((sec + tzSec) * 1000)
  const pad = (n) => (n < 10 ? '0' : '') + n
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}`
}

// Masked sensitive_id is "sk-XXXXX****YYYY" (8-char prefix, 4-char suffix).
function keyMaskOf(apiKey) {
  if (typeof apiKey !== 'string' || apiKey.length < 13) return null
  return { prefix: apiKey.slice(0, 8), suffix: apiKey.slice(-4) }
}

function maskMatches(sensitiveId, mask) {
  if (!mask || typeof sensitiveId !== 'string') return false
  return sensitiveId.startsWith(mask.prefix) && sensitiveId.endsWith(mask.suffix)
}

// Sum cost / tokens / requests from the by_api_key payloads, restricted to the
// series that belong to the current API key (mask match), else valid series,
// else everything. Pure function, exported for tests.
// cost biz_data: { start, end, bucket, models, data: [{ currency, series:
// [{ api_key: {sensitive_id, valid}, model, buckets: [{time, cost}] }] }] }
// amount biz_data: same series shape but `series` sits at the TOP level
// ({ start, end, bucket, models, series: [...] }), and buckets carry
// {time, usage: {RESPONSE_TOKEN, REQUEST, PROMPT_CACHE_HIT_TOKEN,
// PROMPT_CACHE_MISS_TOKEN, ...}}.
function parseByApiKeyUsage(costPayload, amountPayload, apiKey) {
  const bizOf = (payload) => {
    if (!payload || typeof payload !== 'object') return null
    const data = payload.data && typeof payload.data === 'object' ? payload.data : null
    return data && data.biz_data ? data.biz_data : null
  }
  const mask = keyMaskOf(apiKey)
  const costBiz = bizOf(costPayload)
  const amountBiz = bizOf(amountPayload)
  const seriesOf = (biz) => {
    const out = []
    for (const cur of biz?.data ?? []) {
      for (const s of cur.series ?? []) out.push(s)
    }
    for (const s of biz?.series ?? []) out.push(s) // amount payload: top level
    return out
  }
  const pick = (series) => {
    const matched = series.filter((s) => maskMatches(s.api_key?.sensitive_id, mask))
    if (matched.length > 0) return matched
    const valid = series.filter((s) => s.api_key?.valid === true)
    return valid.length > 0 ? valid : series
  }
  const bucketNum = (b) => {
    if (typeof b === 'number') return b
    if (!b || typeof b !== 'object') return 0
    for (const k of ['cost', 'amount', 'value']) {
      const v = b[k]
      if (typeof v === 'number' && Number.isFinite(v)) return v
      if (typeof v === 'string' && v.trim() !== '') {
        const n = Number(v.trim())
        if (Number.isFinite(n)) return n
      }
    }
    return 0
  }
  let costCny = 0
  for (const s of pick(seriesOf(costBiz))) {
    for (const b of s.buckets ?? []) costCny += bucketNum(b)
  }
  let tokens = 0
  let requests = 0
  for (const s of pick(seriesOf(amountBiz))) {
    for (const b of s.buckets ?? []) {
      const usage = b && typeof b.usage === 'object' ? b.usage : null
      if (!usage) continue
      for (const [type, value] of Object.entries(usage)) {
        const n = typeof value === 'number' ? value : Number(value)
        if (!Number.isFinite(n)) continue
        if (type === 'REQUEST') requests += n
        else if (type !== 'PROMPT_TOKEN' && type !== 'PROMPT_CACHE_WRITE_TOKEN') tokens += n
      }
    }
  }
  return { costCny, tokens, requests }
}

// Never throws: returns { ok:true, ...parsed } or { ok:false, error }.
async function queryPlatformToday(tzSec, apiKey) {
  const token = readPlatformToken()
  if (!token) return { ok: false, error: 'DEEPSEEK_PLATFORM_TOKEN not configured' }
  const tz = Number.isFinite(tzSec) ? Math.round(tzSec) : DEFAULT_TZ_SEC
  const start = startOfDaySec(tz)
  const end = start + 86400
  const key = `${tz}-${start}`
  const now = Date.now()
  if (platformCache.key === key && now - platformCache.at < PLATFORM_CACHE_TTL_MS) return platformCache.value
  let amountRes
  let costRes
  try {
    const fetchJson = async (url) => {
      const res = await fetch(url, {
        headers: { Authorization: `Bearer ${token}`, Accept: 'application/json' },
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
      })
      return { status: res.status, body: await res.json().catch(() => null) }
    }
    ;[amountRes, costRes] = await Promise.all([
      fetchJson(`${PLATFORM_USAGE_AMOUNT_URL}?start=${start}&end=${end}&tz=${tz}`),
      fetchJson(`${PLATFORM_USAGE_COST_URL}?start=${start}&end=${end}&tz=${tz}`),
    ])
  } catch (error) {
    return { ok: false, error: `platform usage request failed: ${error?.message ?? error}` }
  }
  if (amountRes.status === 401 || amountRes.status === 403 || costRes.status === 401 || costRes.status === 403) {
    return { ok: false, error: 'platform token invalid or expired' }
  }
  const parsed = parseByApiKeyUsage(costRes.body, amountRes.body, apiKey || readDeepseekApiKeyFromYaml())
  if (!(parsed.costCny > 0) && !(parsed.tokens > 0) && !(parsed.requests > 0)) {
    return { ok: false, error: 'unexpected platform usage response' }
  }
  const value = { ok: true, costCny: parsed.costCny, tokens: parsed.tokens, requests: parsed.requests, date: dateKeyInTz(start, tz), currency: 'CNY' }
  platformCache = { key, at: Date.now(), value }
  return value
}

// One consistent cut over the registered session projections (sessionStats +
// tokenUsage) for one session, served straight from dsh's own fold. Services
// are resolved lazily from the plugin ctx at request time (they are always
// available by then), so the routes register on a webServer-only inject.
function resolveSessionServices(ctx) {
  let sessions
  let sessionProjections
  try {
    sessions = ctx.get('sessions')
    sessionProjections = ctx.get('sessionProjections')
  } catch (error) {
    sessions = undefined
    sessionProjections = undefined
  }
  if (!sessions || !sessionProjections) {
    const error = new Error('session services unavailable (sessions/sessionProjections not running)')
    error.status = 503
    throw error
  }
  return { sessions, sessionProjections }
}

function readSessionUsage(ctx, sessionId) {
  const { sessions, sessionProjections } = resolveSessionServices(ctx)
  const session = sessions.get(sessionId)
  if (!session) {
    const error = new Error(`session not found: ${sessionId}`)
    error.status = 404
    throw error
  }
  let snapshot
  try {
    snapshot = sessionProjections.snapshot(session)
  } catch (error) {
    const wrapped = new Error(`session projections unavailable: ${error?.message ?? error}`)
    wrapped.status = 500
    throw wrapped
  }
  const values = snapshot?.values ?? {}
  return {
    sessionStats: values.sessionStats ?? null,
    tokenUsage: values.tokenUsage ?? null,
    asOfSeq: snapshot?.asOfSeq ?? null,
    model: lastModelOf(session?.log),
  }
}

function registerSessionsRoute(scope, ctx) {
  scope.webServer.register({
    name: 'deepseek-sessions-usage',
    kind: 'exact',
    path: '/dsh-usage/sessions',
    handler: async (req, res) => {
      if (req.method !== 'GET') {
        res.writeHead(405).end()
        return
      }
      try {
        let sessions = []
        let sessionProjections = null
        try {
          const services = resolveSessionServices(ctx)
          sessions = services.sessions.list() ?? []
          sessionProjections = services.sessionProjections
        } catch (error) {
          sessions = []
          sessionProjections = null
        }
        // Most recent first, capped before projection folds (each snapshot
        // replays that session's log on first read).
        const recent = [...sessions]
          .sort((a, b) => (b.header?.createdAt ?? 0) - (a.header?.createdAt ?? 0))
          .slice(0, 20)
        const rows = []
        for (const session of recent) {
          let snap = null
          if (sessionProjections) {
            try {
              snap = sessionProjections.snapshot(session)
            } catch (error) {
              snap = null
            }
          }
          const values = snap?.values ?? {}
          const stats = values.sessionStats ?? null
          const usage = values.tokenUsage ?? null
          // One pass over the log yields both the effective model (for pricing)
          // and the latest title, instead of two separate full walks.
          const meta = scanLogMeta(session.log)
          const cost = estimateCost(usage, meta.model)
          const billedInput = cost.uncached + cost.cacheRead + cost.cacheWrite
          rows.push({
            sessionId: session.id,
            title: meta.title,
            createdAt: session.header?.createdAt ?? null,
            turns: stats?.turns ?? 0,
            steps: stats?.steps ?? 0,
            llmMs: stats?.llmMs ?? 0,
            billedInput,
            uncachedInput: cost.uncached,
            cacheRead: cost.cacheRead,
            cacheWrite: cost.cacheWrite,
            output: cost.output,
            costCny: cost.cny,
          })
        }
        const used = rows.filter((row) => row.steps > 0 || row.billedInput > 0 || row.output > 0)
        const totals = used.reduce(
          (acc, row) => {
            acc.billedInput += row.billedInput
            acc.uncachedInput += row.uncachedInput
            acc.cacheRead += row.cacheRead
            acc.cacheWrite += row.cacheWrite
            acc.output += row.output
            acc.costCny += row.costCny
            return acc
          },
          { billedInput: 0, uncachedInput: 0, cacheRead: 0, cacheWrite: 0, output: 0, costCny: 0 },
        )
        // Today's spend: every usage event that actually occurred today (local
        // calendar day) across ALL sessions — not just the recent-20 list
        // above, and regardless of when each session was created. The fold
        // reads the in-memory log directly, so it only ever looks at today's
        // events and never replays full history. A per-session memo makes a
        // session that has not grown since the last read free to reuse, so the
        // 60s poll does not re-walk every unchanged log on each tick.
        const now = Date.now()
        const todayKey = localDayKey(now)
        const today = { date: todayKey, costCny: 0, count: 0, steps: 0, source: 'estimate' }
        {
          const start = new Date()
          start.setHours(0, 0, 0, 0)
          const startMs = start.getTime()
          const endMs = now
          const seen = new Set()
          for (const session of sessions) {
            const log = session?.log
            if (!Array.isArray(log)) continue
            const sid = session.id
            seen.add(sid)
            const cached = todayWindowCache.get(sid)
            if (cached && cached.dayKey === todayKey && cached.logLen === log.length) {
              // Fresh within the day and unchanged since the last read.
              if (cached.steps > 0) {
                today.costCny += cached.costCny
                today.count += 1
                today.steps += cached.steps
              }
              continue
            }
            const folded = foldUsageInWindow(log, startMs, endMs)
            todayWindowCache.set(sid, { dayKey: todayKey, logLen: log.length, costCny: folded.costCny, steps: folded.steps })
            if (folded.steps > 0) {
              today.costCny += folded.costCny
              today.count += 1
              today.steps += folded.steps
            }
          }
          // Bound the memo to live sessions.
          for (const key of todayWindowCache.keys()) {
            if (!seen.has(key)) todayWindowCache.delete(key)
          }
        }
        // Prefer the real billed number from the DeepSeek platform when the
        // browser session token is configured; the local estimate only counts
        // sessions visible to this process, so it is usually far below the
        // platform's full-account daily cost. The client passes its timezone
        // offset so the day window matches the usage page's "今日" exactly.
        const tzParam = new URL(req.url, 'http://localhost').searchParams.get('tz')
        const tzSec = tzParam !== null && Number.isFinite(Number(tzParam)) ? Number(tzParam) : undefined
        const platform = await queryPlatformToday(tzSec, await resolveApiKey(ctx))
        if (platform.ok) {
          today.costCny = platform.costCny
          today.source = 'platform'
          today.date = platform.date || today.date
          today.platformTokens = platform.tokens
          today.platformRequests = platform.requests
          today.currency = platform.currency
        } else if (platform.error) {
          today.platformError = platform.error
        }
        res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' })
        res.end(JSON.stringify({ ok: true, sessions: used, totals, today, pricing: PRICING, fetchedAt: Date.now() }))
      } catch (error) {
        res.writeHead(500, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ ok: false, error: String(error?.message ?? error) }))
      }
    },
  })
}

function registerBalanceRoute(scope, ctx) {
  scope.webServer.register({
    name: 'deepseek-balance',
    kind: 'exact',
    path: '/dsh-balance/deepseek',
    handler: async (req, res) => {
      if (req.method !== 'GET') {
        res.writeHead(405).end()
        return
      }
      try {
        const body = await queryDeepseekBalance(ctx)
        res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' })
        res.end(JSON.stringify({ ok: true, ...body, fetchedAt: Date.now() }))
      } catch (error) {
        res.writeHead(500, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ ok: false, error: String(error?.message ?? error), ...(error?.code ? { code: error.code } : {}), ...(error?.hint ? { hint: error.hint } : {}) }))
      }
    },
  })
}

function registerUsageRoute(scope, ctx) {
  scope.webServer.register({
    name: 'deepseek-usage',
    kind: 'exact',
    path: '/dsh-usage/details',
    handler: async (req, res) => {
      if (req.method !== 'GET') {
        res.writeHead(405).end()
        return
      }
      try {
        const sessionId = new URL(req.url, 'http://localhost').searchParams.get('session') ?? ''
        if (!sessionId) {
          const error = new Error('missing session parameter')
          error.status = 400
          throw error
        }
        const [balance, usage] = await Promise.all([queryDeepseekBalance(ctx), readSessionUsage(ctx, sessionId)])
        usage.cost = estimateCost(usage.tokenUsage, usage.model)
        res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' })
        res.end(JSON.stringify({ ok: true, balance, ...usage, fetchedAt: Date.now() }))
      } catch (error) {
        const status = typeof error?.status === 'number' ? error.status : 500
        res.writeHead(status, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ ok: false, error: String(error?.message ?? error), ...(error?.code ? { code: error.code } : {}), ...(error?.hint ? { hint: error.hint } : {}) }))
      }
    },
  })
}

export function apply(ctx, _config = {}) {
  // webServer exists only under the web profile; the scoped inject runs when
  // the service appears and never runs where it does not (headless stays
  // untouched). sessions/sessionProjections are core web-profile services.
  if (typeof ctx.inject === 'function') {
    // One webServer-only inject (the same shape the balance route already
    // proves works). sessions/sessionProjections are resolved lazily inside
    // the handlers, so no multi-service inject can silently stall.
    ctx.inject(['webServer'], (scope) => {
      try {
        registerBalanceRoute(scope, ctx)
        registerUsageRoute(scope, ctx)
        registerSessionsRoute(scope, ctx)
      } catch (error) {
        console.error(`[deepseek-balance] routes skipped: ${error}`)
      }
    })
  }
}

// Pure helpers, exported for tests only (cordis ignores extra exports).
export { PRICING, pricingOf, estimateCost, lastModelOf, foldUsageInWindow, parseByApiKeyUsage, keyMaskOf, startOfDaySec, dateKeyInTz }