import { initializeApp } from 'firebase/app';

// Web app config for the `persianseapickle` Firebase project. These values
// identify the project and are safe to ship to the browser; access is
// controlled by firestore.rules, not by keeping them secret.
// Source: firebase apps:sdkconfig web
const firebaseConfig = {
  apiKey: 'AIzaSyCgViOQF9T67kaUczzMmjYwV1xwT40pAnQ',
  authDomain: 'persianseapickle.firebaseapp.com',
  projectId: 'persianseapickle',
  appId: '1:351006130267:web:52ffcaa02e159010a83f7e',
};

export const app = initializeApp(firebaseConfig);
