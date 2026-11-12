// Anonymous click tracking for the heatmap at /heatmap.html.
//
// Clicks are buffered and written to the `clicks` collection in batches, one
// document per batch, to keep Firestore writes (and cost) low. Nothing here
// identifies a visitor: the session id is random and lives only as long as the
// page. Uses the REST-based "lite" Firestore SDK so the main page does not
// pay for the full realtime client.
import { addDoc, collection, getFirestore, serverTimestamp } from 'firebase/firestore/lite';
import { app } from './firebase';

export interface Click {
  /** Position as a fraction of the viewport, 0–1 from the top-left. */
  x: number;
  y: number;
  /** Milliseconds since the page loaded. */
  t: number;
  /** What was clicked: the nearest element id, or a tag name. */
  el: string;
  /** 'mouse', 'touch' or 'pen'. */
  kind: string;
  /** Clicked while the pointer was locked for looking around, so the position
   *  is where the cursor was captured, not where the visitor aimed. */
  locked: boolean;
}

export interface ClickBatch {
  session: string;
  page: string;
  /** Viewport size in CSS pixels when the batch was sent. */
  vw: number;
  vh: number;
  clicks: Click[];
}

/** Largest batch the security rules accept. */
const MAX_BATCH = 50;
const FLUSH_AFTER_MS = 5000;

const batches = collection(getFirestore(app), 'clicks');
const session = crypto.randomUUID();
const queue: Click[] = [];
let timer: number | undefined;

function flush(): void {
  clearTimeout(timer);
  timer = undefined;
  while (queue.length) {
    const batch: ClickBatch = {
      session,
      page: location.pathname,
      vw: innerWidth,
      vh: innerHeight,
      clicks: queue.splice(0, MAX_BATCH),
    };
    // Tracking must never disturb the visit: a failed write is dropped.
    addDoc(batches, { ...batch, createdAt: serverTimestamp() }).catch(() => {});
  }
}

function label(target: EventTarget | null): string {
  if (!(target instanceof Element)) return '';
  const named = target.closest('[id]');
  return (named ? `#${named.id}` : target.tagName.toLowerCase()).slice(0, 40);
}

const round = (value: number) => Math.round(value * 10000) / 10000;

export function trackClicks(): void {
  document.addEventListener(
    'pointerdown',
    (event) => {
      queue.push({
        x: round(event.clientX / innerWidth),
        y: round(event.clientY / innerHeight),
        t: Math.round(performance.now()),
        el: label(event.target),
        kind: event.pointerType || 'mouse',
        locked: document.pointerLockElement !== null,
      });
      if (queue.length >= MAX_BATCH) flush();
      else timer ??= window.setTimeout(flush, FLUSH_AFTER_MS);
    },
    { capture: true, passive: true },
  );
  // Send what is left when the visitor switches away or closes the tab.
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') flush();
  });
}
