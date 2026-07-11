import * as THREE from '/static/vendor/three.home.min.js';

const MAX_DESKTOP_DPR = 1.5;
const MAX_MOBILE_DPR = 1.25;
const DESKTOP_NODE_COUNT = 110;
const MOBILE_NODE_COUNT = 48;
const MAX_CONTROLLED_PULSES = 4;
const INTERACTION_LAYER = 1;
const FRAME_REPORT_INTERVAL_MS = 250;
const TAU = Math.PI * 2;
const palette = [0x53e0e8, 0xff6a55, 0xc7f45b, 0xf3f1ea, 0x6fa7ff];
const algorithmFamilies = ['Graph', 'Dynamic programming', 'Geometry', 'Strings', 'Trees'];
const illustrativeProblemTitles = [
    'Shortest Path Relay',
    'State Compression Workshop',
    'Convex Hull Signal',
    'Pattern Match Index',
    'Balanced Tree Queries',
];
const accessPathLabels = ['resolution', 'retrieval', 'MCP', 'API'];

let sceneRuntime = null;
let failurePosted = false;

function postFailureOnce(reason) {
    if (failurePosted) return;
    failurePosted = true;
    postMessage({ type: 'failure', reason });
}

self.addEventListener('message', (event) => {
    const message = event.data || {};
    try {
        switch (message.type) {
            case 'init':
                failurePosted = false;
                if (sceneRuntime) sceneRuntime.dispose();
                sceneRuntime = createSceneRuntime(message);
                break;
            case 'resize':
                if (sceneRuntime) sceneRuntime.resize(message.width, message.height, message.pixelRatio);
                break;
            case 'pointer':
                if (sceneRuntime) sceneRuntime.updatePointer(message.x, message.y, message.inside);
                break;
            case 'select':
                if (sceneRuntime) sceneRuntime.selectPointer(message.x, message.y, message.inside);
                break;
            case 'rendering':
                if (sceneRuntime) sceneRuntime.updateRendering(message.visible, message.hidden);
                break;
            default:
                break;
        }
    } catch (error) {
        postFailureOnce(failureReason(error));
        if (sceneRuntime) sceneRuntime.dispose();
        sceneRuntime = null;
    }
});

