import { expect, it } from 'vitest';
import { TaskStore } from '../../src/storage/taskStore';
import { applyChanges, type TaskChange, type TaskSnapshot } from '../../src/domain/changes';
import type { Task } from '../../src/domain/tasks';
import type { TaskRepository } from '../../src/storage/repository';
const task: Task = { id: 'a', title: '交付检查', completed: false, quadrant: 'do' };
function memoryRepository(initial: Task[] = [], delay = 0) {
  let snapshot: TaskSnapshot = { revision: 0, tasks: initial };
  const listeners: ((snapshot: TaskSnapshot) => void)[] = [];
  const writes: Task[][] = [];
  const repository: TaskRepository = {
    load: async () => snapshot,
    subscribe: async receive => { listeners.push(receive); return () => {}; },
    commit: async changes => {
      if (delay) await new Promise(resolve => setTimeout(resolve, delay));
      snapshot = { revision: snapshot.revision + 1, tasks: applyChanges(snapshot.tasks, changes) };
      writes.push(snapshot.tasks); listeners.forEach(fn => fn(snapshot)); return snapshot;
    },
  };
  return { repository, writes };
}
it('serializes rapid changes and flushes the newest pending intent before closing', async () => {
  const { repository, writes } = memoryRepository([], 8);
  const store = new TaskStore(repository);
  await store.load();
  store.change(() => [task]);
  store.change(tasks => tasks.map(t => ({ ...t, completed: true })));
  const flushing = store.flush();
  store.change(tasks => tasks.map(t => ({ ...t, quadrant: 'plan' })));
  expect(await flushing).toBe(true);
  expect(writes).toHaveLength(3);
  expect(writes[2]).toEqual([{ ...task, completed: true, quadrant: 'plan' }]);
});
it('retains unsaved changes after disk failure, incorporates remote changes and retries', async () => {
  let failed = true;
  const shared = memoryRepository([task]);
  const store = new TaskStore({ ...shared.repository, commit: async changes => { if (failed) throw new Error('disk full'); return shared.repository.commit(changes); } });
  const other = new TaskStore(shared.repository);
  await store.load(); await other.load();
  store.change(tasks => tasks.map(t => ({ ...t, title: '修改后的名称' })));
  expect(await store.flush()).toBe(false);
  other.change(tasks => tasks.map(t => ({ ...t, completed: true })));
  await other.flush();
  expect(store.getSnapshot().tasks).toEqual([{ ...task, title: '修改后的名称', completed: true }]);
  failed = false; store.retry();
  expect(await store.flush()).toBe(true);
  expect(other.getSnapshot().tasks).toEqual(store.getSnapshot().tasks);
});
it('merges concurrent edits from independent stores without losing additions or unrelated fields', async () => {
  const shared = memoryRepository([task], 5);
  const main = new TaskStore(shared.repository), other = new TaskStore(shared.repository);
  await main.load(); await other.load();
  main.change(tasks => tasks.map(t => ({ ...t, title: 'Edited in main' })));
  other.change(tasks => tasks.map(t => ({ ...t, completed: true })));
  main.change(tasks => [...tasks, { ...task, id: 'b', title: '新增 B' }]);
  other.change(tasks => [...tasks, { ...task, id: 'c', title: '新增 C' }]);
  await Promise.all([main.flush(), other.flush()]);
  expect(main.getSnapshot().tasks).toEqual(other.getSnapshot().tasks);
  expect(main.getSnapshot().tasks).toHaveLength(3);
  expect(main.getSnapshot().tasks.find(t => t.id === 'a')).toEqual({ ...task, title: 'Edited in main', completed: true });
});
it('ignores late snapshots, keeps pending edits, and never resurrects a deleted task', async () => {
  let receive!: (snapshot: TaskSnapshot) => void;
  let complete!: (snapshot: TaskSnapshot) => void;
  const store = new TaskStore({ load: async () => ({ revision: 2, tasks: [task] }), subscribe: async listener => { receive = listener; return () => {}; }, commit: async () => new Promise(resolve => { complete = resolve; }) });
  await store.load();
  store.change(tasks => tasks.map(t => ({ ...t, title: '待保存标题' })));
  receive({ revision: 4, tasks: [] });
  receive({ revision: 1, tasks: [task] });
  complete({ revision: 3, tasks: [{ ...task, title: '待保存标题' }] });
  await store.flush();
  expect(store.getSnapshot().tasks).toEqual([]);
});
it('does not overwrite unreadable data and permits quitting without edits', async () => {
  let writes = 0;
  const store = new TaskStore({ load: async () => { throw new Error('bad data'); }, commit: async () => { writes++; return { revision: 0, tasks: [] }; } });
  await store.load(); store.change(() => [task]);
  expect(store.getSnapshot().ready).toBe(false);
  expect(writes).toBe(0);
  expect(await store.flush()).toBe(true);
});
it('moves one task using an anchor while preserving concurrent additions', async () => {
  const shared = memoryRepository([task, { ...task, id: 'b' }], 5);
  const main = new TaskStore(shared.repository), other = new TaskStore(shared.repository);
  await main.load(); await other.load();
  main.move('a', 'do', 'b');
  other.change(tasks => [...tasks, { ...task, id: 'c' }]);
  await Promise.all([main.flush(), other.flush()]);
  expect(main.getSnapshot().tasks.map(t => t.id)).toEqual(['b', 'a', 'c']);
});
it('only sends the changed field, even when another field differs on disk', () => {
  const changes: TaskChange[] = [{ kind: 'patch', id: 'a', completed: true }];
  expect(applyChanges([{ ...task, title: '新标题' }], changes)).toEqual([{ ...task, title: '新标题', completed: true }]);
});
it('keeps map associations and pending title edits together, rejects late map snapshots', async () => {
  let receive!: (snapshot: TaskSnapshot) => void;
  let complete!: (snapshot: TaskSnapshot) => void;
  const store = new TaskStore({ load: async () => ({ revision: 1, tasks: [task] }), subscribe: async listener => { receive = listener; return () => {}; }, commit: async () => new Promise(resolve => { complete = resolve; }) });
  await store.load(); store.change(tasks => tasks.map(t => ({ ...t, title: '正在编辑' })));
  receive({ revision: 3, tasks: [{ ...task, title: '正在编辑' }], mindmaps: { a: '/maps/a.xmind' } });
  complete({ revision: 2, tasks: [{ ...task, title: '正在编辑' }] }); await store.flush();
  expect(store.getSnapshot().mindmaps).toEqual({ a: '/maps/a.xmind' });
  expect(store.getSnapshot().tasks[0].title).toBe('正在编辑');
  receive({ revision: 2, tasks: [task] });
  expect(store.getSnapshot().mindmaps.a).toBe('/maps/a.xmind');
});
it('waits for a pending native map operation before hiding or exiting', async () => {
  const store = new TaskStore(memoryRepository([task]).repository); await store.load();
  let finish!: () => void;
  const operation = store.trackOperation(() => new Promise<void>(resolve => { finish = resolve; }));
  let flushed = false; const flushing = store.flush().then(value => { flushed = true; return value; });
  await Promise.resolve(); expect(flushed).toBe(false);
  finish(); await operation; expect(await flushing).toBe(true);
});
