import './style.css';
import init, { clear_moves, interact, look, reset_view, run, set_active, set_hurry, set_move } from './wasm/game.js';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

const canvas = $<HTMLCanvasElement>('house-canvas');
const enterButton = $<HTMLButtonElement>('enter');
const helpButton = $<HTMLButtonElement>('help');
const helpPanel = $('help-panel');
const interaction = $('interaction');
const toastBox = $('toast');

const house = $('house');
const coarse = matchMedia('(pointer: coarse)').matches;

/** Most rows of pixels the hall renders at, before scaling up to the window. */
const MAX_ROWS = 900;

// Cap the canvas resolution and scale it up smoothly: large and high-density
// screens would otherwise multiply the per-pixel lighting cost. The canvas
// backing store is its container's size in device pixels, measured before the
// transform, so the container is sized in device pixels / ratio.
function fitPixels(): void {
  const ratio = devicePixelRatio || 1;
  const rows = Math.min(MAX_ROWS, Math.round(innerHeight * ratio));
  const columns = Math.round((innerWidth * rows) / innerHeight);
  house.style.width = `${columns / ratio}px`;
  house.style.height = `${rows / ratio}px`;
  house.style.transform = `scale(${(innerWidth * ratio) / columns}, ${(innerHeight * ratio) / rows})`;
}
fitPixels();
addEventListener('resize', fitPixels);
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

let active = false;
let started = false;
let toastTimer: number | undefined;

function showError(): void {
  $('error').hidden = false;
}

function toast(message: string): void {
  toastBox.textContent = message;
  toastBox.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toastBox.classList.remove('show'), 4500);
}

function showTarget(label: string | null): void {
  interaction.textContent = label ?? '';
  interaction.style.display = label && active ? 'block' : 'none';
}

function setActive(value: boolean): void {
  active = value;
  set_active(value);
  document.body.classList.toggle('exploring', value);
  enterButton.textContent = started ? 'Continue exploring →' : 'Step inside →';
  $('controls-label').textContent = value
    ? 'W A S D to walk · E to interact · Esc to pause'
    : 'An unhurried place to explore';
  if (!value) interaction.style.display = 'none';
}

function closeHelp(): void {
  helpPanel.hidden = true;
  helpButton.setAttribute('aria-expanded', 'false');
}

function dragFallback(): void {
  setActive(true);
  toast('Drag to look around. Use W A S D to walk.');
}

async function enter(): Promise<void> {
  started = true;
  closeHelp();
  if (coarse) {
    setActive(true);
    return;
  }
  try {
    await canvas.requestPointerLock();
  } catch {
    dragFallback();
  }
}

function bindControls(): void {
  enterButton.addEventListener('click', enter);
  document.addEventListener('pointerlockchange', () => setActive(document.pointerLockElement === canvas));
  document.addEventListener('pointerlockerror', dragFallback);

  // Without pointer lock (touch, or when it's refused), drag to look and tap to use.
  let drag: { x: number; y: number; moved: boolean } | null = null;
  canvas.style.touchAction = 'none';
  canvas.addEventListener('pointerdown', (event) => {
    if (!active) return;
    if (document.pointerLockElement) {
      interact();
      return;
    }
    drag = { x: event.clientX, y: event.clientY, moved: false };
    canvas.setPointerCapture(event.pointerId);
  });
  canvas.addEventListener('pointermove', (event) => {
    if (!active || document.pointerLockElement || !drag) return;
    const dx = event.clientX - drag.x;
    const dy = event.clientY - drag.y;
    look(-dx * 0.004, -dy * 0.003);
    drag.moved ||= Math.abs(dx) + Math.abs(dy) > 2;
    drag.x = event.clientX;
    drag.y = event.clientY;
  });
  canvas.addEventListener('pointerup', () => {
    if (drag && !drag.moved) interact();
    drag = null;
  });
  document.addEventListener('mousemove', (event) => {
    if (active && document.pointerLockElement === canvas) look(-event.movementX * 0.002, -event.movementY * 0.002);
  });

  const moveCodes: Record<string, string> = {
    KeyW: 'forward', ArrowUp: 'forward',
    KeyS: 'backward', ArrowDown: 'backward',
    KeyA: 'left', ArrowLeft: 'left',
    KeyD: 'right', ArrowRight: 'right',
  };
  document.addEventListener('keydown', (event) => {
    if (event.code === 'Escape') {
      setActive(false);
      return;
    }
    if (event.code === 'ShiftLeft' || event.code === 'ShiftRight') set_hurry(true);
    if (!active) return;
    const direction = moveCodes[event.code];
    if (direction) {
      set_move(direction, true);
      event.preventDefault();
    }
    if (event.code === 'KeyE' && !event.repeat) interact();
  });
  document.addEventListener('keyup', (event) => {
    if (event.code === 'ShiftLeft' || event.code === 'ShiftRight') set_hurry(false);
    const direction = moveCodes[event.code];
    if (direction) set_move(direction, false);
  });
  window.addEventListener('blur', () => {
    clear_moves();
    if (active) {
      document.exitPointerLock?.();
      setActive(false);
    }
  });

  document.querySelectorAll<HTMLButtonElement>('[data-move]').forEach((button) => {
    const direction = button.dataset.move!;
    button.addEventListener('pointerdown', (event) => {
      event.preventDefault();
      button.setPointerCapture(event.pointerId);
      set_move(direction, true);
    });
    for (const type of ['pointerup', 'pointercancel', 'lostpointercapture']) {
      button.addEventListener(type, () => set_move(direction, false));
    }
  });
  $('touch-use').addEventListener('click', () => interact());

  helpButton.addEventListener('click', () => {
    if (document.pointerLockElement) document.exitPointerLock();
    setActive(false);
    helpPanel.hidden = !helpPanel.hidden;
    helpButton.setAttribute('aria-expanded', String(!helpPanel.hidden));
  });
  $('reset').addEventListener('click', () => {
    reset_view();
    closeHelp();
    toast('Back at the entrance.');
  });
  $('sound').addEventListener('click', toggleSound);
}

