import { expect, it } from 'vitest';
import { TaskStore } from '../../src/storage/taskStore';
import type { Task } from '../../src/domain/tasks';
const task: Task = { id: 'a', title: '交付检查', completed: false, quadrant: 'do' };
it('serializes rapid changes and flushes the newest state before closing', async () => {
  const writes: Task[][] = [];
  const store = new TaskStore({ load: async () => [], save: async tasks => { await new Promise(resolve => setTimeout(resolve, 8)); writes.push(tasks); } });
  await store.load();
  store.change(() => [task]);
  store.change(tasks => tasks.map(t => ({ ...t, completed: true })));
  const flushing = store.flush();
  store.change(tasks => tasks.map(t => ({ ...t, quadrant: 'plan' })));
  expect(await flushing).toBe(true);
  expect(writes).toHaveLength(3);
  expect(writes[2]).toEqual([{ ...task, completed: true, quadrant: 'plan' }]);
});
it('retains unsaved changes after disk failure and can retry', async () => {
  let failed = true;
  let saved: Task[] = [];
  const store = new TaskStore({ load: async () => [], save: async tasks => { if (failed) throw new Error('disk full'); saved = tasks; } });
  await store.load(); store.change(() => [task]);
  expect(await store.flush()).toBe(false);
  expect(store.getSnapshot().tasks).toEqual([task]);
  expect(store.getSnapshot().error).toContain('保存未成功');
  failed = false; store.retry();
  expect(await store.flush()).toBe(true);
  expect(saved).toEqual([task]);
});
it('does not overwrite unreadable data and permits quitting without edits', async () => {
  let writes = 0;
  const store = new TaskStore({ load: async () => { throw new Error('bad data'); }, save: async () => { writes++; } });
  await store.load(); store.change(() => [task]);
  expect(store.getSnapshot().ready).toBe(false);
  expect(writes).toBe(0);
  expect(await store.flush()).toBe(true);
});
