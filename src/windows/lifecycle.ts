import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { native } from '../storage/repository';
import { taskStore } from '../storage/taskStore';

const commitEditor = () => (document.activeElement as HTMLElement | null)?.blur();
const waitForMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches ? Promise.resolve() : new Promise<void>(resolve => setTimeout(resolve, 140));
export async function hideCurrentWindow() {
  commitEditor();
  if (!await taskStore.flush()) return;
  document.documentElement.classList.add('window-hiding');
  await waitForMotion();
  try { if (native) await invoke('hide_window'); }
  finally { document.documentElement.classList.remove('window-hiding'); }
}
export function useWindowLifecycle() {
  useEffect(() => {
    void taskStore.load();
    if (!native) return;
    let exiting = false;
    const subscriptions = [
      listen<number>('request-shutdown', async event => {
        if (exiting) return;
        exiting = true;
        commitEditor();
        document.body.inert = true;
        const success = await taskStore.flush();
        await invoke('finish_exit', { success, exitId: event.payload });
        exiting = false;
      }),
      listen('shutdown-cancelled', () => { document.body.inert = false; exiting = false; }),
      getCurrentWebviewWindow().listen('request-hide', () => { void hideCurrentWindow(); }),
    ];
    return () => { subscriptions.forEach(subscription => { void subscription.then(unlisten => unlisten()); }); };
  }, []);
}
