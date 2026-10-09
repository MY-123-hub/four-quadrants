import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { validateTasks } from '../domain/tasks';
import { applyChanges, type TaskChange, type TaskSnapshot } from '../domain/changes';
const previewKey = 'four-quadrants-browser-preview-v1';
const revisionKey = `${previewKey}-revision`;
export const native = isTauri();
export interface TaskRepository {
  load(): Promise<TaskSnapshot>;
  commit(changes: TaskChange[]): Promise<TaskSnapshot>;
  subscribe?(receive: (snapshot: TaskSnapshot) => void): Promise<() => void>;
}
const previewSnapshot = (): TaskSnapshot => ({ revision: Number(localStorage.getItem(revisionKey) ?? 0), tasks: validateTasks(JSON.parse(localStorage.getItem(previewKey) ?? '[]')) });
export const repository: TaskRepository = {
  async load() { return native ? await invoke<TaskSnapshot>('load_tasks') : previewSnapshot(); },
  async commit(changes) {
    if (native) return await invoke<TaskSnapshot>('commit_tasks', { changes });
    const old = previewSnapshot();
    const snapshot = { revision: old.revision + 1, tasks: validateTasks(applyChanges(old.tasks, changes)) };
    localStorage.setItem(previewKey, JSON.stringify(snapshot.tasks));
    localStorage.setItem(revisionKey, String(snapshot.revision));
    return snapshot;
  },
  async subscribe(receive) {
    if (native) return await listen<TaskSnapshot>('tasks-updated', event => receive(event.payload));
    const onStorage = (event: StorageEvent) => { if (event.key === revisionKey) receive(previewSnapshot()); };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  },
};
