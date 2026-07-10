import * as THREE from '/static/vendor/three.module.min.js';

const MAX_DESKTOP_DPR = 1.5;
const MAX_MOBILE_DPR = 1.25;
const DESKTOP_NODE_COUNT = 110;
const MOBILE_NODE_COUNT = 48;
const MOBILE_QUERY = '(max-width: 760px)';
const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';
const TAU = Math.PI * 2;
const palette = [0x53e0e8, 0xff6a55, 0xc7f45b, 0xf3f1ea, 0x6fa7ff];
const algorithmFamilies = ['Graph', 'Dynamic programming', 'Geometry', 'Strings', 'Trees'];

const canvas = document.querySelector('[data-observatory-scene]');
const sourceElements = Array.from(document.querySelectorAll('.scene-source-data [data-source]'));

if (canvas && sourceElements.length > 0) {
    initializeScene();
}

function initializeScene() {
    const shell = canvas.closest('.scene-shell');
    const hero = canvas.closest('.observatory-hero');
    const inspector = document.querySelector('.scene-inspector span:last-child');
    const idleInspectorText = inspector ? inspector.textContent : '';
    const mobileMedia = window.matchMedia(MOBILE_QUERY);
    const reducedMotionMedia = window.matchMedia(REDUCED_MOTION_QUERY);
    const isMobile = mobileMedia.matches;
    const reducedMotion = reducedMotionMedia.matches;

    let renderer;
    try {
        renderer = new THREE.WebGLRenderer({
            canvas,
            alpha: true,
            antialias: !isMobile,
            powerPreference: 'high-performance',
        });
    } catch (error) {
        activateFallback(shell, canvas);
        return;
    }

    renderer.setClearColor(0x07090b, 0);
    renderer.outputColorSpace = THREE.SRGBColorSpace;

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(46, 1, 0.1, 80);
    const network = new THREE.Group();
    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2(2, 2);
    const pointerTarget = new THREE.Vector2(0, 0);
    const anchorMeshes = [];
    const sourceNames = sourceElements.map((element) => element.dataset.source);
    const nodeCount = isMobile ? MOBILE_NODE_COUNT : DESKTOP_NODE_COUNT;
    const nodesPerSource = Math.max(6, Math.floor(nodeCount / sourceNames.length));
    const random = seededRandom(42031);

    camera.position.set(0, 0, 12.5);
    scene.add(network);

    const coreMaterial = new THREE.MeshBasicMaterial({
        color: 0x53e0e8,
        transparent: true,
        opacity: 0.24,
        wireframe: true,
    });
    const core = new THREE.Mesh(new THREE.IcosahedronGeometry(1.18, 1), coreMaterial);
    network.add(core);

    const coreHalo = new THREE.Mesh(
        new THREE.TorusGeometry(1.72, 0.012, 4, 96),
        new THREE.MeshBasicMaterial({ color: 0xf3f1ea, transparent: true, opacity: 0.17 })
    );
    coreHalo.rotation.x = 1.12;
    coreHalo.rotation.z = 0.42;
    network.add(coreHalo);

    const pointPositions = [];
    const pointColors = [];
    const edgePositions = [];
    const anchorPositions = [];
    const color = new THREE.Color();

    sourceNames.forEach((source, index) => {
        const angle = (index / sourceNames.length) * TAU - Math.PI * 0.38;
        const anchor = new THREE.Vector3(
            Math.cos(angle) * (isMobile ? 3.25 : 4.25),
            Math.sin(angle) * (isMobile ? 2.35 : 3.05),
            Math.sin(angle * 1.7) * 1.15
        );
        anchorPositions.push(anchor);

        const anchorColor = palette[index % palette.length];
        const anchorMesh = new THREE.Mesh(
            new THREE.SphereGeometry(isMobile ? 0.14 : 0.18, 18, 18),
            new THREE.MeshBasicMaterial({ color: anchorColor })
        );
        anchorMesh.position.copy(anchor);
        anchorMesh.userData = {
            source,
            algorithm: algorithmFamilies[index % algorithmFamilies.length],
            similarity: 86 + ((index * 3) % 13),
        };
        anchorMeshes.push(anchorMesh);
        network.add(anchorMesh);

        const halo = new THREE.Mesh(
            new THREE.RingGeometry(isMobile ? 0.25 : 0.31, isMobile ? 0.27 : 0.33, 40),
            new THREE.MeshBasicMaterial({
                color: anchorColor,
                transparent: true,
                opacity: 0.35,
                side: THREE.DoubleSide,
            })
        );
        halo.position.copy(anchor);
        network.add(halo);

        color.setHex(anchorColor);
        for (let nodeIndex = 0; nodeIndex < nodesPerSource; nodeIndex += 1) {
            const spread = 0.5 + random() * (isMobile ? 1.25 : 1.7);
            const theta = random() * TAU;
            const phi = Math.acos(2 * random() - 1);
            const node = new THREE.Vector3(
                anchor.x + Math.sin(phi) * Math.cos(theta) * spread,
                anchor.y + Math.sin(phi) * Math.sin(theta) * spread * 0.72,
                anchor.z + Math.cos(phi) * spread * 0.86
            );
            pointPositions.push(node.x, node.y, node.z);
            pointColors.push(color.r, color.g, color.b);

            if (nodeIndex % (isMobile ? 3 : 2) === 0) {
                edgePositions.push(anchor.x, anchor.y, anchor.z, node.x, node.y, node.z);
            }
        }
    });

    for (let index = 0; index < anchorPositions.length; index += 1) {
        const current = anchorPositions[index];
        const next = anchorPositions[(index + 1) % anchorPositions.length];
        edgePositions.push(current.x, current.y, current.z, next.x, next.y, next.z);
        edgePositions.push(0, 0, 0, current.x, current.y, current.z);
    }

    const pointsGeometry = new THREE.BufferGeometry();
    pointsGeometry.setAttribute('position', new THREE.Float32BufferAttribute(pointPositions, 3));
    pointsGeometry.setAttribute('color', new THREE.Float32BufferAttribute(pointColors, 3));
    const points = new THREE.Points(
        pointsGeometry,
        new THREE.PointsMaterial({
            size: isMobile ? 0.055 : 0.065,
            sizeAttenuation: true,
            vertexColors: true,
            transparent: true,
            opacity: 0.88,
        })
    );
    network.add(points);

    const edgesGeometry = new THREE.BufferGeometry();
    edgesGeometry.setAttribute('position', new THREE.Float32BufferAttribute(edgePositions, 3));
    const edgesMaterial = new THREE.LineBasicMaterial({
        color: 0x53e0e8,
        transparent: true,
        opacity: isMobile ? 0.09 : 0.13,
    });
    const edges = new THREE.LineSegments(edgesGeometry, edgesMaterial);
    network.add(edges);

    const ambientPoints = createAmbientField(random, isMobile ? 34 : 76);
    scene.add(ambientPoints);

    let pixelRatio = Math.min(window.devicePixelRatio || 1, isMobile ? MAX_MOBILE_DPR : MAX_DESKTOP_DPR);
    let animationFrame = 0;
    let frameCount = 0;
    let lastFrameTime = 0;
    let frameSamples = [];
    let heroVisible = true;
    let sampledPixels = false;
    let hoveredAnchor = null;

    function resize() {
        const width = Math.max(1, shell.clientWidth);
        const height = Math.max(1, shell.clientHeight);
        renderer.setPixelRatio(pixelRatio);
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        network.position.x = width <= 760 ? 0 : 2.45;
        renderFrame(performance.now(), true);
    }

    function renderFrame(time, forceStatic = false) {
        const delta = lastFrameTime === 0 ? 16 : Math.min(50, time - lastFrameTime);
        lastFrameTime = time;

        if (!forceStatic) {
            frameSamples.push(delta);
            if (frameSamples.length >= 120) {
                const average = frameSamples.reduce((sum, sample) => sum + sample, 0) / frameSamples.length;
                frameSamples = [];
                if (average > 26 && pixelRatio > 0.85) {
                    pixelRatio = Math.max(0.85, pixelRatio - 0.2);
                    renderer.setPixelRatio(pixelRatio);
                    renderer.setSize(shell.clientWidth, shell.clientHeight, false);
                }
                if (average > 34) {
                    edgesMaterial.opacity = 0.055;
                }
            }
        }

        pointer.lerp(pointerTarget, reducedMotion ? 1 : 0.045);
        const phase = time * 0.00018;
        camera.position.x = pointer.x * 0.42 + Math.sin(phase * 0.7) * 0.18;
        camera.position.y = pointer.y * 0.3 + Math.cos(phase * 0.52) * 0.13;
        camera.position.z = 12.5 + Math.sin(phase * 0.41) * 0.22;
        camera.lookAt(network.position.x * 0.36, 0, 0);

        network.rotation.y = Math.sin(phase * 0.45) * 0.075;
        network.rotation.x = Math.cos(phase * 0.34) * 0.025;
        core.rotation.x = phase * 0.72;
        core.rotation.y = phase * 0.56;
        coreHalo.rotation.z = 0.42 - phase * 0.18;
        ambientPoints.rotation.y = phase * 0.06;

        renderer.render(scene, camera);
        frameCount += 1;
        canvas.dataset.frameCount = String(frameCount);

        if (!sampledPixels) {
            sampledPixels = true;
            sampleCanvasPixels(renderer, canvas);
        }
    }

    function tick(time) {
        animationFrame = 0;
        if (!heroVisible || document.hidden || reducedMotion) return;
        renderFrame(time);
        animationFrame = window.requestAnimationFrame(tick);
    }

    function start() {
        if (animationFrame || reducedMotion || !heroVisible || document.hidden) return;
        lastFrameTime = 0;
        animationFrame = window.requestAnimationFrame(tick);
    }

    function stop() {
        if (!animationFrame) return;
        window.cancelAnimationFrame(animationFrame);
        animationFrame = 0;
    }

    function inspectPointer(event) {
        const rect = canvas.getBoundingClientRect();
        const x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        const y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
        pointerTarget.set(x * 0.72, y * 0.56);
        if (reducedMotion) return;

        raycaster.setFromCamera(new THREE.Vector2(x, y), camera);
        const intersection = raycaster.intersectObjects(anchorMeshes, false)[0];
        const nextAnchor = intersection ? intersection.object : null;
        if (nextAnchor === hoveredAnchor) return;

        if (hoveredAnchor) {
            hoveredAnchor.scale.setScalar(1);
        }
        hoveredAnchor = nextAnchor;
        if (hoveredAnchor) {
            hoveredAnchor.scale.setScalar(1.65);
            updateInspector(inspector, hoveredAnchor.userData);
        } else if (inspector) {
            inspector.textContent = idleInspectorText;
        }
    }

    function resetPointer() {
        pointerTarget.set(0, 0);
        if (hoveredAnchor) hoveredAnchor.scale.setScalar(1);
        hoveredAnchor = null;
        if (inspector) inspector.textContent = idleInspectorText;
    }

    const intersectionObserver = new IntersectionObserver(
        ([entry]) => {
            heroVisible = entry.isIntersecting;
            if (heroVisible) start();
            else stop();
        },
        { threshold: 0.05 }
    );
    intersectionObserver.observe(hero);

    document.addEventListener('visibilitychange', () => {
        if (document.hidden) stop();
        else start();
    });
    hero.addEventListener('pointermove', inspectPointer, { passive: true });
    hero.addEventListener('pointerleave', resetPointer, { passive: true });

    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(shell);
    resize();
    shell.classList.add('is-ready');
    canvas.dataset.sceneStatus = 'ready';

    if (reducedMotion) {
        renderFrame(0, true);
    } else {
        start();
    }
}

