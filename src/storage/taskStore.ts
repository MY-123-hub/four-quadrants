import { useSyncExternalStore } from 'react';
import { moveTask, type QuadrantId, type Task } from '../domain/tasks';
import { applyChanges, changesBetween, type TaskChange, type TaskSnapshot } from '../domain/changes';
import { repository, type TaskRepository } from './repository';
type State = { tasks: Task[]; ready: boolean; saving: boolean; error: string | null };
export class TaskStore {
  constructor(private persistence: TaskRepository = repository) {}
  private state: State = { tasks: [], ready: false, saving: false, error: null };
  private base: TaskSnapshot = { revision: -1, tasks: [] };
  private listeners = new Set<() => void>();
  private pending: TaskChange[][] = [];
  private queue: Promise<void> | null = null;
  private loading: Promise<void> | null = null;
  private subscribed = false;
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  getSnapshot = () => this.state;
  private publish(patch: Partial<State>) { this.state = { ...this.state, ...patch }; this.listeners.forEach(fn => fn()); }
  private receive = (snapshot: TaskSnapshot) => {
    if (snapshot.revision < this.base.revision) return;
    this.base = snapshot;
    this.publish({ tasks: applyChanges(snapshot.tasks, this.pending.flat()), ready: true });
  };
  load = () => {
    if (this.loading) return this.loading;
    this.loading = (async () => {
      try {
        // Subscribe before reading: a commit between listener registration and load cannot be lost.
        if (!this.subscribed) { await this.persistence.subscribe?.(this.receive); this.subscribed = true; }
        this.receive(await this.persistence.load());
        if (!this.pending.length) this.publish({ error: null });
      } catch { this.publish({ error: '无法读取本机任务。请重试；现有数据不会被覆盖。' }); }
    })().finally(() => { this.loading = null; });
    return this.loading;
  };
  change = (update: (tasks: Task[]) => Task[]) => {
    if (!this.state.ready) return;
    this.enqueue(changesBetween(this.state.tasks, update(this.state.tasks)));
  };
  move = (id: string, quadrant: QuadrantId, overId?: string) => {
    if (!this.state.ready) return;
    const next = moveTask(this.state.tasks, id, quadrant, overId);
    if (next === this.state.tasks) return;
    const target = next.filter(t => t.quadrant === quadrant);
    this.enqueue([{ kind: 'move', id, quadrant, before: target[target.findIndex(t => t.id === id) + 1]?.id ?? null }]);
  };
  private enqueue(changes: TaskChange[]) {
    if (!changes.length) return;
    this.pending.push(changes);
    this.publish({ tasks: applyChanges(this.state.tasks, changes), saving: true });
    if (!this.state.error) this.drain();
  }
  private drain() {
    if (this.queue) return;
    this.queue = (async () => {
      while (this.pending.length) {
        try {
          const snapshot = await this.persistence.commit(this.pending[0]);
          this.pending.shift();
          this.receive(snapshot.revision >= this.base.revision ? snapshot : this.base);
          this.publish({ error: null });
        } catch {
          this.publish({ error: '保存未成功，修改仍保留在窗口中。请检查磁盘空间后重试。' });
          break;
        }
      }
      this.publish({ saving: this.pending.length > 0 && !this.state.error });
    })().finally(() => { this.queue = null; });
  }
  retry = () => { if (!this.state.ready) void this.load(); else { this.publish({ error: null }); this.drain(); } };
  flush = async () => {
    do { await this.queue; } while (this.queue);
    return !this.pending.length;
  };
}
export const taskStore = new TaskStore();
export function useTasks() { return useSyncExternalStore(taskStore.subscribe, taskStore.getSnapshot); }