// Ambient wind: looping brown noise through a low-pass filter whose cutoff
// drifts slowly, following the storm's gusting character.
let audio: AudioContext | undefined;
let windGain: GainNode | undefined;
let soundOn = false;
const WIND_VOLUME = 0.12;

// Footsteps: recorded wood and carpet steps (Kenney, CC0), a few takes of
// each picked at random and never twice in a row, with slight pitch and
// level variation so the walk doesn't sound mechanical.
type Surface = 'wood' | 'carpet';
const STEP_TAKES = 5;
const STEP_VOLUME: Record<Surface, number> = { wood: 0.35, carpet: 0.45 };
const steps: Partial<Record<Surface, AudioBuffer[]>> = {};
const lastTake: Record<Surface, number> = { wood: -1, carpet: -1 };

async function loadSteps(context: AudioContext): Promise<void> {
  await Promise.all((['wood', 'carpet'] as const).map(async (surface) => {
    steps[surface] = await Promise.all(Array.from({ length: STEP_TAKES }, async (_, take) => {
      const response = await fetch(`/sounds/${surface}-${take}.wav`);
      return context.decodeAudioData(await response.arrayBuffer());
    }));
  }));
}

function footstep(loudness: number, onRug: boolean): void {
  const surface: Surface = onRug ? 'carpet' : 'wood';
  const takes = steps[surface];
  if (!audio || !soundOn || !takes) return;
  let take = Math.floor(Math.random() * (takes.length - 1));
  if (take >= lastTake[surface]) take += 1;
  lastTake[surface] = take;
  const vary = (amount: number) => 1 + (Math.random() * 2 - 1) * amount;
  const source = audio.createBufferSource();
  source.buffer = takes[take];
  source.playbackRate.value = vary(0.06);
  // Take the edge off the highs for a softer, indoor step.
  const tone = audio.createBiquadFilter();
  tone.type = 'lowpass';
  tone.frequency.value = onRug ? 2200 : 4200;
  const gain = audio.createGain();
  gain.gain.value = STEP_VOLUME[surface] * loudness * vary(0.12);
  source.connect(tone).connect(gain).connect(audio.destination);
  source.start();
}

function startWind(context: AudioContext): GainNode {
  const buffer = context.createBuffer(1, context.sampleRate * 3, context.sampleRate);
  const data = buffer.getChannelData(0);
  let last = 0;
  for (let i = 0; i < data.length; i++) {
    last = (last + Math.random() * 0.04 - 0.02) / 1.02;
    data[i] = last * 4;
  }
  const noise = context.createBufferSource();
  noise.buffer = buffer;
  noise.loop = true;
  const filter = context.createBiquadFilter();
  filter.type = 'lowpass';
  filter.frequency.value = 520;
  const gain = context.createGain();
  gain.gain.value = 0;
  noise.connect(filter).connect(gain).connect(context.destination);
  noise.start();
  const gust = context.createOscillator();
  gust.frequency.value = 0.12;
  const gustDepth = context.createGain();
  gustDepth.gain.value = 170;
  gust.connect(gustDepth).connect(filter.frequency);
  gust.start();
  return gain;
}

async function toggleSound(): Promise<void> {
  const button = $('sound');
  try {
    audio ??= new AudioContext();
    if (!windGain) {
      windGain = startWind(audio);
      loadSteps(audio).catch((error: unknown) => console.warn('Footstep sounds failed to load', error));
    }
    await audio.resume();
    soundOn = !soundOn;
    windGain.gain.setTargetAtTime(soundOn ? WIND_VOLUME : 0, audio.currentTime, 0.3);
    button.innerHTML = `Sound ${soundOn ? 'on' : 'off'} <span>♫</span>`;
    button.setAttribute('aria-pressed', String(soundOn));
  } catch {
    toast('Ambient sound isn’t available in this browser.');
  }
}

function ready(): void {
  enterButton.disabled = false;
  enterButton.textContent = 'Step inside →';
}

async function start(): Promise<void> {
  if (!document.createElement('canvas').getContext('webgl2')) {
    showError();
    return;
  }
  bindControls();
  await init();
  try {
    run(ready, showTarget, toast, footstep, showError, reducedMotion);
  } catch (error) {
    // Bevy throws to hand control to the browser's event loop; that is not a failure.
    if (error instanceof Error && error.message.includes('Using exceptions for control flow')) return;
    throw error;
  }
}

start().catch((error: unknown) => {
  console.error(error);
  showError();
});