function createSceneRuntime(options) {
    const {
        canvas,
        sourceNames,
        mobile: isMobile,
        reducedMotion,
    } = options;
    if (!canvas || !Array.isArray(sourceNames) || sourceNames.length === 0) {
        throw new Error('invalid scene initialization');
    }

    const renderer = new THREE.WebGLRenderer({
        canvas,
        alpha: true,
        antialias: !isMobile,
        powerPreference: 'high-performance',
    });
    renderer.setClearColor(0x07090b, 0);
    renderer.outputColorSpace = THREE.SRGBColorSpace;

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(46, 1, 0.1, 80);
    const network = new THREE.Group();
    const raycaster = new THREE.Raycaster();
    raycaster.params.Points.threshold = isMobile ? 0.16 : 0.12;
    raycaster.layers.enable(INTERACTION_LAYER);
    const pointer = new THREE.Vector2(2, 2);
    const pointerTarget = new THREE.Vector2(0, 0);
    const lastPointerCoordinates = new THREE.Vector2(2, 2);
    const intersections = [];
    const anchorMeshes = [];
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
    const problemMetadata = [];
    const problemInteractionTargets = [];
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
        const anchorMetadata = {
            source,
            problemTitle: illustrativeProblemTitles[index % illustrativeProblemTitles.length],
            algorithm: algorithmFamilies[index % algorithmFamilies.length],
            similarity: 86 + ((index * 3) % 13),
        };
        anchorMesh.userData.interactionTarget = {
            identity: `anchor-${source}`,
            metadata: anchorMetadata,
            sourceAnchor: anchorMesh,
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
            const metadata = {
                source,
                problemTitle: `${illustrativeProblemTitles[index % illustrativeProblemTitles.length]} ${nodeIndex + 1}`,
                algorithm: algorithmFamilies[index % algorithmFamilies.length],
                similarity: 72 + ((index * 7 + nodeIndex * 5) % 27),
            };
            problemMetadata.push(metadata);
            problemInteractionTargets.push({
                identity: `problem-${problemMetadata.length - 1}`,
                metadata,
                sourceAnchor: anchorMesh,
            });

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

    const controlledPulses = createControlledPulses(anchorPositions, anchorMeshes, isMobile);
    controlledPulses.forEach((pulse) => network.add(pulse));
    const raycastTargets = [
        ...anchorMeshes,
        points,
        ...controlledPulses.map((pulse) => pulse.userData.pulseHitTarget),
    ];

    const ambientPoints = createAmbientField(random, isMobile ? 34 : 76);
    scene.add(ambientPoints);

    let width = Math.max(1, options.width);
    let height = Math.max(1, options.height);
    let pixelRatio = cappedPixelRatio(options.pixelRatio, isMobile);
    let animationFrame = 0;
    let frameCount = 0;
    let lastFrameReportTime = Number.NEGATIVE_INFINITY;
    let lastFrameTime = 0;
    let frameSamples = [];
    let heroVisible = Boolean(options.visible);
    let documentHidden = Boolean(options.hidden);
    let ready = false;
    let hoveredTarget = null;
    let selectedTarget = null;
    let pointerInside = false;
    let disposed = false;

    function handleContextLost(event) {
        event.preventDefault();
        stop();
        postFailureOnce('webgl context lost');
    }
    canvas.addEventListener('webglcontextlost', handleContextLost);

    function applySize(nextWidth, nextHeight, requestedPixelRatio) {
        width = Math.max(1, Number(nextWidth) || 1);
        height = Math.max(1, Number(nextHeight) || 1);
        if (requestedPixelRatio !== undefined) {
            pixelRatio = Math.min(pixelRatio, cappedPixelRatio(requestedPixelRatio, isMobile));
        }
        renderer.setPixelRatio(pixelRatio);
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        network.position.x = width <= 760 ? 0 : 2.45;
    }

    function renderFrame(time, forceStatic = false) {
        if (disposed) return;
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
                    renderer.setSize(width, height, false);
                }
                if (average > 34) edgesMaterial.opacity = 0.055;
            }
        }

        pointer.lerp(pointerTarget, reducedMotion ? 1 : 0.045);
        const motionTime = reducedMotion ? 0 : time;
        const phase = motionTime * 0.00018;
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
        updateControlledPulses(controlledPulses, motionTime);

        renderer.render(scene, camera);
        if (!forceStatic && pointerInside) refreshHoveredTarget();
        frameCount += 1;
        reportFrame(time, frameCount === 1);

        if (!ready) {
            const pixelStatus = sampleCanvasPixels(renderer);
            if (pixelStatus !== 'nonblank') {
                throw new Error(`scene pixel sample ${pixelStatus}`);
            }
            ready = true;
            postMessage({ type: 'ready', nonblank: true, pixelStatus, frameCount });
        }
    }

    function reportFrame(time, force = false) {
        if (!force && time - lastFrameReportTime < FRAME_REPORT_INTERVAL_MS) return;
        lastFrameReportTime = time;
        postMessage({ type: 'frame', frameCount });
    }

    function tick(time) {
        animationFrame = 0;
        if (disposed || !heroVisible || documentHidden || reducedMotion) return;
        renderFrame(time);
        animationFrame = requestAnimationFrame(tick);
    }

    function start() {
        if (disposed || animationFrame || reducedMotion || !heroVisible || documentHidden) return;
        lastFrameTime = 0;
        animationFrame = requestAnimationFrame(tick);
    }

    function stop() {
        if (!animationFrame) return;
        cancelAnimationFrame(animationFrame);
        animationFrame = 0;
    }

    function raycastTarget() {
        if (!pointerInside) return null;
        raycaster.setFromCamera(lastPointerCoordinates, camera);
        intersections.length = 0;
        raycaster.intersectObjects(raycastTargets, false, intersections);
        const intersection = intersections[0];
        if (!intersection) return null;
        if (intersection.object === points) {
            return problemMetadata[intersection.index]
                ? problemInteractionTargets[intersection.index]
                : null;
        }
        return intersection.object.userData.interactionTarget || null;
    }

    function targetsMatch(left, right) {
        return left === right || (left && right && left.identity === right.identity);
    }

    function updateAnchorPresentation() {
        if (reducedMotion) return;
        const selectedAnchor = selectedTarget ? selectedTarget.sourceAnchor : null;
        const hoveredAnchor = hoveredTarget ? hoveredTarget.sourceAnchor : null;
        anchorMeshes.forEach((anchor) => {
            const scale = anchor === selectedAnchor ? 1.9 : anchor === hoveredAnchor ? 1.65 : 1;
            anchor.scale.setScalar(scale);
        });
    }

    function refreshHoveredTarget() {
        const nextTarget = raycastTarget();
        if (targetsMatch(nextTarget, hoveredTarget)) return;
        hoveredTarget = nextTarget;
        updateAnchorPresentation();
        postMessage({ type: 'hover', metadata: cloneSafeMetadata(hoveredTarget) });
    }

    function updatePointer(x, y, inside) {
        pointerInside = Boolean(inside);
        lastPointerCoordinates.set(Number(x) || 0, Number(y) || 0);
        if (!reducedMotion) updatePointerTarget();
        refreshHoveredTarget();
    }

    function updatePointerTarget() {
        if (pointerInside) pointerTarget.set(lastPointerCoordinates.x * 0.72, lastPointerCoordinates.y * 0.56);
        else pointerTarget.set(0, 0);
    }

    function selectPointer(x, y, inside) {
        pointerInside = Boolean(inside);
        lastPointerCoordinates.set(Number(x) || 0, Number(y) || 0);
        selectedTarget = raycastTarget();
        hoveredTarget = selectedTarget;
        updateAnchorPresentation();
        postMessage({
            type: 'selection',
            metadata: cloneSafeMetadata(selectedTarget),
            identity: selectedTarget ? selectedTarget.identity : null,
        });
        postMessage({ type: 'hover', metadata: cloneSafeMetadata(hoveredTarget) });
    }

    function resize(nextWidth, nextHeight, requestedPixelRatio) {
        applySize(nextWidth, nextHeight, requestedPixelRatio);
        renderFrame(performance.now(), true);
    }

    function updateRendering(visible, hidden) {
        heroVisible = Boolean(visible);
        documentHidden = Boolean(hidden);
        if (heroVisible && !documentHidden) start();
        else stop();
    }

    function dispose() {
        if (disposed) return;
        disposed = true;
        stop();
        canvas.removeEventListener('webglcontextlost', handleContextLost);
        renderer.dispose();
    }

    applySize(width, height, options.pixelRatio);
    renderFrame(0, true);
    if (!reducedMotion) start();

    return { dispose, resize, selectPointer, updatePointer, updateRendering };
}

