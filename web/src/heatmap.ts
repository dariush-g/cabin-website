import { collection, getDocs, getFirestore, limit, orderBy, query } from 'firebase/firestore/lite';
import { app } from './firebase';
import type { Click, ClickBatch } from './track';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

const canvas = $<HTMLCanvasElement>('heat');
const status = $('status');
const kindSelect = $<HTMLSelectElement>('kind');
const lockedBox = $<HTMLInputElement>('locked');

/** Newest batches to load: up to 50 clicks each. */
const MAX_BATCHES = 500;

let clicks: Click[] = [];
let sessions = 0;

// Cold-to-hot ramp, looked up by accumulated intensity (0–255).
const ramp = (() => {
  const strip = document.createElement('canvas');
  strip.width = 256;
  strip.height = 1;
  const context = strip.getContext('2d')!;
  const gradient = context.createLinearGradient(0, 0, 256, 0);
  gradient.addColorStop(0.0, '#2b4a6b');
  gradient.addColorStop(0.35, '#4f8f8a');
  gradient.addColorStop(0.6, '#d9c27a');
  gradient.addColorStop(0.85, '#e08a4a');
  gradient.addColorStop(1.0, '#f4e9d2');
  context.fillStyle = gradient;
  context.fillRect(0, 0, 256, 1);
  return context.getImageData(0, 0, 256, 1).data;
})();

function draw(): void {
  const width = (canvas.width = innerWidth);
  const height = (canvas.height = innerHeight);
  const context = canvas.getContext('2d', { willReadFrequently: true })!;
  const shown = clicks.filter(
    (click) => (kindSelect.value === 'all' || click.kind === kindSelect.value) && (lockedBox.checked || !click.locked),
  );

  // Accumulate soft blobs in the alpha channel, then colour by density.
  const radius = Math.max(18, Math.min(width, height) * 0.035);
  // Fewer points get stronger blobs so a sparse map is still readable.
  const strength = Math.min(0.5, Math.max(0.06, 6 / Math.sqrt(shown.length || 1)));
  for (const click of shown) {
    const x = click.x * width;
    const y = click.y * height;
    const blob = context.createRadialGradient(x, y, 0, x, y, radius);
    blob.addColorStop(0, `rgba(0,0,0,${strength})`);
    blob.addColorStop(1, 'rgba(0,0,0,0)');
    context.fillStyle = blob;
    context.fillRect(x - radius, y - radius, radius * 2, radius * 2);
  }
  const image = context.getImageData(0, 0, width, height);
  const pixels = image.data;
  for (let i = 0; i < pixels.length; i += 4) {
    const heat = pixels[i + 3];
    if (!heat) continue;
    pixels[i] = ramp[heat * 4];
    pixels[i + 1] = ramp[heat * 4 + 1];
    pixels[i + 2] = ramp[heat * 4 + 2];
    pixels[i + 3] = Math.min(230, 60 + heat * 1.6);
  }
  context.putImageData(image, 0, 0);

  const hidden = clicks.length - shown.length;
  status.textContent =
    `${shown.length} click${shown.length === 1 ? '' : 's'} · ${sessions} visit${sessions === 1 ? '' : 's'}` +
    (hidden ? ` · ${hidden} filtered out` : '');
}

async function load(): Promise<void> {
  const snapshot = await getDocs(
    query(collection(getFirestore(app), 'clicks'), orderBy('createdAt', 'desc'), limit(MAX_BATCHES)),
  );
  const batches = snapshot.docs.map((doc) => doc.data() as ClickBatch);
  clicks = batches.flatMap((batch) => batch.clicks ?? []);
  sessions = new Set(batches.map((batch) => batch.session)).size;
}

kindSelect.addEventListener('change', draw);
lockedBox.addEventListener('change', draw);
addEventListener('resize', draw);

try {
  await load();
  draw();
} catch (error) {
  status.textContent = `Couldn’t load clicks: ${error instanceof Error ? error.message : String(error)}`;
}
