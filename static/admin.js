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

    function copyToClipboard(text) {
        if (navigator.clipboard) {
            navigator.clipboard.writeText(text).then(function() {
                toast('Copied to clipboard');
            });
        } else {
            var ta = document.createElement('textarea');
            ta.value = text;
            document.body.appendChild(ta);
            ta.select();
            document.execCommand('copy');
            ta.remove();
            toast('Copied to clipboard');
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
                    toast(enabled ? 'Token auth enabled' : 'Token auth disabled');
                } else {
                    toggle.checked = !enabled;
                    toast('Failed to update setting', 'error');
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
                toast('Failed to create token', 'error');
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
            '<td><span class="badge badge-active">Active</span></td>' +
            '<td><button class="btn btn-danger btn-sm btn-revoke">Revoke</button></td>';
        tbody.prepend(tr);
    }

    // Revoke token (event delegation)
    document.addEventListener('click', function(e) {
        if (!e.target.classList.contains('btn-revoke')) return;
        var row = e.target.closest('tr');
        if (!row || !row.dataset.token) return;
        showConfirm('Revoke this token?', function() {
            api('/admin/api/tokens/' + encodeURIComponent(row.dataset.token), {
                method: 'DELETE'
            }).then(function(res) {
                if (res.ok) {
                    row.remove();
                    toast('Token revoked');
                } else {
                    toast('Failed to revoke token', 'error');
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
        showConfirm('Delete this problem?', function() {
            api(url, { method: 'DELETE' }).then(function(res) {
                if (res.ok) {
                    var row = e.target.closest('tr');
                    if (row) row.remove();
                    toast('Problem deleted');
                } else {
                    toast('Failed to delete problem', 'error');
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
            '<button class="btn btn-danger btn-confirm-yes">Confirm</button>' +
            '<button class="btn btn-confirm-no" style="background:rgba(255,255,255,0.1)">Cancel</button>' +
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

    // Logout
    var logoutBtn = document.getElementById('logout-btn');
    if (logoutBtn) {
        logoutBtn.addEventListener('click', function(e) {
            e.preventDefault();
            fetch('/admin/logout', { method: 'POST', credentials: 'same-origin' })
                .then(function() { window.location.href = '/admin/login'; });
        });
    }
})();
