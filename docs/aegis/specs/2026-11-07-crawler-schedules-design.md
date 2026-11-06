# Per-source crawler schedules design

Date: `2026-11-07`
Status: `approved-design`

## Intent

Add persistent per-source crawler schedules to the admin backend. Each supported
problem source can be enabled independently, use a five-field Cron expression,
choose an IANA timezone, and pass the same validated crawler arguments as the
existing manual trigger.

## Success evidence

- An administrator can read and update one schedule for each supported problem
  source from the crawler admin page.
- Basic controls generate a valid Cron expression and advanced controls accept a
  validated raw Cron expression.
- The default timezone is `UTC`; a source may override it with an IANA timezone.
- Enabled schedules survive process restart and run at the next matching
  occurrence.
- Scheduled jobs use the existing crawler execution, timeout, output, progress,
  history, and artifact behavior.
- A scheduled run does not duplicate a crawler already running for the same
  source.
- Invalid Cron expressions, timezones, sources, and arguments are rejected
  without replacing the active saved configuration.

## Scope

In scope:

- LeetCode, AtCoder, Codeforces, Luogu, and SPOJ problem crawlers.
- SQLite persistence through the existing `app_settings` table.
- Admin API read/update endpoints.
- Basic and advanced crawler schedule controls.
- Per-source enablement, Cron expression, timezone, and arguments.
- Scheduled job history and status visibility.

Out of scope:

- Scheduling the diagnostic crawler.
- Replacing the existing additional daily-source scheduler.
- Distributed locks, a durable task queue, or retry policies beyond the next
  Cron occurrence.
- Editing the system configuration file from the admin UI.

## Data contract

The persisted setting uses the existing `app_settings` key
`crawler_schedules`. Its value is JSON:

```json
{
  "leetcode": {
    "enabled": true,
    "cron": "30 3 * * *",
    "timezone": "UTC",
    "args": ["--full"]
  }
}
```

Missing sources use disabled defaults with an empty Cron expression, an empty
timezone override (inheriting the global UTC default), and no operation arguments. The server exposes all supported sources in a stable
order. The API wraps the configuration as `{source, schedule, effective_timezone,
next_run_at, next_runs, last_job, last_event}`. Updates contain only
`{source, schedule}` so saving one source does not overwrite another source.

Cron input is five-field standard Cron, parsed directly by `croner` with
standard Sunday 0/7 and day-of-month/day-of-week OR semantics. `chrono-tz`
validates and evaluates IANA timezone names. Fixed local times run once in a
fall-back overlap and move to the first valid local time in a spring-forward gap.

## Runtime behavior

The scheduler loads the persisted schedule set at startup and runs one
background task. It waits until the earliest enabled occurrence, wakes early
when settings change, and recalculates all next occurrences. Each scheduled
source uses runtime key `scheduled:<source>`.

Before launching a scheduled job, the scheduler checks all running crawler jobs
for the same source. If one exists, it records a skip log and waits for the
next scheduled occurrence. A scheduled job is represented by
`CrawlerTrigger::Scheduled` and reuses the generic crawler runner used by manual
jobs. Scheduled jobs are not cancellable from the existing manual cancel
endpoint.

## Admin experience

The crawler page keeps the existing manual trigger controls and adds a
per-source schedule cards. Each source card shows the enabled switch, source, basic or
advanced editor, timezone, generated/raw Cron, next run, last run, and save
state.

Basic mode offers common controls for hourly, daily, weekly, and monthly
patterns. It renders the generated Cron as a preview. Advanced mode accepts a
raw five-field expression and shows the next five occurrences after validation.
Both modes update the same API contract and preserve the selected timezone and
arguments.

## Compatibility and ownership

The existing manual trigger, daily fallback scheduler, job artifact format,
history endpoints, and public APIs remain compatible. The new general
scheduler owns only persisted per-source schedules and delegates actual process
execution to the shared crawler runner. The existing daily scheduler remains the
owner of additional daily sources.

## Verification

- Unit tests cover five-field normalization, Cron validation, timezone parsing,
  next occurrence calculation, basic-form conversion, persisted defaults, and
  schedule update rejection.
- Handler tests cover GET/PUT schedule responses, invalid input, and
  notification of the scheduler.
- Scheduler tests use a fake clock/input or pure next-occurrence helpers and
  verify same-source conflict skipping.
- Existing Rust tests and Python crawler tests remain green.