function createAmbientField(random, count) {
    const positions = [];
    for (let index = 0; index < count; index += 1) {
        positions.push((random() - 0.5) * 21, (random() - 0.5) * 13, (random() - 0.5) * 9 - 2);
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    return new THREE.Points(
        geometry,
        new THREE.PointsMaterial({
            color: 0xf3f1ea,
            size: 0.026,
            transparent: true,
            opacity: 0.28,
        })
    );
}

function updateInspector(inspector, metadata) {
    if (!inspector) return;
    inspector.textContent = `${metadata.source} · ${metadata.algorithm} · ${metadata.similarity}% semantic`;
}

function sampleCanvasPixels(renderer, targetCanvas) {
    try {
        const context = renderer.getContext();
        const width = context.drawingBufferWidth;
        const height = context.drawingBufferHeight;
        const pixels = new Uint8Array(width * height * 4);
        context.readPixels(0, 0, width, height, context.RGBA, context.UNSIGNED_BYTE, pixels);
        let coloredSamples = 0;
        const stride = Math.max(4, Math.floor(pixels.length / 12000 / 4) * 4);
        for (let index = 0; index < pixels.length; index += stride) {
            if (pixels[index] + pixels[index + 1] + pixels[index + 2] > 70) {
                coloredSamples += 1;
            }
        }
        targetCanvas.dataset.sceneNonblank = coloredSamples > 12 ? 'true' : 'false';
    } catch (error) {
        targetCanvas.dataset.sceneNonblank = 'unknown';
    }
}

function activateFallback(shell, targetCanvas) {
    shell.classList.add('is-webgl-fallback');
    targetCanvas.dataset.sceneStatus = 'fallback';
    targetCanvas.dataset.sceneNonblank = 'false';
}

function seededRandom(seed) {
    let value = seed >>> 0;
    return () => {
        value += 0x6d2b79f5;
        let next = value;
        next = Math.imul(next ^ (next >>> 15), next | 1);
        next ^= next + Math.imul(next ^ (next >>> 7), next | 61);
        return ((next ^ (next >>> 14)) >>> 0) / 4294967296;
    };
}
