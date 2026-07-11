const MOBILE_QUERY = '(max-width: 760px)';
const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';
const INTERACTIVE_SELECTOR = 'a, button, input, label, select, textarea, summary';

const canvas = document.querySelector('[data-observatory-scene]');
const sourceElements = Array.from(document.querySelectorAll('.scene-source-data [data-source]'));

if (canvas && sourceElements.length > 0) {
    initializeScene();
}

function translate(key, fallback) {
    if (!window.i18n || typeof window.i18n.t !== 'function') return fallback;
    const value = window.i18n.t(key);
    return value === key ? fallback : value;
}

function initializeScene() {
    const shell = canvas.closest('.scene-shell');
    const hero = canvas.closest('.observatory-hero');
    const inspector = document.querySelector('.scene-inspector span:last-child');
    const idleInspectorFallback = inspector ? inspector.textContent : '';
    let idleInspectorText = translate('home.hero.inspector_idle', idleInspectorFallback);
    let worker = null;
    let workerFailed = false;
    let hoveredMetadata = null;
    let selectedMetadata = null;
    let pointerInside = false;
    let heroVisible = true;

    function activateWorkerFallback() {
        if (workerFailed) return;
        workerFailed = true;
        resetInteractionState();
        if (worker) worker.terminate();
        worker = null;
        if (shell) shell.classList.remove('is-ready');
        activateFallback(shell, canvas);
    }

    function post(message, transfer) {
        if (!worker || workerFailed) return;
        try {
            worker.postMessage(message, transfer || []);
        } catch (error) {
            activateWorkerFallback();
        }
    }

    function updateInspector() {
        if (!inspector) return;
        const metadata = hoveredMetadata || selectedMetadata;
        if (metadata) updateInspectorText(inspector, metadata);
        else inspector.textContent = idleInspectorText;
    }

    function refreshInspectorLanguage() {
        idleInspectorText = translate('home.hero.inspector_idle', idleInspectorFallback);
        updateInspector();
    }

    function clearSelectionDatasets() {
        delete canvas.dataset.selectedItem;
        delete canvas.dataset.selectedSource;
        delete canvas.dataset.selectedProblemTitle;
        delete canvas.dataset.selectedAccessPath;
    }

    function resetInteractionState() {
        hoveredMetadata = null;
        selectedMetadata = null;
        clearSelectionDatasets();
        updateInspector();
    }

    function handleSelection(metadata, identity) {
        selectedMetadata = metadata;
        clearSelectionDatasets();
        if (metadata && identity) {
            canvas.dataset.selectedItem = identity;
            canvas.dataset.selectedSource = metadata.source;
            if (metadata.problemTitle) canvas.dataset.selectedProblemTitle = metadata.problemTitle;
            if (metadata.accessPath) canvas.dataset.selectedAccessPath = metadata.accessPath;
        }
        updateInspector();
    }

    function handleWorkerMessage(event) {
        const message = event.data || {};
        switch (message.type) {
            case 'ready':
                if (message.nonblank !== true || message.pixelStatus !== 'nonblank') {
                    activateWorkerFallback();
                    break;
                }
                shell.classList.add('is-ready');
                canvas.dataset.sceneStatus = 'ready';
                canvas.dataset.sceneNonblank = 'true';
                canvas.dataset.frameCount = String(message.frameCount);
                break;
            case 'frame':
                canvas.dataset.frameCount = String(message.frameCount);
                break;
            case 'hover':
                hoveredMetadata = message.metadata;
                updateInspector();
                break;
            case 'selection':
                handleSelection(message.metadata, message.identity);
                break;
            case 'failure':
                activateWorkerFallback();
                break;
            default:
                break;
        }
    }

    function normalizedPointer(event) {
        const rect = canvas.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) return { x: 2, y: 2, inside: false };
        const x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        const y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
        return { x, y, inside: x >= -1 && x <= 1 && y >= -1 && y <= 1 };
    }

    function isInteractiveEvent(event) {
        return Boolean(event.target && event.target.closest && event.target.closest(INTERACTIVE_SELECTOR));
    }

    function suspendPointerInteraction() {
        pointerInside = false;
        post({ type: 'pointer', x: 2, y: 2, inside: false });
    }

    function inspectPointer(event) {
        if (isInteractiveEvent(event)) {
            suspendPointerInteraction();
            return;
        }
        const coordinates = normalizedPointer(event);
        pointerInside = coordinates.inside;
        post({ type: 'pointer', ...coordinates });
    }

    function selectPointer(event) {
        if (isInteractiveEvent(event)) return;
        const coordinates = normalizedPointer(event);
        pointerInside = coordinates.inside;
        post({ type: 'select', ...coordinates });
    }

    function resetPointer() {
        suspendPointerInteraction();
    }

    function resize() {
        post({
            type: 'resize',
            width: Math.max(1, shell.clientWidth),
            height: Math.max(1, shell.clientHeight),
            pixelRatio: window.devicePixelRatio || 1,
        });
    }

    function updateRendering() {
        post({ type: 'rendering', visible: heroVisible, hidden: document.hidden });
    }

    if (
        typeof Worker !== 'function' ||
        typeof canvas.transferControlToOffscreen !== 'function' ||
        !shell ||
        !hero
    ) {
        activateWorkerFallback();
        return;
    }

    try {
        worker = new Worker('/static/home-scene-worker.js', { type: 'module' });
        worker.addEventListener('message', handleWorkerMessage);
        worker.addEventListener('error', activateWorkerFallback);
        worker.addEventListener('messageerror', activateWorkerFallback);
        const offscreenCanvas = canvas.transferControlToOffscreen();
        const mobile = window.matchMedia(MOBILE_QUERY).matches;
        const reducedMotion = window.matchMedia(REDUCED_MOTION_QUERY).matches;
        canvas.dataset.sceneStatus = 'initializing';
        post(
            {
                type: 'init',
                canvas: offscreenCanvas,
                sourceNames: sourceElements.map((element) => element.dataset.source),
                width: Math.max(1, shell.clientWidth),
                height: Math.max(1, shell.clientHeight),
                pixelRatio: window.devicePixelRatio || 1,
                mobile,
                reducedMotion,
                visible: heroVisible,
                hidden: document.hidden,
            },
            [offscreenCanvas]
        );
    } catch (error) {
        activateWorkerFallback();
        return;
    }

    const intersectionObserver = new IntersectionObserver(
        ([entry]) => {
            heroVisible = entry.isIntersecting;
            updateRendering();
        },
        { threshold: 0.05 }
    );
    intersectionObserver.observe(hero);

    document.addEventListener('visibilitychange', updateRendering);
    document.addEventListener('languageChanged', refreshInspectorLanguage);
    refreshInspectorLanguage();
    hero.addEventListener('pointermove', inspectPointer, { passive: true });
    hero.addEventListener('pointerleave', resetPointer, { passive: true });
    hero.addEventListener('click', selectPointer);

    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(shell);
}

function updateInspectorText(inspector, metadata) {
    if (metadata.accessPath) {
        const accessPath = translate('home.hero.inspector_access_path', 'access path');
        const coreTo = translate('home.hero.inspector_core_to', 'core to');
        inspector.textContent = `${metadata.accessPath} ${accessPath} · ${coreTo} ${metadata.source}`;
        return;
    }
    const similarity = translate('home.hero.inspector_similarity', 'similarity');
    const illustrative = translate('home.hero.inspector_illustrative', 'illustrative');
    inspector.textContent = `${metadata.source} · ${metadata.problemTitle} · ${metadata.algorithm} · ${metadata.similarity}% ${similarity} (${illustrative})`;
}

function activateFallback(shell, targetCanvas) {
    if (shell) shell.classList.add('is-webgl-fallback');
    targetCanvas.dataset.sceneStatus = 'fallback';
    targetCanvas.dataset.sceneNonblank = 'false';
}
