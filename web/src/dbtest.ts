import { addDoc, collection, getDoc, getDocs, limit, orderBy, query, serverTimestamp } from 'firebase/firestore';
import { db } from './db';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

const status = $('status');
const form = $<HTMLFormElement>('form');
const input = $<HTMLInputElement>('text');
const button = $<HTMLButtonElement>('write');
const entries = $('entries');

const notes = collection(db, 'dbtest');

function report(state: 'ok' | 'fail' | 'busy', message: string): void {
  status.dataset.state = state;
  status.textContent = message;
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Read the newest notes from the server and list them. */
async function read(): Promise<number> {
  const snapshot = await getDocs(query(notes, orderBy('createdAt', 'desc'), limit(10)));
  entries.replaceChildren(
    ...snapshot.docs.map((doc) => {
      const item = document.createElement('li');
      const text = document.createElement('span');
      const time = document.createElement('time');
      text.textContent = doc.get('text');
      time.textContent = doc.get('createdAt')?.toDate().toLocaleString() ?? '';
      item.append(text, time);
      return item;
    }),
  );
  if (snapshot.empty) {
    const item = document.createElement('li');
    item.className = 'empty';
    item.textContent = 'Nothing yet.';
    entries.append(item);
  }
  return snapshot.size;
}

/** Write a note, then fetch that same document back from the server. */
async function write(text: string): Promise<void> {
  const ref = await addDoc(notes, { text, createdAt: serverTimestamp() });
  const saved = await getDoc(ref);
  if (saved.get('text') !== text) throw new Error('The note read back does not match what was written.');
  await read();
  report('ok', `Write OK · read back OK · document ${ref.id}`);
}

form.addEventListener('submit', async (event) => {
  event.preventDefault();
  const text = input.value.trim();
  if (!text) return;
  button.disabled = true;
  report('busy', 'Writing…');
  try {
    await write(text);
    input.value = '';
  } catch (error) {
    report('fail', `Write failed: ${describe(error)}`);
  } finally {
    button.disabled = false;
  }
});

try {
  const count = await read();
  report('ok', `Read OK · ${count} note${count === 1 ? '' : 's'} loaded`);
} catch (error) {
  report('fail', `Read failed: ${describe(error)}`);
}

// /dbtest.html?selftest writes a note without the form, for automated checks.
if (new URLSearchParams(location.search).has('selftest')) {
  try {
    await write(`selftest ${new Date().toISOString()}`);
  } catch (error) {
    report('fail', `Write failed: ${describe(error)}`);
  }
}
