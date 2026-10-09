import { describe, expect, it } from 'vitest';
import { moveTask, validateTasks, type Task } from '../../src/domain/tasks';
const task = (id: string, quadrant: Task['quadrant'] = 'do'): Task => ({ id, title: id, quadrant, completed: false });
describe('moving tasks', () => {
  it('reorders in both directions without losing any task', () => {
    const tasks = [task('a'), task('b'), task('c')];
    const down = moveTask(tasks, 'a', 'do', 'c');
    expect(down.map(t => t.id)).toEqual(['b', 'c', 'a']);
    expect(moveTask(down, 'a', 'do', 'b').map(t => t.id)).toEqual(['a', 'b', 'c']);
  });
  it('moves to empty quadrants and inserts before a target across quadrants', () => {
    let tasks = [task('a'), task('b'), task('c', 'plan')];
    tasks = moveTask(tasks, 'a', 'eliminate');
    expect(tasks.find(t => t.id === 'a')?.quadrant).toBe('eliminate');
    tasks = moveTask(tasks, 'b', 'plan', 'c');
    expect(tasks.filter(t => t.quadrant === 'plan').map(t => t.id)).toEqual(['b', 'c']);
    expect(new Set(tasks.map(t => t.id)).size).toBe(3);
  });
  it('rejects duplicate ids and malformed saved data', () => {
    expect(() => validateTasks([task('a'), task('a')])).toThrow();
    expect(() => validateTasks([{ ...task('b'), title: ' ' }])).toThrow();
    expect(() => validateTasks({ tasks: [] })).toThrow();
  });
});
