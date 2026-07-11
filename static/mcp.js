'use strict';

(function initializeMcpReference() {
    const feedback = document.querySelector('.copy-feedback');
    const copyButtons = Array.from(document.querySelectorAll('[data-copy-target]'));
    let feedbackTimeout;

    function translate(key, fallback) {
        if (!window.i18n || typeof window.i18n.t !== 'function') return fallback;
        const value = window.i18n.t(key);
        return value === key ? fallback : value;
    }

    function openReferenceFromHash() {
        const hash = window.location.hash;
        if (!hash || hash.length <= 1) return;
        const container = document.getElementById(hash.slice(1));
        if (!container || !container.classList.contains('mcp-reference-row')) return;
        const details = container.querySelector('.reference-details');
        if (details) details.open = true;
    }

    async function writeClipboard(text) {
        if (navigator.clipboard && window.isSecureContext) {
            await navigator.clipboard.writeText(text);
            return;
        }

        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.setAttribute('readonly', '');
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        const copied = document.execCommand('copy');
        textarea.remove();
        if (!copied) throw new Error('copy command failed');
    }

    async function copyExample(button) {
        const target = document.getElementById(button.dataset.copyTarget);
        if (!target) return;
        const label = button.querySelector('[data-copy-label]');

        try {
            await writeClipboard(target.textContent);
            const copiedText = translate('docs_mcp.copy.copied', 'Copied');
            if (label) label.textContent = copiedText;
            if (feedback) feedback.textContent = copiedText;

            if (button._copyTimeout) window.clearTimeout(button._copyTimeout);
            if (feedbackTimeout) window.clearTimeout(feedbackTimeout);

            button._copyTimeout = window.setTimeout(() => {
                if (label) label.textContent = translate('docs_mcp.copy.copy', 'Copy');
                delete button._copyTimeout;
            }, 1800);
            feedbackTimeout = window.setTimeout(() => {
                if (feedback) feedback.textContent = '';
                feedbackTimeout = undefined;
            }, 1800);
        } catch (error) {
            const failedText = translate('docs_mcp.copy.failed', 'Copy failed');
            if (feedback) feedback.textContent = failedText;
        }
    }

    copyButtons.forEach((button) => {
        button.addEventListener('click', () => copyExample(button));
    });
    window.addEventListener('hashchange', openReferenceFromHash);
    document.addEventListener('DOMContentLoaded', openReferenceFromHash);
    document.addEventListener('languageChanged', () => {
        copyButtons.forEach((button) => {
            const label = button.querySelector('[data-copy-label]');
            if (label) label.textContent = translate('docs_mcp.copy.copy', 'Copy');
        });
    });
})();
