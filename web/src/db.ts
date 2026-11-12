import { getFirestore } from 'firebase/firestore';
import { app } from './firebase';

export { app };
export const db = getFirestore(app);
