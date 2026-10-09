import { useSyncExternalStore } from 'react';
import type { Task } from '../domain/tasks';
import { repository } from './repository';
type State = { tasks: Task[]; ready: boolean; saving: boolean; error: string | null };
export class TaskStore {
  constructor(private persistence = repository) {}
  private state: State = { tasks: [], ready: false, saving: false, error: null };
  private listeners = new Set<() => void>();
  private queue: Promise<void> = Promise.resolve();
  private version = 0;
  private loading: Promise<void> | null = null;
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  getSnapshot = () => this.state;
  private publish(patch: Partial<State>) { this.state = { ...this.state, ...patch }; this.listeners.forEach(fn => fn()); }
  load = () => {
    if (this.loading) return this.loading;
    this.loading = this.persistence.load().then(tasks => this.publish({ tasks, ready: true, error: null })).catch(() => this.publish({ error: '无法读取本机任务。请重试；现有数据不会被覆盖。' })).finally(() => { this.loading = null; });
    return this.loading;
  };
  change = (update: (tasks: Task[]) => Task[]) => {
    if (!this.state.ready) return;
    const tasks = update(this.state.tasks);
    if (tasks === this.state.tasks) return;
    this.publish({ tasks });
    this.save(tasks);
  };
  private save(tasks: Task[]) {
    const version = ++this.version;
    this.publish({ saving: true });
    this.queue = this.queue.then(() => this.persistence.save(tasks)).then(() => {
      if (version === this.version) this.publish({ saving: false, error: null });
    }).catch(() => {
      if (version === this.version) this.publish({ saving: false, error: '保存未成功，修改仍保留在窗口中。请检查磁盘空间后重试。' });
    });
  }
  retry = () => this.state.ready ? this.save(this.state.tasks) : void this.load();
  flush = async () => {
    let queue: Promise<void>;
    do { queue = this.queue; await queue; } while (queue !== this.queue);
    return !this.state.ready || this.state.error === null;
  };
}
export const taskStore = new TaskStore();
export function useTasks() { return useSyncExternalStore(taskStore.subscribe, taskStore.getSnapshot); }
