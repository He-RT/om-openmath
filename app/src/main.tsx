import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles/theme.css';
import { KernelProvider } from './kernel/provider';

const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');

createRoot(root).render(
  <StrictMode>
    <KernelProvider><App /></KernelProvider>
  </StrictMode>,
);
