(function() {
    'use strict';

    function api(url, opts, retries) {
        opts = opts || {};
        retries = retries === undefined ? 2 : retries;
        opts.credentials = 'same-origin';
        opts.headers = Object.assign({ 'Content-Type': 'application/json' }, opts.headers || {});
        return fetch(url, opts).then(function(res) {
            if (res.status === 401) {
                window.location.href = '/admin/login';
                return Promise.reject(new Error('unauthorized'));
            }
            return res;
        }).catch(function(err) {
            if (err.message === 'unauthorized' || retries <= 0) throw err;
            return new Promise(function(resolve) {
                setTimeout(resolve, 1000);
            }).then(function() {
                return api(url, opts, retries - 1);
            });
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
            ],
            diag: [
                { flag: '--test', i18nKey: 'test', type: 'select', options: ['global', 'leetcode', 'atcoder', 'codeforces'] }
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
                } else if (f.type === 'select') {
                    var sel = document.createElement('select');
                    sel.className = 'flag-input';
                    sel.disabled = true;
                    (f.options || []).forEach(function(o) {
                        var opt = document.createElement('option');
                        opt.value = o;
                        opt.textContent = o;
                        sel.appendChild(opt);
                    });
                    cb.addEventListener('change', function() { sel.disabled = !cb.checked; });
                    item.appendChild(sel);
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
        var sourceBtns = document.getElementById('problem-source-btns');
        var activeBtn = sourceBtns ? sourceBtns.querySelector('.source-btn.active') : null;
        var currentSource = activeBtn ? activeBtn.dataset.source : 'leetcode';
        var currentPage = 1;
        var currentSearch = '';
        var currentDifficulty = '';
        var currentTags = [];
        var currentTagMode = 'any';
        var currentSortBy = '';
        var currentSortOrder = '';
        var currentPerPage = 50;
        var currentRatingMin = '';
        var currentRatingMax = '';

        function debounce(fn, ms) {
            var timer;
            return function() {
                var args = arguments;
                var ctx = this;
                clearTimeout(timer);
                timer = setTimeout(function() { fn.apply(ctx, args); }, ms);
            };
        }

        function parseUrlState() {
            var params = new URLSearchParams(window.location.search);
            if (params.get('source') && ['leetcode', 'atcoder', 'codeforces'].indexOf(params.get('source')) !== -1) {
                currentSource = params.get('source');
            }
            currentPage = parseInt(params.get('page'), 10) || 1;
            currentSearch = params.get('search') || '';
            currentDifficulty = params.get('difficulty') || '';
            currentPerPage = parseInt(params.get('per_page'), 10) || 50;
            currentSortBy = params.get('sort_by') || '';
            currentSortOrder = params.get('sort_order') || '';
            currentTagMode = params.get('tag_mode') || 'any';
            currentRatingMin = params.get('rating_min') || '';
            currentRatingMax = params.get('rating_max') || '';
            var tagsParam = params.get('tags') || '';
            currentTags = tagsParam ? tagsParam.split(',').filter(function(t) { return t; }) : [];
        }

        function syncUrlState() {
            var params = new URLSearchParams();
            params.set('source', currentSource);
            if (currentPage > 1) params.set('page', currentPage);
            if (currentSearch) params.set('search', currentSearch);
            if (currentDifficulty) params.set('difficulty', currentDifficulty);
            if (currentPerPage !== 50) params.set('per_page', currentPerPage);
            if (currentSortBy) params.set('sort_by', currentSortBy);
            if (currentSortOrder) params.set('sort_order', currentSortOrder);
            if (currentTags.length) params.set('tags', currentTags.join(','));
            if (currentTagMode !== 'any') params.set('tag_mode', currentTagMode);
            if (currentRatingMin) params.set('rating_min', currentRatingMin);
            if (currentRatingMax) params.set('rating_max', currentRatingMax);
            history.replaceState(null, '', '/admin/problems?' + params.toString());
        }

        function resetFilters() {
            currentPage = 1;
            currentSearch = '';
            currentDifficulty = '';
            currentTags = [];
            currentTagMode = 'any';
            currentSortBy = '';
            currentSortOrder = '';
            currentPerPage = 50;
            currentRatingMin = '';
            currentRatingMax = '';

            var searchInput = document.getElementById('problem-search');
            if (searchInput) searchInput.value = '';
            var diffSelect = document.getElementById('problem-difficulty');
            if (diffSelect) diffSelect.value = '';
            var ppSelect = document.getElementById('problem-per-page');
            if (ppSelect) ppSelect.value = '50';
            var modeBtn = document.getElementById('tag-mode-btn');
            if (modeBtn) modeBtn.textContent = 'OR';
            var ratingMinInput = document.getElementById('rating-min');
            if (ratingMinInput) ratingMinInput.value = '';
            var ratingMaxInput = document.getElementById('rating-max');
            if (ratingMaxInput) ratingMaxInput.value = '';

            updateSortHeaders();
            updateTagsBtnText();
        }

        function setSourceBtnsDisabled(disabled) {
            if (!sourceBtns) return;
            sourceBtns.querySelectorAll('.source-btn').forEach(function(b) {
                b.disabled = disabled;
            });
        }

        function loadProblems(source, page) {
            currentSource = source || currentSource;
            currentPage = page || 1;
            var tbody = problemsTable.querySelector('tbody');
            tbody.innerHTML = '<tr><td colspan="8" style="text-align:center">' + i18n.t('common.loading') + '</td></tr>';
            setSourceBtnsDisabled(true);

            var url = '/admin/api/problems/' + currentSource + '?page=' + currentPage + '&per_page=' + currentPerPage;
            if (currentSearch) url += '&search=' + encodeURIComponent(currentSearch);
            if (currentDifficulty) url += '&difficulty=' + encodeURIComponent(currentDifficulty);
            if (currentTags.length) url += '&tags=' + encodeURIComponent(currentTags.join(','));
            if (currentTagMode !== 'any') url += '&tag_mode=' + encodeURIComponent(currentTagMode);
            if (currentSortBy) url += '&sort_by=' + encodeURIComponent(currentSortBy);
            if (currentSortOrder) url += '&sort_order=' + encodeURIComponent(currentSortOrder);
            if (currentRatingMin) url += '&rating_min=' + encodeURIComponent(currentRatingMin);
            if (currentRatingMax) url += '&rating_max=' + encodeURIComponent(currentRatingMax);

            syncUrlState();

            api(url)
                .then(function(res) {
                    if (!res.ok) throw new Error('failed to load');
                    return res.json();
                })
                .then(function(res) {
                    renderProblems(res.data);
                    updateStats(res.meta);
                    renderPagination(res.meta);
                    setSourceBtnsDisabled(false);
                })
                .catch(function(err) {
                    console.error('failed to load problems', err);
                    tbody.innerHTML = '<tr><td colspan="8" style="text-align:center;color:var(--color-danger)">' + i18n.t('messages.failed_load_problems') + '</td></tr>';
                    toast(i18n.t('messages.failed_load_problems'), 'error');
                    setSourceBtnsDisabled(false);
                });
        }

        function loadTags(source) {
            var panel = document.getElementById('tags-panel');
            if (!panel) return;
            panel.innerHTML = '';
            api('/admin/api/tags/' + source).then(function(res) {
                if (!res.ok) return;
                return res.json();
            }).then(function(tags) {
                if (!tags || !tags.length) return;
                tags.forEach(function(tag) {
                    var item = document.createElement('label');
                    item.className = 'multi-select-item';
                    var cb = document.createElement('input');
                    cb.type = 'checkbox';
                    cb.value = tag;
                    if (currentTags.indexOf(tag) !== -1) cb.checked = true;
                    cb.addEventListener('change', function() {
                        if (cb.checked) {
                            if (currentTags.indexOf(tag) === -1) currentTags.push(tag);
                        } else {
                            currentTags = currentTags.filter(function(t) { return t !== tag; });
                        }
                        currentPage = 1;
                        updateTagsBtnText();
                        loadProblems();
                    });
                    var span = document.createElement('span');
                    span.textContent = tag;
                    item.appendChild(cb);
                    item.appendChild(span);
                    panel.appendChild(item);
                });
            });
        }

        function updateTagsBtnText() {
            var btn = document.getElementById('tags-select-btn');
            if (!btn) return;
            if (currentTags.length > 0) {
                var tmpl = i18n.t('problems.tags_selected');
                btn.textContent = tmpl.replace('{count}', currentTags.length);
            } else {
                btn.textContent = i18n.t('problems.tags_placeholder');
            }
        }

        function updateSortHeaders() {
            problemsTable.querySelectorAll('th[data-sort]').forEach(function(th) {
                th.classList.remove('sort-asc', 'sort-desc');
                if (th.dataset.sort === currentSortBy) {
                    if (currentSortOrder === 'asc') th.classList.add('sort-asc');
                    else if (currentSortOrder === 'desc') th.classList.add('sort-desc');
                }
            });
        }

        // Tags multi-select dropdown
        var tagsBtn = document.getElementById('tags-select-btn');
        var tagsPanel = document.getElementById('tags-panel');
        if (tagsBtn && tagsPanel) {
            tagsBtn.addEventListener('click', function(e) {
                e.stopPropagation();
                tagsPanel.classList.toggle('open');
            });
            document.addEventListener('click', function(e) {
                if (!e.target.closest('#tags-select')) {
                    tagsPanel.classList.remove('open');
                }
            });
        }

        // Tag mode toggle
        var tagModeBtn = document.getElementById('tag-mode-btn');
        if (tagModeBtn) {
            tagModeBtn.addEventListener('click', function() {
                currentTagMode = currentTagMode === 'any' ? 'all' : 'any';
                tagModeBtn.textContent = currentTagMode === 'any' ? 'OR' : 'AND';
                currentPage = 1;
                loadProblems();
            });
        }

        // Search input
        var searchInput = document.getElementById('problem-search');
        if (searchInput) {
            var debouncedSearch = debounce(function() {
                currentSearch = searchInput.value.trim();
                currentPage = 1;
                loadProblems();
            }, 300);
            searchInput.addEventListener('input', debouncedSearch);
            searchInput.addEventListener('keydown', function(e) {
                if (e.key === 'Enter') {
                    currentSearch = searchInput.value.trim();
                    currentPage = 1;
                    loadProblems();
                }
            });
        }

        // Difficulty select
        var diffSelect = document.getElementById('problem-difficulty');
        if (diffSelect) {
            diffSelect.addEventListener('change', function() {
                currentDifficulty = diffSelect.value;
                currentPage = 1;
                loadProblems();
            });
        }

        // Per-page select
        var ppSelect = document.getElementById('problem-per-page');
        if (ppSelect) {
            ppSelect.addEventListener('change', function() {
                currentPerPage = parseInt(ppSelect.value, 10) || 50;
                currentPage = 1;
                loadProblems();
            });
        }

        // Rating range inputs
        var ratingMinInput = document.getElementById('rating-min');
        var ratingMaxInput = document.getElementById('rating-max');
        if (ratingMinInput && ratingMaxInput) {
            var debouncedRating = debounce(function() {
                currentRatingMin = ratingMinInput.value.trim();
                currentRatingMax = ratingMaxInput.value.trim();
                currentPage = 1;
                loadProblems();
            }, 500);
            ratingMinInput.addEventListener('input', debouncedRating);
            ratingMaxInput.addEventListener('input', debouncedRating);
        }

        // Source-aware filter/column visibility
        function updateSourceVisibility(source) {
            var diffField = document.getElementById('difficulty-filter-field');
            var ratingField = document.getElementById('rating-range-field');
            var tagsSelect = document.getElementById('tags-select');
            var tagModeContainer = document.querySelector('.tag-mode-container');
            var showRating = source === 'leetcode' || source === 'codeforces';
            if (diffField) diffField.style.display = source === 'leetcode' ? '' : 'none';
            if (ratingField) ratingField.style.display = showRating ? '' : 'none';
            if (tagsSelect) tagsSelect.parentElement.style.display = source === 'atcoder' ? 'none' : '';
            if (tagModeContainer) tagModeContainer.style.display = source === 'atcoder' ? 'none' : '';
            problemsTable.classList.remove('source-leetcode', 'source-atcoder', 'source-codeforces');
            problemsTable.classList.add('source-' + source);
        }

        // Sortable headers
        problemsTable.querySelectorAll('th[data-sort]').forEach(function(th) {
            th.addEventListener('click', function() {
                var col = th.dataset.sort;
                if (currentSortBy === col) {
                    if (currentSortOrder === 'asc') {
                        currentSortOrder = 'desc';
                    } else if (currentSortOrder === 'desc') {
                        currentSortBy = '';
                        currentSortOrder = '';
                    } else {
                        currentSortOrder = 'asc';
                    }
                } else {
                    currentSortBy = col;
                    currentSortOrder = 'asc';
                }
                currentPage = 1;
                updateSortHeaders();
                loadProblems();
            });
        });

        function renderProblems(problems) {
            var tbody = problemsTable.querySelector('tbody');
            tbody.innerHTML = '';
            if (!problems.length) {
                tbody.innerHTML = '<tr><td colspan="8" style="text-align:center;color:var(--color-muted)">' + i18n.t('problems.no_results') + '</td></tr>';
                return;
            }
            problems.forEach(function(p) {
                var tr = document.createElement('tr');
                var difficultyBadge = '';
                if (p.difficulty) {
                    var lower = p.difficulty.toLowerCase();
                    var badgeClass = 'badge-' + lower;
                    var i18nKey = 'problems.difficulty.' + lower;
                    var label = i18n.t(i18nKey);
                    if (label === i18nKey) label = p.difficulty;
                    difficultyBadge = '<span class="badge ' + badgeClass + '">' + label + '</span>';
                }

                var title = p.title || '-';
                if (i18n.getLanguage() !== 'en' && p.title_cn) {
                    title = p.title_cn;
                }

                var tagsHtml = '-';
                if (p.tags && p.tags.length) {
                    tagsHtml = p.tags.map(function(t) {
                        return '<span class="table-tag">' + esc(t) + '</span>';
                    }).join(' ');
                }

                var ratingDisplay = '-';
                if (p.rating != null) {
                    ratingDisplay = currentSource === 'leetcode' ? p.rating.toFixed(2) : String(p.rating);
                }

                tr.innerHTML =
                    '<td>' + esc(p.source) + '</td>' +
                    '<td>' + esc(p.id) + '</td>' +
                    '<td>' + esc(title) + '</td>' +
                    '<td class="col-tags">' + tagsHtml + '</td>' +
                    '<td class="col-difficulty">' + difficultyBadge + '</td>' +
                    '<td class="col-rating">' + ratingDisplay + '</td>' +
                    '<td class="col-ac-rate">' + (p.ac_rate ? p.ac_rate.toFixed(1) + '%' : '-') + '</td>' +
                    '<td>' +
                    '<button class="btn btn-sm btn-primary btn-view-detail" data-source="' + esc(p.source) + '" data-id="' + esc(p.id) + '" style="margin-right:0.4rem">' + i18n.t('common.detail') + '</button>' +
                    '<button class="btn btn-danger btn-sm btn-delete-problem" data-url="/admin/api/problems/' + encodeURIComponent(p.source) + '/' + encodeURIComponent(p.id) + '">' + i18n.t('common.delete') + '</button>' +
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

            if (meta.total_pages <= 1) return;

            function createBtn(text, page, disabled) {
                var btn = document.createElement('a');
                btn.href = '#';
                btn.textContent = text;
                if (disabled) {
                    btn.classList.add('disabled');
                    btn.onclick = function(e) { e.preventDefault(); };
                } else {
                    btn.onclick = function(e) {
                        e.preventDefault();
                        currentPage = page;
                        loadProblems(currentSource, page);
                    };
                }
                return btn;
            }

            // First button
            container.appendChild(createBtn(i18n.t('problems.pagination.first'), 1, meta.page === 1));

            // Prev button
            container.appendChild(createBtn(i18n.t('problems.pagination.prev'), meta.page - 1, meta.page === 1));

            // Page numbers (show max 7 buttons)
            var start = Math.max(1, meta.page - 3);
            var end = Math.min(meta.total_pages, meta.page + 3);

            if (start > 1) {
                container.appendChild(createBtn('1', 1, false));
                if (start > 2) {
                    var ellipsis = document.createElement('span');
                    ellipsis.textContent = '...';
                    ellipsis.style.padding = '0 0.5rem';
                    container.appendChild(ellipsis);
                }
            }

            for (var i = start; i <= end; i++) {
                var btn = createBtn(String(i), i, false);
                if (i === meta.page) btn.classList.add('active');
                container.appendChild(btn);
            }

            if (end < meta.total_pages) {
                if (end < meta.total_pages - 1) {
                    var ellipsis = document.createElement('span');
                    ellipsis.textContent = '...';
                    ellipsis.style.padding = '0 0.5rem';
                    container.appendChild(ellipsis);
                }
                container.appendChild(createBtn(String(meta.total_pages), meta.total_pages, false));
            }

            // Next button
            container.appendChild(createBtn(i18n.t('problems.pagination.next'), meta.page + 1, meta.page === meta.total_pages));

            // Last button
            container.appendChild(createBtn(i18n.t('problems.pagination.last'), meta.total_pages, meta.page === meta.total_pages));
        }

        // View detail
        document.addEventListener('click', function(e) {
            var btn = e.target.closest('.btn-view-detail');
            if (!btn || btn.disabled) return;
            btn.disabled = true;
            var source = btn.dataset.source;
            var id = btn.dataset.id;
            showProblemDetail(source, id).finally(function() { btn.disabled = false; });
        });

        function showProblemDetail(source, id) {
            var modal = document.getElementById('problem-detail-modal');
            if (!modal) return Promise.resolve();

            var titleEl = document.getElementById('detail-title');
            var metaEl = document.getElementById('detail-meta');
            var fieldsEl = document.getElementById('detail-fields');
            var contentEl = document.getElementById('detail-content');
            var linkEl = document.getElementById('detail-link');

            titleEl.textContent = i18n.t('common.loading');
            metaEl.innerHTML = '';
            fieldsEl.innerHTML = '';
            contentEl.innerHTML = '';
            linkEl.style.display = 'none';
            openModal(modal);

            return api('/admin/api/problems/' + source + '/' + id)
                .then(function(res) {
                    if (!res.ok) throw new Error('HTTP ' + res.status);
                    return res.json();
                })
                .then(function(p) {
                    var title = p.title || '-';
                    if (i18n.getLanguage() !== 'en' && p.title_cn) title = p.title_cn;
                    titleEl.textContent = title;

                    // Meta: source/id + difficulty badge
                    var metaHtml = '<span class="detail-source-id">' + esc(p.source) + ' ' + esc(p.id) + '</span>';
                    if (p.difficulty) {
                        var lower = p.difficulty.toLowerCase();
                        var dKey = 'problems.difficulty.' + lower;
                        var dLabel = i18n.t(dKey);
                        if (dLabel === dKey) dLabel = p.difficulty;
                        metaHtml += ' <span class="badge badge-' + lower + '">' + dLabel + '</span>';
                    }
                    if (p.paid_only) metaHtml += ' <span class="badge badge-paid">' + esc(i18n.t('problems.detail.paid_only')) + '</span>';
                    metaEl.innerHTML = metaHtml;

                    // Fields
                    var rows = '';
                    if (p.slug) rows += '<div class="detail-row"><dt>Slug</dt><dd>' + esc(p.slug) + '</dd></div>';
                    if (p.rating) rows += '<div class="detail-row"><dt>' + esc(i18n.t('problems.table.rating')) + '</dt><dd>' + p.rating + '</dd></div>';
                    if (p.ac_rate != null) rows += '<div class="detail-row"><dt>' + esc(i18n.t('problems.table.ac_rate')) + '</dt><dd>' + p.ac_rate.toFixed(1) + '%</dd></div>';
                    if (p.contest) rows += '<div class="detail-row"><dt>' + esc(i18n.t('problems.detail.contest')) + '</dt><dd>' + esc(p.contest) + (p.problem_index ? ' / ' + esc(p.problem_index) : '') + '</dd></div>';
                    if (p.category) rows += '<div class="detail-row"><dt>' + esc(i18n.t('problems.detail.category')) + '</dt><dd>' + esc(p.category) + '</dd></div>';
                    if (p.tags && p.tags.length) {
                        rows += '<div class="detail-row"><dt>' + esc(i18n.t('problems.detail.tags')) + '</dt><dd>' +
                            p.tags.map(function(t) { return '<span class="detail-tag">' + esc(t) + '</span>'; }).join('') +
                            '</dd></div>';
                    }
                    if (p.similar_questions && p.similar_questions.length) {
                        rows += '<div class="detail-row"><dt>' + esc(i18n.t('problems.detail.similar')) + '</dt><dd>' +
                            p.similar_questions.map(function(q) { return '<span class="detail-tag">' + esc(q) + '</span>'; }).join('') +
                            '</dd></div>';
                    }
                    fieldsEl.innerHTML = rows;

                    // Content
                    var content = p.content || '';
                    if (i18n.getLanguage() !== 'en' && p.content_cn) content = p.content_cn;
                    contentEl.innerHTML = content;

                    // Link button
                    if (p.link) {
                        linkEl.href = p.link;
                        linkEl.style.display = '';
                    } else {
                        linkEl.style.display = 'none';
                    }
                })
                .catch(function() {
                    titleEl.textContent = i18n.t('messages.failed_load_detail');
                    fieldsEl.innerHTML = '<p style="color:var(--color-danger)">' + esc(i18n.t('messages.failed_load_detail')) + '</p>';
                });
        }

        // Focus trap and restore
        var previousFocus = null;

        function openModal(modal) {
            previousFocus = document.activeElement;
            modal.classList.add('active');
            var firstClose = modal.querySelector('.btn-close-modal');
            if (firstClose) firstClose.focus();
        }

        function closeModal() {
            var modal = document.getElementById('problem-detail-modal');
            if (modal) modal.classList.remove('active');
            if (previousFocus) { previousFocus.focus(); previousFocus = null; }
        }

        // Close modal
        document.querySelectorAll('.btn-close-modal').forEach(function(btn) {
            btn.onclick = closeModal;
        });
        var detailModal = document.getElementById('problem-detail-modal');
        if (detailModal) {
            detailModal.onclick = function(e) {
                if (e.target === detailModal) closeModal();
            };
            // Focus trap and Escape
            detailModal.addEventListener('keydown', function(e) {
                if (e.key === 'Escape') { closeModal(); return; }
                if (e.key !== 'Tab') return;
                var dialog = detailModal.querySelector('[role="dialog"]');
                if (!dialog) return;
                var focusable = dialog.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
                if (!focusable.length) return;
                var first = focusable[0];
                var last = focusable[focusable.length - 1];
                if (e.shiftKey) {
                    if (document.activeElement === first) { e.preventDefault(); last.focus(); }
                } else {
                    if (document.activeElement === last) { e.preventDefault(); first.focus(); }
                }
            });
        }

        // Source tab click handlers
        if (sourceBtns) {
            var tabs = Array.prototype.slice.call(sourceBtns.querySelectorAll('.source-btn'));

            function activateTab(tab) {
                var newSource = tab.dataset.source;
                if (newSource === currentSource) return;
                tabs.forEach(function(b) {
                    b.classList.remove('active');
                    b.setAttribute('aria-selected', 'false');
                    b.setAttribute('tabindex', '-1');
                });
                tab.classList.add('active');
                tab.setAttribute('aria-selected', 'true');
                tab.setAttribute('tabindex', '0');
                tab.focus();
                currentSource = newSource;
                resetFilters();
                updateSourceVisibility(currentSource);
                loadTags(currentSource);
                loadProblems(currentSource, 1);
            }

            tabs.forEach(function(btn, idx) {
                if (!btn.classList.contains('active')) btn.setAttribute('tabindex', '-1');
                btn.addEventListener('click', function() { activateTab(this); });
                btn.addEventListener('keydown', function(e) {
                    var next;
                    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                        next = tabs[(idx + 1) % tabs.length];
                    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                        next = tabs[(idx - 1 + tabs.length) % tabs.length];
                    } else if (e.key === 'Home') {
                        next = tabs[0];
                    } else if (e.key === 'End') {
                        next = tabs[tabs.length - 1];
                    }
                    if (next) { e.preventDefault(); activateTab(next); }
                });
            });
        }

        // Initial load
        parseUrlState();

        // Restore UI state from URL
        if (searchInput) searchInput.value = currentSearch;
        if (diffSelect) diffSelect.value = currentDifficulty;
        if (ppSelect) ppSelect.value = String(currentPerPage);
        if (tagModeBtn) tagModeBtn.textContent = currentTagMode === 'any' ? 'OR' : 'AND';
        if (ratingMinInput) ratingMinInput.value = currentRatingMin;
        if (ratingMaxInput) ratingMaxInput.value = currentRatingMax;

        // Activate correct source tab
        if (sourceBtns) {
            sourceBtns.querySelectorAll('.source-btn').forEach(function(b) {
                b.classList.remove('active');
                b.setAttribute('aria-selected', 'false');
                if (b.dataset.source === currentSource) {
                    b.classList.add('active');
                    b.setAttribute('aria-selected', 'true');
                }
            });
        }

        updateSortHeaders();
        updateTagsBtnText();
        updateSourceVisibility(currentSource);
        loadTags(currentSource);
        loadProblems(currentSource, currentPage);
        
        // Language change listener
        document.addEventListener('languageChanged', function() {
            updateTagsBtnText();
            loadProblems(currentSource, currentPage);
        });
    }
})();
