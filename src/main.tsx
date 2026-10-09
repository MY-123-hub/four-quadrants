import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './fonts.css';
import './style.css';
import App from './App';
import { Widget } from './components/Widget';
import './widget.css';
const widget = new URLSearchParams(location.search).has('widget');
createRoot(document.getElementById('root')!).render(<StrictMode>{widget ? <Widget /> : <App />}</StrictMode>);
