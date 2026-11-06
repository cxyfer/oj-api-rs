(function () {
    'use strict';
    var root = document.getElementById('crawler-schedules');
    if (!root) return;
    var defaultTimezone = 'UTC';
    var cards = {};
    function t(key) { return i18n.t('schedules.' + key); }
    function node(tag, text, className) {
        var el = document.createElement(tag);
        if (text !== undefined) el.textContent = text;
        if (className) el.className = className;
        return el;
    }
    function label(key) {
        var el = node('span', t(key));
        el.dataset.i18n = 'schedules.' + key;
        return el;
    }
    function field(form, key, control) {
        var wrapper = node('label', undefined, 'schedule-field');
        wrapper.appendChild(label(key));
        control.name = key;
        wrapper.appendChild(control);
        form.appendChild(wrapper);
        return control;
    }
    function select(values) {
        var el = node('select');
        values.forEach(function (value) {
            var option = node('option');
            option.value = value[0];
            if (value[2]) option.textContent = value[1];
            else { option.textContent = t(value[1]); option.dataset.i18n = 'schedules.' + value[1]; }
            el.appendChild(option);
        });
        return el;
    }
    function input(type, value) { var el = node('input'); el.type = type; el.value = value; return el; }
    function request(path, method, data) {
        return fetch('/admin/api/crawler-schedules' + path, {
            method: method, credentials: 'same-origin', headers: { 'Content-Type': 'application/json' },
            body: data ? JSON.stringify(data) : undefined
        }).then(function (res) {
            if (res.status === 401) { window.location.href = '/admin/login'; throw new Error(t('session_expired')); }
            return res.json().then(function (json) { if (!res.ok) throw new Error(json.detail || t('request_failed')); return json; });
        });
    }
    function formatDate(value, zone) {
        return new Intl.DateTimeFormat(i18n.getLanguage(), {
            timeZone: zone, year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23', timeZoneName: 'short'
        }).format(new Date(value));
    }
    function basicFromCron(cron) {
        var p = cron.trim().split(/\s+/);
        if (p.length !== 5 || !/^\d+$/.test(p[0])) return null;
        var time = String(p[1]).padStart(2, '0') + ':' + p[0].padStart(2, '0');
        if (p.slice(2).join(' ') === '* * *') {
            if (p[1] === '*') return { period: 'hourly', minute: p[0], interval: '1' };
            if (/^\*\/(2|3|4|6|8|12)$/.test(p[1])) return { period: 'interval', minute: p[0], interval: p[1].slice(2) };
            if (/^\d+$/.test(p[1])) return { period: 'daily', time: time };
        }
        if (!/^\d+$/.test(p[1])) return null;
        if (p[2] === '*' && p[3] === '*' && /^[0-7]$/.test(p[4])) return { period: 'weekly', time: time, weekday: String(Number(p[4]) % 7) };
        if (/^\d+$/.test(p[2]) && p[3] === '*' && p[4] === '*') return { period: 'monthly', time: time, day: p[2] };
        return null;
    }
    function buildCard(view) {
        var source = view.source;
        var card = node('article', undefined, 'card schedule-card');
        card.dataset.source = source;
        var title = node('h3', { leetcode: 'LeetCode', atcoder: 'AtCoder', codeforces: 'Codeforces', luogu: 'Luogu', spoj: 'SPOJ' }[source]);
        card.appendChild(title);
        var form = node('form'); form.noValidate = true;
        var fields = node('fieldset'); fields.className = 'schedule-fields'; form.appendChild(fields);
        var enabled = field(fields, 'enabled', input('checkbox', '')); enabled.checked = view.schedule.enabled;
        var mode = field(fields, 'mode', select([['basic', 'basic'], ['advanced', 'advanced']]));
        var basic = node('div', undefined, 'schedule-basic'); fields.appendChild(basic);
        var period = field(basic, 'period', select([['daily', 'daily'], ['weekly', 'weekly'], ['monthly', 'monthly'], ['hourly', 'hourly'], ['interval', 'interval'], ['custom', 'custom']]));
        var time = field(basic, 'time', input('time', '03:00')); time.required = true;
        var minute = field(basic, 'minute', input('number', '0')); minute.min = '0'; minute.max = '59'; minute.required = true;
        var hours = field(basic, 'hours', select(['2','3','4','6','8','12'].map(function (n) { return [n,n,true]; })));
        var weekday = field(basic, 'weekday', select(['sun','mon','tue','wed','thu','fri','sat'].map(function (day, i) { return [String(i), day]; })));
        var day = field(basic, 'day', input('number', '1')); day.min = '1'; day.max = '31'; day.required = true;
        var basicNote = node('p', undefined, 'schedule-note'); basicNote.appendChild(label('basic_note')); basic.appendChild(basicNote);
        var cron = field(fields, 'cron', input('text', view.schedule.cron)); cron.placeholder = '30 3 * * *'; cron.maxLength = 256;
        var cronHelp = node('p', undefined, 'schedule-note'); cronHelp.appendChild(label('cron_help')); fields.appendChild(cronHelp);
        var timezone = field(fields, 'timezone', input('text', view.schedule.timezone));
        timezone.setAttribute('list', 'crawler-timezones'); timezone.placeholder = defaultTimezone;
        var timezoneHelp = node('p', t('timezone_help') + ' ' + defaultTimezone, 'schedule-note'); fields.appendChild(timezoneHelp);
        var operation = field(fields, 'operation', select([['sync', 'sync'], ['content', 'content'], ['both', 'both'], ['custom', 'custom_args']]));
        var args = field(fields, 'args', node('textarea')); args.rows = 3; args.spellcheck = false; args.value = JSON.stringify(view.schedule.args);
        var preview = node('div', undefined, 'schedule-preview'); preview.setAttribute('aria-live', 'polite'); fields.appendChild(preview);
        var actions = node('div', undefined, 'schedule-actions');
        var previewBtn = node('button', undefined, 'btn'); previewBtn.type = 'button'; previewBtn.appendChild(label('preview'));
        var save = node('button', undefined, 'btn btn-primary'); save.type = 'submit'; save.appendChild(label('save'));
        var status = node('span', '', 'schedule-message'); status.setAttribute('role', 'status');
        actions.append(previewBtn, save, status); fields.appendChild(actions);
        card.appendChild(form);
        var result = node('div', undefined, 'schedule-result'); result.setAttribute('aria-live', 'polite'); card.appendChild(result);
        var revision = 0, timer = null, dirty = false, lastPreview = null, lastResult = view;
        function data() {
            var parsed;
            try { parsed = JSON.parse(args.value); } catch (_) { throw new Error(t('args_error')); }
            if (!Array.isArray(parsed) || !parsed.every(function (a) { return typeof a === 'string'; })) throw new Error(t('args_error'));
            return { source: source, schedule: { enabled: enabled.checked, cron: cron.value, timezone: timezone.value, args: parsed } };
        }
        function detectBasic() {
            var decoded = basicFromCron(cron.value);
            if (!decoded) { period.value = 'custom'; return false; }
            period.value = decoded.period;
            if (decoded.time) time.value = decoded.time;
            if (decoded.minute) minute.value = decoded.minute;
            if (decoded.interval) hours.value = decoded.interval;
            if (decoded.weekday) weekday.value = decoded.weekday;
            if (decoded.day) day.value = decoded.day;
            return true;
        }
        function visibility() {
            basic.hidden = mode.value !== 'basic'; cron.readOnly = mode.value === 'basic';
            time.parentNode.hidden = ['hourly','interval','custom'].includes(period.value);
            minute.parentNode.hidden = !['hourly','interval'].includes(period.value);
            hours.parentNode.hidden = period.value !== 'interval';
            weekday.parentNode.hidden = period.value !== 'weekly'; day.parentNode.hidden = period.value !== 'monthly';
            args.parentNode.hidden = mode.value !== 'advanced';
            cronHelp.hidden = mode.value !== 'advanced';
        }
        function generate() {
            if (period.value === 'custom') return;
            var hm = time.value.split(':');
            var m = String(Number(hm[1] || 0)), h = String(Number(hm[0] || 0));
            if (period.value === 'hourly') cron.value = minute.value + ' * * * *';
            if (period.value === 'interval') cron.value = minute.value + ' */' + hours.value + ' * * *';
            if (period.value === 'daily') cron.value = m + ' ' + h + ' * * *';
            if (period.value === 'weekly') cron.value = m + ' ' + h + ' * * ' + weekday.value;
            if (period.value === 'monthly') cron.value = m + ' ' + h + ' ' + day.value + ' * *';
        }
        function detectOperation(setDefault) {
            var raw = JSON.parse(args.value).slice();
            var sourceAt = raw.indexOf('--source'); if (sourceAt >= 0) raw.splice(sourceAt, 2);
            var sorted = raw.sort().join(' ');
            if (setDefault && !raw.length) { args.value = JSON.stringify(['--sync-problemset'].concat(JSON.parse(args.value))); sorted = '--sync-problemset'; }
            operation.value = sorted === '--sync-problemset' ? 'sync' : sorted === '--fill-missing-content' ? 'content' : sorted === '--fill-missing-content --sync-problemset' ? 'both' : 'custom';
        }
        function previewRuns() {
            clearTimeout(timer);
            var current = ++revision;
            if (!cron.value.trim()) { preview.textContent = ''; return; }
            preview.textContent = t('checking');
            var payload;
            try { payload = data(); } catch (err) { preview.textContent = err.message; return; }
            request('/preview', 'POST', payload).then(function (res) {
                if (current !== revision) return;
                lastPreview = res; renderPreview(res);
            }).catch(function (err) { if (current === revision) preview.textContent = err.message; });
        }
        function renderPreview(res) {
            preview.replaceChildren(label('next_five'));
            var list = node('ol');
            res.next_runs.forEach(function (date) { list.appendChild(node('li', formatDate(date, res.effective_timezone))); });
            preview.appendChild(list);
        }
        function changed() {
            lastPreview = null;
            dirty = true; ++revision;
            status.textContent = t('unsaved'); status.classList.remove('error');
            preview.textContent = '';
            clearTimeout(timer); timer = setTimeout(previewRuns, 500);
        }
        basic.addEventListener('input', function () { generate(); visibility(); });
        mode.addEventListener('change', function () { if (mode.value === 'basic') detectBasic(); visibility(); });
        operation.addEventListener('change', function () {
            if (operation.value !== 'custom') {
                var raw = [];
                if (operation.value !== 'content') raw.push('--sync-problemset');
                if (operation.value !== 'sync') raw.push('--fill-missing-content');
                args.value = JSON.stringify(raw);
            } else { mode.value = 'advanced'; visibility(); }
        });
        args.addEventListener('input', function () { try { detectOperation(); } catch (_) { operation.value = 'custom'; } });
        form.addEventListener('input', changed); form.addEventListener('change', changed);
        previewBtn.addEventListener('click', previewRuns);
        form.addEventListener('submit', function (event) {
            event.preventDefault();
            var payload;
            try { payload = data(); } catch (err) { status.textContent = err.message; status.classList.add('error'); return; }
            fields.disabled = true; status.textContent = t('saving');
            request('', 'PUT', payload).then(function (saved) {
                view = saved; dirty = false; cron.value = saved.schedule.cron; timezone.value = saved.schedule.timezone; args.value = JSON.stringify(saved.schedule.args);
                status.textContent = t('saved'); status.classList.remove('error'); showResult(saved);
            }).catch(function (err) { status.textContent = err.message; status.classList.add('error'); })
                .finally(function () { fields.disabled = false; });
        });
        function showResult(row) {
            lastResult = row;
            result.replaceChildren();
            var next = node('p'); next.append(label('next_run'), node('span', ': ' + (row.next_run_at ? formatDate(row.next_run_at, row.effective_timezone) : t('disabled')))); result.appendChild(next);
            var last = row.last_job;
            var recent = node('p'); recent.appendChild(label('last_run'));
            recent.appendChild(node('span', ': ' + (last ? last.status + ' · ' + formatDate(last.started_at, row.effective_timezone) : t('never')))); result.appendChild(recent);
            if (row.last_event && (!last || row.last_event.at > last.started_at)) {
                result.appendChild(node('p', row.last_event.message + ' · ' + formatDate(row.last_event.at, row.effective_timezone), 'schedule-note'));
            }
        }
        if (!cron.value) { period.value = 'daily'; generate(); }
        mode.value = detectBasic() ? 'basic' : 'advanced';
        if (!view.schedule.args.length) { operation.value = 'sync'; args.value = '["--sync-problemset"]'; } else detectOperation(true);
        visibility(); showResult(view); previewRuns();
        cards[source] = { update: showResult, isDirty: function () { return dirty; }, localize: function () {
            timezoneHelp.textContent = t('timezone_help') + ' ' + defaultTimezone;
            if (dirty && !status.classList.contains('error')) status.textContent = t('unsaved');
            if (!dirty && status.textContent) status.textContent = t('saved');
            showResult(lastResult);
            if (lastPreview) renderPreview(lastPreview);
        } };
        return card;
    }
    function load() {
        root.textContent = t('loading');
        request('', 'GET').then(function (res) {
            defaultTimezone = res.default_timezone;
            root.replaceChildren();
            var zones = node('datalist'); zones.id = 'crawler-timezones';
            res.timezones.forEach(function (zone) { var option = node('option'); option.value = zone; zones.appendChild(option); });
            root.appendChild(zones);
            res.schedules.forEach(function (view) { root.appendChild(buildCard(view)); });
            i18n.applyTranslations();
        }).catch(function (err) {
            root.textContent = err.message;
            var retry = node('button', t('retry'), 'btn'); retry.type = 'button'; retry.addEventListener('click', load); root.appendChild(retry);
        });
    }
    document.addEventListener('languageChanged', function () {
        // 語言切換只更新標籤，不重建表單，保留尚未儲存的輸入。
        root.querySelectorAll('[data-i18n]').forEach(function (el) { el.textContent = i18n.t(el.dataset.i18n); });
        Object.values(cards).forEach(function (card) { card.localize(); });
    });
    var poll = setInterval(function () {
        if (document.hidden || !Object.keys(cards).length) return;
        request('', 'GET').then(function (res) { res.schedules.forEach(function (row) { if (cards[row.source]) cards[row.source].update(row); }); }).catch(function () {});
    }, 15000);
    window.addEventListener('beforeunload', function (event) {
        if (Object.values(cards).some(function (c) { return c.isDirty(); })) { event.preventDefault(); event.returnValue = ''; }
    });
    window.addEventListener('pagehide', function () { clearInterval(poll); });
    load();
}());
