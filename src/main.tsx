import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './fonts.css';
import './style.css';
import App from './App';
import { TaskSelectionProvider } from './components/TaskSelection';
createRoot(document.getElementById('root')!).render(<StrictMode><TaskSelectionProvider><App /></TaskSelectionProvider></StrictMode>);