function createControlledPulses(anchorPositions, anchorMeshes, isMobile) {
    return anchorPositions.slice(0, MAX_CONTROLLED_PULSES).map((anchor, index) => {
        const pulse = new THREE.Group();
        const visualPulse = new THREE.Mesh(
            new THREE.SphereGeometry(isMobile ? 0.045 : 0.06, 10, 10),
            new THREE.MeshBasicMaterial({
                color: palette[index % palette.length],
                transparent: true,
                opacity: 0.9,
            })
        );
        const pulseHitTarget = new THREE.Mesh(
            new THREE.SphereGeometry(isMobile ? 0.28 : 0.2, 10, 10),
            new THREE.MeshBasicMaterial({
                transparent: true,
                opacity: 0,
                colorWrite: false,
                depthWrite: false,
            })
        );
        pulseHitTarget.layers.set(INTERACTION_LAYER);
        const source = anchorMeshes[index].userData.interactionTarget.metadata.source;
        const metadata = { accessPath: accessPathLabels[index], source };
        pulseHitTarget.userData.interactionTarget = {
            identity: `access-${metadata.accessPath}-${source}`,
            metadata,
            sourceAnchor: anchorMeshes[index],
        };
        pulse.add(visualPulse, pulseHitTarget);
        pulse.userData = {
            visualPulse,
            pulseHitTarget,
            destination: anchor.clone(),
            phase: index / MAX_CONTROLLED_PULSES,
            speed: 0.00011 + index * 0.000008,
        };
        return pulse;
    });
}

function updateControlledPulses(pulses, time) {
    pulses.forEach((pulse) => {
        const progress = (pulse.userData.phase + time * pulse.userData.speed) % 1;
        const easedProgress = progress * progress * (3 - 2 * progress);
        pulse.position.copy(pulse.userData.destination).multiplyScalar(easedProgress);
        pulse.userData.visualPulse.material.opacity = 0.32 + Math.sin(progress * Math.PI) * 0.68;
    });
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

function cloneSafeMetadata(target) {
    if (!target || !target.metadata) return null;
    const { source, problemTitle, algorithm, similarity, accessPath } = target.metadata;
    return { source, problemTitle, algorithm, similarity, accessPath };
}

function sampleCanvasPixels(renderer) {
    try {
        const context = renderer.getContext();
        const width = context.drawingBufferWidth;
        const height = context.drawingBufferHeight;
        const pixels = new Uint8Array(width * height * 4);
        context.readPixels(0, 0, width, height, context.RGBA, context.UNSIGNED_BYTE, pixels);
        let coloredSamples = 0;
        const stride = Math.max(4, Math.floor(pixels.length / 12000 / 4) * 4);
        for (let index = 0; index < pixels.length; index += stride) {
            if (pixels[index] + pixels[index + 1] + pixels[index + 2] > 70) coloredSamples += 1;
        }
        if (coloredSamples > 12) return 'nonblank';
        return 'blank';
    } catch (error) {
        return 'unknown';
    }
}

function cappedPixelRatio(pixelRatio, isMobile) {
    const cap = isMobile ? MAX_MOBILE_DPR : MAX_DESKTOP_DPR;
    return Math.min(Number(pixelRatio) || 1, cap);
}

function failureReason(error) {
    return error instanceof Error ? error.message : 'scene worker failure';
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
