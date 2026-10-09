import { invoke, isTauri } from '@tauri-apps/api/core';
import { validateTasks, type Task } from '../domain/tasks';
const previewKey = 'four-quadrants-browser-preview-v1';
export const native = isTauri();
export const repository = {
  async load(): Promise<Task[]> {
    const value: unknown = native ? await invoke('load_tasks') : JSON.parse(localStorage.getItem(previewKey) ?? '[]');
    return validateTasks(value);
  },
  async save(tasks: Task[]): Promise<void> {
    if (native) await invoke('save_tasks', { tasks });
    else localStorage.setItem(previewKey, JSON.stringify(tasks));
  },
};
