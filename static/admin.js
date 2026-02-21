(function() {
    'use strict';

    function api(url, opts = {}) {
        opts.credentials = 'same-origin';
        opts.headers = Object.assign({ 'Content-Type': 'application/json' }, opts.headers || {});
        return fetch(url, opts).then(function(res) {
            if (res.status === 401) {
                window.location.href = '/admin/login';
                return Promise.reject(new Error('unauthorized'));
            }
            return res;
        });
    }

    function toast(message, type) {
        var container = document.getElementById('toast-container');
        if (!container) return;
        var el = document.createElement('div');
        el.className = 'toast ' + (type || 'success');
        el.textContent = message;
        container.appendChild(el);
        setTimeout(function() {
            el.classList.add('fadeout');
            setTimeout(function() { el.remove(); }, 300);
        }, 3000);
    }

    function esc(s) {
        var d = document.createElement('div');
        d.textContent = s;
        return d.innerHTML;
    }

    function copyToClipboard(text) {
        if (navigator.clipboard) {
            navigator.clipboard.writeText(text).then(function() {
                toast(i18n.t('messages.copied'));
            });
        } else {
            var ta = document.createElement('textarea');
            ta.value = text;
            document.body.appendChild(ta);
            ta.select();
            document.execCommand('copy');
            ta.remove();
            toast(i18n.t('messages.copied'));
        }
    }

    // Token auth toggle
    var toggle = document.getElementById('token-auth-toggle');
    if (toggle) {
        toggle.addEventListener('change', function() {
            var enabled = toggle.checked;
            api('/admin/api/settings/token-auth', {
                method: 'PUT',
                body: JSON.stringify({ enabled: enabled })
            }).then(function(res) {
                if (res.ok) {
                    toast(enabled ? i18n.t('messages.token_auth_enabled') : i18n.t('messages.token_auth_disabled'));
                } else {
                    toggle.checked = !enabled;
                    toast(i18n.t('messages.failed_update_setting'), 'error');
                }
            }).catch(function() {
                toggle.checked = !enabled;
            });
        });
    }

    // Create token
    var createBtn = document.getElementById('create-token-btn');
    if (createBtn) {
        createBtn.addEventListener('click', function() {
            var labelInput = document.getElementById('token-label');
            var label = labelInput ? labelInput.value.trim() : null;
            api('/admin/api/tokens', {
                method: 'POST',
                body: JSON.stringify({ label: label || null })
            }).then(function(res) {
                if (res.ok) return res.json();
                throw new Error('failed');
            }).then(function(data) {
                if (labelInput) labelInput.value = '';
                showTokenModal(data.token);
                addTokenRow(data);
            }).catch(function() {
                toast(i18n.t('messages.failed_create_token'), 'error');
            });
        });
    }

    function showTokenModal(token) {
        var overlay = document.getElementById('token-modal');
        if (!overlay) return;
        var display = overlay.querySelector('.token-display');
        if (display) display.textContent = token;
        overlay.classList.add('active');

        var copyBtn = overlay.querySelector('.btn-copy');
        if (copyBtn) {
            copyBtn.onclick = function() { copyToClipboard(token); };
        }
        var closeBtn = overlay.querySelector('.btn-close-modal');
        if (closeBtn) {
            closeBtn.onclick = function() { overlay.classList.remove('active'); };
        }
        overlay.addEventListener('click', function(e) {
            if (e.target === overlay) overlay.classList.remove('active');
        });
    }

    function addTokenRow(token) {
        var tbody = document.querySelector('#tokens-table tbody');
        if (!tbody) return;
        var masked = token.token.substring(0, 8) + '...' + token.token.substring(token.token.length - 8);
        var tr = document.createElement('tr');
        tr.dataset.token = token.token;
        tr.innerHTML =
            '<td><code>' + masked + '</code></td>' +
            '<td>' + (token.label || '-') + '</td>' +
            '<td>' + new Date(token.created_at * 1000).toLocaleDateString() + '</td>' +
            '<td>-</td>' +
            '<td><span class="badge badge-active" data-i18n="common.active">' + i18n.t('common.active') + '</span></td>' +
            '<td><button class="btn btn-danger btn-sm btn-revoke" data-i18n="common.revoke">' + i18n.t('common.revoke') + '</button></td>';
        tbody.prepend(tr);
    }

    // Revoke token (event delegation)
    document.addEventListener('click', function(e) {
        if (!e.target.classList.contains('btn-revoke')) return;
        var row = e.target.closest('tr');
        if (!row || !row.dataset.token) return;
        showConfirm(i18n.t('messages.confirm_revoke'), function() {
            api('/admin/api/tokens/' + encodeURIComponent(row.dataset.token), {
                method: 'DELETE'
            }).then(function(res) {
                if (res.ok) {
                    row.remove();
                    toast(i18n.t('messages.token_revoked'));
                } else {
                    toast(i18n.t('messages.failed_revoke_token'), 'error');
                }
            });
        });
    });

    // Delete problem (event delegation)
    document.addEventListener('click', function(e) {
        if (!e.target.classList.contains('btn-delete-problem')) return;
        e.preventDefault();
        var url = e.target.dataset.url;
        if (!url) return;
        showConfirm(i18n.t('messages.confirm_delete_problem'), function() {
            api(url, { method: 'DELETE' }).then(function(res) {
                if (res.ok) {
                    var row = e.target.closest('tr');
                    if (row) row.remove();
                    toast(i18n.t('messages.problem_deleted'));
                } else {
                    toast(i18n.t('messages.failed_delete_problem'), 'error');
                }
            });
        });
    });

    // Confirm dialog
    function showConfirm(message, onConfirm) {
        var existing = document.querySelector('.confirm-overlay');
        if (existing) existing.remove();

        var overlay = document.createElement('div');
        overlay.className = 'confirm-overlay active';
        overlay.innerHTML =
            '<div class="confirm-box">' +
            '<p>' + message + '</p>' +
            '<div class="actions">' +
            '<button class="btn btn-danger btn-confirm-yes" data-i18n="common.confirm">' + i18n.t('common.confirm') + '</button>' +
            '<button class="btn btn-confirm-no" style="background:rgba(255,255,255,0.1)" data-i18n="common.cancel">' + i18n.t('common.cancel') + '</button>' +
            '</div></div>';
        document.body.appendChild(overlay);

        overlay.querySelector('.btn-confirm-yes').onclick = function() {
            overlay.remove();
            onConfirm();
        };
        overlay.querySelector('.btn-confirm-no').onclick = function() { overlay.remove(); };
        overlay.addEventListener('click', function(e) {
            if (e.target === overlay) overlay.remove();
        });
    }

    // Crawlers page
    var triggerBtn = document.getElementById('crawler-trigger-btn');
    if (triggerBtn) {
        var crawlerPollId = null;
        var selectedSource = 'leetcode';

        var CRAWLER_CONFIG = {
            leetcode: [
                { flag: '--init', i18nKey: 'init', type: 'checkbox' },
                { flag: '--full', i18nKey: 'full', type: 'checkbox' },
                { flag: '--daily', i18nKey: 'daily', type: 'checkbox' },
                { flag: '--date', i18nKey: 'date', type: 'date', placeholder: 'YYYY-MM-DD' },
                { flag: '--monthly', i18nKey: 'monthly', type: 'month-year' },
                { flag: '--fill-missing-content', i18nKey: 'fill_missing_content', type: 'checkbox' },
                { flag: '--fill-missing-content-workers', i18nKey: 'fill_missing_content_workers', type: 'number', placeholder: 'N', step: '1' },
                { flag: '--missing-content-stats', i18nKey: 'missing_content_stats', type: 'checkbox' }
            ],
            atcoder: [
                { flag: '--sync-kenkoooo', i18nKey: 'sync_kenkoooo', type: 'checkbox' },
                { flag: '--sync-history', i18nKey: 'sync_history', type: 'checkbox' },
                { flag: '--fetch-all', i18nKey: 'fetch_all', type: 'checkbox' },
                { flag: '--resume', i18nKey: 'resume', type: 'checkbox' },
                { flag: '--contest', i18nKey: 'contest', type: 'text', placeholder: 'Contest ID' },
                { flag: '--status', i18nKey: 'status', type: 'checkbox' },
                { flag: '--fill-missing-content', i18nKey: 'fill_missing_content', type: 'checkbox' },
                { flag: '--missing-content-stats', i18nKey: 'missing_content_stats', type: 'checkbox' },
                { flag: '--reprocess-content', i18nKey: 'reprocess_content', type: 'checkbox' },
                { flag: '--rate-limit', i18nKey: 'rate_limit', type: 'number', placeholder: 'seconds', step: '0.1' }
            ],
            codeforces: [
                { flag: '--sync-problemset', i18nKey: 'sync_problemset', type: 'checkbox' },
                { flag: '--fetch-all', i18nKey: 'fetch_all', type: 'checkbox' },
                { flag: '--resume', i18nKey: 'resume', type: 'checkbox' },
                { flag: '--contest', i18nKey: 'contest', type: 'number', placeholder: 'Contest ID', step: '1' },
                { flag: '--status', i18nKey: 'status', type: 'checkbox' },
                { flag: '--fill-missing-content', i18nKey: 'fill_missing_content', type: 'checkbox' },
                { flag: '--missing-content-stats', i18nKey: 'missing_content_stats', type: 'checkbox' },
                { flag: '--missing-problems', i18nKey: 'missing_problems', type: 'checkbox' },
                { flag: '--reprocess-content', i18nKey: 'reprocess_content', type: 'checkbox' },
                { flag: '--include-gym', i18nKey: 'include_gym', type: 'checkbox' },
                { flag: '--rate-limit', i18nKey: 'rate_limit', type: 'number', placeholder: 'seconds', step: '0.1' }
            ]
        };

        function renderArgs(source) {
            var container = document.getElementById('crawler-args-options');
            if (!container) return;
            container.innerHTML = '';
            var flags = CRAWLER_CONFIG[source] || [];
            flags.forEach(function(f) {
                var item = document.createElement('div');
                item.className = 'flag-item';

                var cb = document.createElement('input');
                cb.type = 'checkbox';
                cb.dataset.flag = f.flag;
                cb.id = 'flag-' + f.flag.replace(/^--/, '');

                var lbl = document.createElement('label');
                lbl.htmlFor = cb.id;
                lbl.textContent = i18n.t('crawlers.flags.' + f.i18nKey);

                item.appendChild(cb);
                item.appendChild(lbl);

                if (f.type === 'month-year') {
                    var yw = document.createElement('input');
                    yw.type = 'number';
                    yw.className = 'flag-input';
                    yw.placeholder = 'Year';
                    yw.min = '2000';
                    yw.max = '2100';
                    yw.dataset.role = 'year';
                    yw.disabled = true;

                    var mw = document.createElement('input');
                    mw.type = 'number';
                    mw.className = 'flag-input';
                    mw.placeholder = 'Month';
                    mw.min = '1';
                    mw.max = '12';
                    mw.dataset.role = 'month';
                    mw.disabled = true;

                    cb.addEventListener('change', function() { yw.disabled = !cb.checked; mw.disabled = !cb.checked; });
                    item.appendChild(yw);
                    item.appendChild(mw);
                } else if (f.type !== 'checkbox') {
                    var inp = document.createElement('input');
                    inp.type = f.type === 'date' ? 'date' : (f.type === 'number' ? 'number' : 'text');
                    inp.className = 'flag-input';
                    if (f.placeholder) inp.placeholder = f.placeholder;
                    if (f.step) inp.step = f.step;
                    inp.disabled = true;

                    cb.addEventListener('change', function() { inp.disabled = !cb.checked; });
                    item.appendChild(inp);
                }

                container.appendChild(item);
            });
        }

        function getArgs() {
            var args = [];
            var container = document.getElementById('crawler-args-options');
            if (!container) return args;
            var items = container.querySelectorAll('.flag-item');
            for (var i = 0; i < items.length; i++) {
                var cb = items[i].querySelector('input[type="checkbox"]');
                if (!cb || !cb.checked) continue;
                var flag = cb.dataset.flag;
                args.push(flag);

                var yearInp = items[i].querySelector('[data-role="year"]');
                if (yearInp) {
                    var monthInp = items[i].querySelector('[data-role="month"]');
                    if (!yearInp.value || !monthInp.value) {
                        toast(i18n.t('messages.fill_year_month') + flag, 'error');
                        return null;
                    }
                    args.push(yearInp.value);
                    args.push(monthInp.value);
                    continue;
                }

                var inp = items[i].querySelector('.flag-input');
                if (inp) {
                    if (!inp.value) {
                        toast(i18n.t('messages.fill_args') + flag, 'error');
                        return null;
                    }
                    args.push(inp.value);
                }
            }
            return args;
        }

        // Source selection
        document.querySelectorAll('.source-btn').forEach(function(btn) {
            btn.addEventListener('click', function() {
                document.querySelectorAll('.source-btn').forEach(function(b) { b.classList.remove('active'); });
                btn.classList.add('active');
                selectedSource = btn.dataset.source;
                renderArgs(selectedSource);
            });
        });

        // Initial render
        renderArgs(selectedSource);

        // Listen for language changes to re-render crawler args labels
        document.addEventListener('languageChanged', function() {
            renderArgs(selectedSource);
        });

        // Trigger
        triggerBtn.addEventListener('click', function() {
            var args = getArgs();
            if (args === null) return;
            triggerBtn.disabled = true;
            api('/admin/api/crawlers/trigger', {
                method: 'POST',
                body: JSON.stringify({ source: selectedSource, args: args })
            }).then(function(res) {
                if (res.ok) {
                    return res.json().then(function(data) {
                        toast(i18n.t('messages.crawler_triggered') + ': ' + data.job_id);
                        startPolling();
                    });
                } else {
                    return res.json().then(function(data) {
                        toast(data.detail || i18n.t('messages.failed_trigger_crawler'), 'error');
                        triggerBtn.disabled = false;
                    });
                }
            }).catch(function() {
                toast(i18n.t('messages.failed_trigger_crawler'), 'error');
                triggerBtn.disabled = false;
            });
        });

        // Polling
        function startPolling() {
            if (crawlerPollId) return;
            crawlerPollId = setInterval(pollStatus, 3000);
            pollStatus();
        }

        function stopPolling() {
            if (crawlerPollId) {
                clearInterval(crawlerPollId);
                crawlerPollId = null;
            }
        }

        function pollStatus() {
            api('/admin/api/crawlers/status').then(function(res) {
                if (!res.ok) return;
                return res.json();
            }).then(function(data) {
                if (!data) return;
                updateStatusCard(data);
                updateHistoryTable(data.history || []);
                if (!data.running) {
                    stopPolling();
                    triggerBtn.disabled = false;
                }
            });
        }

            function updateStatusCard(data) {
                var card = document.getElementById('crawler-status-card');
                if (!card) return;
                if (data.running && data.current_job) {
                    var job = data.current_job;
                    card.style.display = '';
                    card.innerHTML =
                        '<div class="status-header running" data-i18n="crawlers.status.running">' + i18n.t('crawlers.status.running') + '</div>' +
                        '<div class="status-details">' +
                        '<span><strong data-i18n="crawlers.status.job">' + i18n.t('crawlers.status.job') + '</strong>: ' + job.job_id + '</span> ' +
                        '<span><strong data-i18n="crawlers.status.source">' + i18n.t('crawlers.control.source') + '</strong>: ' + job.source + '</span> ' +
                        '<span><strong data-i18n="crawlers.status.args">' + i18n.t('crawlers.status.args') + '</strong>: ' + (job.args || []).join(' ') + '</span> ' +
                        '<span><strong data-i18n="crawlers.status.started">' + i18n.t('crawlers.status.started') + '</strong>: ' + job.started_at + '</span>' +
                        '</div>';
                } else {
                    card.style.display = 'none';
                }
            }
        
            function updateHistoryTable(history) {
                var tbody = document.querySelector('#crawler-history-table tbody');
                if (!tbody) return;
                tbody.innerHTML = '';
                history.forEach(function(job) {
                    var tr = document.createElement('tr');
                    var logBtn = job.status !== 'running'
                        ? '<button class="btn btn-sm btn-view-log" data-job-id="' + esc(job.job_id) + '" data-i18n="common.view">' + i18n.t('common.view') + '</button>'
                        : '-';
                    tr.innerHTML =
                        '<td>' + esc(job.source) + '</td>' +
                        '<td>' + esc((job.args || []).join(' ')) + '</td>' +
                        '<td>' + esc(job.trigger) + '</td>' +
                        '<td>' + esc(job.started_at) + '</td>' +
                        '<td>' + esc(job.finished_at || '-') + '</td>' +
                        '<td><span class="badge badge-crawler-' + esc(job.status) + '">' + esc(job.status) + '</span></td>' +
                        '<td>' + logBtn + '</td>';
                    tbody.appendChild(tr);
                });
            }
                // Log modal
        document.addEventListener('click', function(e) {
            if (!e.target.classList.contains('btn-view-log')) return;
            var jobId = e.target.dataset.jobId;
            if (!jobId) return;
            var modal = document.getElementById('log-modal');
            if (!modal) return;

            var stdoutPre = document.getElementById('log-stdout');
            var stderrPre = document.getElementById('log-stderr');
            stdoutPre.textContent = i18n.t('common.loading');
            stderrPre.textContent = '';
            stderrPre.style.display = 'none';
            stdoutPre.style.display = '';

            // Reset tabs
            modal.querySelectorAll('.log-tab').forEach(function(t) { t.classList.remove('active'); });
            modal.querySelector('[data-tab="stdout"]').classList.add('active');

            modal.classList.add('active');

            api('/admin/api/crawlers/' + encodeURIComponent(jobId) + '/output').then(function(res) {
                if (!res.ok) {
                    stdoutPre.textContent = i18n.t('messages.failed_load_output') + ' (HTTP ' + res.status + ')';
                    return;
                }
                return res.json();
            }).then(function(data) {
                if (!data) return;
                stdoutPre.textContent = data.stdout || '(empty)';
                stderrPre.textContent = data.stderr || '(empty)';
            }).catch(function() {
                stdoutPre.textContent = i18n.t('messages.failed_load_output');
            });
        });

        // Log modal tabs
        document.addEventListener('click', function(e) {
            if (!e.target.classList.contains('log-tab')) return;
            var tab = e.target.dataset.tab;
            var modal = document.getElementById('log-modal');
            if (!modal) return;
            modal.querySelectorAll('.log-tab').forEach(function(t) { t.classList.remove('active'); });
            e.target.classList.add('active');
            document.getElementById('log-stdout').style.display = tab === 'stdout' ? '' : 'none';
            document.getElementById('log-stderr').style.display = tab === 'stderr' ? '' : 'none';
        });

        // Close log modal
        document.addEventListener('click', function(e) {
            if (e.target.classList.contains('btn-close-log')) {
                document.getElementById('log-modal').classList.remove('active');
            }
            if (e.target.id === 'log-modal') {
                e.target.classList.remove('active');
            }
        });
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape') {
                var modal = document.getElementById('log-modal');
                if (modal) modal.classList.remove('active');
            }
        });

        // Auto-start polling if already running
        var statusCard = document.getElementById('crawler-status-card');
        if (statusCard && statusCard.style.display !== 'none') {
            startPolling();
        }
    }

    // Logout
    var logoutBtn = document.getElementById('logout-btn');
    if (logoutBtn) {
        logoutBtn.addEventListener('click', function(e) {
            e.preventDefault();
            fetch('/admin/logout', { method: 'POST', credentials: 'same-origin' })
                .then(function() { window.location.href = '/admin/login'; });
        });
    }

    // Problems Page
    var problemsTable = document.getElementById('problems-table');
    if (problemsTable) {
        var currentSource = 'leetcode';
        var currentPage = 1;

        function loadProblems(source, page) {
            currentSource = source || currentSource;
            currentPage = page || 1;
            var tbody = problemsTable.querySelector('tbody');
            tbody.innerHTML = '<tr><td colspan="7" style="text-align:center">' + i18n.t('common.loading') + '</td></tr>';

            api('/admin/api/problems/' + currentSource + '?page=' + currentPage + '&per_page=50')
                .then(function(res) { return res.json(); })
                .then(function(res) {
                    renderProblems(res.data);
                    updateStats(res.meta);
                    renderPagination(res.meta);
                })
                .catch(function(err) {
                    console.error('failed to load problems', err);
                    tbody.innerHTML = '<tr><td colspan="7" style="text-align:center;color:var(--color-danger)">Failed to load problems</td></tr>';
                });
        }

        function renderProblems(problems) {
            var tbody = problemsTable.querySelector('tbody');
            tbody.innerHTML = '';
            problems.forEach(function(p) {
                var tr = document.createElement('tr');
                var difficultyBadge = '';
                if (p.difficulty) {
                    var lower = p.difficulty.toLowerCase();
                    var badgeClass = 'badge-' + lower;
                    var i18nKey = 'problems.difficulty.' + lower;
                    var label = i18n.t(i18nKey);
                    if (label === i18nKey) label = p.difficulty; // fallback
                    difficultyBadge = '<span class="badge ' + badgeClass + '">' + label + '</span>';
                }

                var title = p.title || '-';
                if (i18n.getLanguage() !== 'en' && p.title_cn) {
                    title = p.title_cn;
                }

                tr.innerHTML =
                    '<td>' + p.source + '</td>' +
                    '<td>' + p.id + '</td>' +
                    '<td>' + esc(title) + '</td>' +
                    '<td>' + difficultyBadge + '</td>' +
                    '<td>' + (p.rating || '-') + '</td>' +
                    '<td>' + (p.ac_rate ? p.ac_rate.toFixed(1) + '%' : '-') + '</td>' +
                    '<td>' +
                    '<button class="btn btn-sm btn-primary btn-view-detail" data-source="' + p.source + '" data-id="' + p.id + '" style="margin-right:0.4rem">' + i18n.t('common.detail') + '</button>' +
                    '<button class="btn btn-danger btn-sm btn-delete-problem" data-url="/admin/api/problems/' + p.source + '/' + p.id + '">' + i18n.t('common.delete') + '</button>' +
                    '</td>';
                tbody.appendChild(tr);
            });
        }

        function updateStats(meta) {
            document.getElementById('total-count').textContent = meta.total;
            document.getElementById('current-page').textContent = meta.page;
            document.getElementById('total-pages').textContent = meta.total_pages;
        }

        function renderPagination(meta) {
            var container = document.getElementById('problems-pagination');
            if (!container) return;
            container.innerHTML = '';
            if (meta.page > 1) {
                var prev = document.createElement('a');
                prev.href = '#';
                prev.innerHTML = i18n.t('problems.pagination.prev');
                prev.onclick = function(e) { e.preventDefault(); loadProblems(currentSource, meta.page - 1); };
                container.appendChild(prev);
            }
            if (meta.page < meta.total_pages) {
                var next = document.createElement('a');
                next.href = '#';
                next.innerHTML = i18n.t('problems.pagination.next');
                next.onclick = function(e) { e.preventDefault(); loadProblems(currentSource, meta.page + 1); };
                container.appendChild(next);
            }
        }

        // Source buttons
        var sourceBtns = document.getElementById('problem-source-btns');
        if (sourceBtns) {
            sourceBtns.querySelectorAll('.source-btn').forEach(function(btn) {
                btn.onclick = function() {
                    sourceBtns.querySelectorAll('.source-btn').forEach(function(b) { b.classList.remove('active'); });
                    btn.classList.add('active');
                    loadProblems(btn.dataset.source, 1);
                };
            });
        }

        // View detail
        document.addEventListener('click', function(e) {
            if (!e.target.classList.contains('btn-view-detail')) return;
            var source = e.target.dataset.source;
            var id = e.target.dataset.id;
            showProblemDetail(source, id);
        });

        function showProblemDetail(source, id) {
            var modal = document.getElementById('problem-detail-modal');
            if (!modal) return;
            
            var titleEl = document.getElementById('detail-title');
            var sourceIdEl = document.getElementById('detail-source-id');
            var difficultyEl = document.getElementById('detail-difficulty');
            var contentEl = document.getElementById('detail-content');

            titleEl.textContent = i18n.t('common.loading');
            sourceIdEl.textContent = '';
            difficultyEl.textContent = '';
            contentEl.textContent = '';
            modal.classList.add('active');

            api('/admin/api/problems/' + source + '/' + id)
                .then(function(res) { return res.json(); })
                .then(function(p) {
                    var title = p.title || '-';
                    if (i18n.getLanguage() !== 'en' && p.title_cn) {
                        title = p.title_cn;
                    }
                    titleEl.textContent = title;
                    sourceIdEl.textContent = p.source + ' ' + p.id;
                    if (p.difficulty) {
                        difficultyEl.textContent = p.difficulty;
                        difficultyEl.className = 'badge badge-' + p.difficulty.toLowerCase();
                    }
                    
                    var content = p.content || '';
                    if (i18n.getLanguage() !== 'en' && p.content_cn) {
                        content = p.content_cn;
                    }
                    contentEl.innerHTML = content; // Assuming content might have HTML/Markdown
                })
                .catch(function(err) {
                    titleEl.textContent = 'Error';
                    contentEl.textContent = 'Failed to load problem detail';
                });
        }

        // Close modal
        document.querySelectorAll('.btn-close-modal').forEach(function(btn) {
            btn.onclick = function() {
                document.getElementById('problem-detail-modal').classList.remove('active');
            };
        });
        var detailModal = document.getElementById('problem-detail-modal');
        if (detailModal) {
            detailModal.onclick = function(e) {
                if (e.target === detailModal) detailModal.classList.remove('active');
            };
        }

        // Initial load
        loadProblems(currentSource, 1);
        
        // Language change listener
        document.addEventListener('languageChanged', function() {
            loadProblems(currentSource, currentPage);
        });
    }
})();
