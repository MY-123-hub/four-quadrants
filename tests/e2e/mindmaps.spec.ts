import { expect, test } from '@playwright/test';

const tasks = [
  { id: 'map-demo', title: '梳理项目思路', completed: false, quadrant: 'plan' },
  { id: 'map-other', title: '准备测试方案', completed: false, quadrant: 'do' },
];
  test(`one header button follows selection, supports rename and clears on deletion in main`, async ({ page }, info) => {
    await page.setViewportSize({ width: 620, height: 440 });
    await page.goto('/');
    await page.evaluate(tasks => localStorage.setItem('four-quadrants-browser-preview-v1', JSON.stringify(tasks)), tasks);
    await page.reload();
    const toolbar = page.locator('.mindmap-button');
    await expect(toolbar).toHaveCount(1); await expect(toolbar).toBeDisabled();
    await expect(page.locator('[data-task-id] .mindmap-button')).toHaveCount(0);
    const title = page.getByRole('button', { name: tasks[0].title, exact: true });
    await title.click(); await expect(title).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByRole('textbox')).toHaveCount(0);
    await expect(toolbar).toBeEnabled(); await expect(toolbar).toHaveAccessibleName(`思维导图 ${tasks[0].title}`);
    const header = page.locator('.window-actions');
    await expect(header.locator('.mindmap-button')).toHaveCount(1);
    const position = await toolbar.boundingBox();
    expect(position!.y).toBeLessThan(44); expect(position!.x).toBeGreaterThan(620 - 150);
    await page.screenshot({ path: info.outputPath(`mindmap-main-selected-light.png`) });
    await toolbar.click();
    const dialog = page.getByRole('dialog', { name: '思维导图', exact: true });
    await expect(dialog.locator('.mindmap-task-name')).toHaveText(tasks[0].title);
    await expect(dialog.getByRole('button', { name: '新建思维导图', exact: true })).toBeVisible();
    await expect(dialog.getByRole('button', { name: '关联已有导图', exact: true })).toBeVisible();
    await page.keyboard.press('Escape'); await expect(dialog).not.toBeVisible(); await expect(toolbar).toBeFocused();
    const other = page.getByRole('button', { name: tasks[1].title, exact: true });
    await other.click(); await expect(title).toHaveAttribute('aria-pressed', 'false');
    await expect(toolbar).toHaveAccessibleName(`思维导图 ${tasks[1].title}`);
    await other.dblclick(); await page.getByRole('textbox').fill('修改后的测试方案'); await page.getByRole('textbox').press('Enter');
    await expect(toolbar).toHaveAccessibleName('思维导图 修改后的测试方案');
    await page.getByRole('button', { name: '修改后的测试方案', exact: true }).press('Enter');
    await expect(page.getByRole('textbox')).toBeVisible(); await page.keyboard.press('Escape');
    await page.emulateMedia({ colorScheme: 'dark' });
    await expect(page.locator('body')).toHaveCSS('color', 'rgb(223, 223, 216)');
    await expect(page.getByRole('button', { name: '修改后的测试方案', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: '修改后的测试方案', exact: true })).toHaveCSS('color', 'rgb(223, 223, 216)');
    await expect(title).toHaveCSS('color', 'rgb(223, 223, 216)');
    await page.mouse.move(2, 2);
    await page.screenshot({ path: info.outputPath(`mindmap-main-selected-dark.png`) });
    await toolbar.click();
    await expect(dialog.locator('.mindmap-task-name')).toHaveText('修改后的测试方案');
    await dialog.getByRole('button', { name: '关联已有导图', exact: true }).click();
    await expect(dialog.getByRole('alert')).toHaveText('请在四象限桌面应用中打开思维导图。');
    await dialog.getByRole('button', { name: '关闭导图选项' }).click();
    await page.getByRole('button', { name: '修改后的测试方案', exact: true }).hover();
    await page.getByRole('button', { name: '删除 修改后的测试方案', exact: true }).click();
    await expect(toolbar).toBeDisabled();
    await title.click(); await expect(toolbar).toBeEnabled();
    await page.reload(); await expect(toolbar).toBeDisabled();
  });
  test(`header creates and reopens the selected task only in main`, async ({ page }) => {
    await page.addInitScript(({ tasks }) => {
      const state = { revision: 1, tasks, mindmaps: { 'map-demo': '/maps/existing.xmind' } as Record<string, string> };
      const calls: { id: string; action: string }[] = [];
      Object.assign(window, {
        isTauri: true, mindmapCalls: calls,
        __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
        __TAURI_INTERNALS__: {
          metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
          transformCallback: () => 0,
          invoke: async (command: string, args: { id: string; action: string }) => {
            if (command === 'load_tasks') return structuredClone(state);
            if (command === 'mindmap_action') {
              calls.push(args);
              if (args.action === 'create') { state.mindmaps[args.id] = '/maps/created.xmind'; state.revision++; }
              return structuredClone(state);
            }
            return 0;
          },
        },
      });
    }, { tasks });
    await page.goto('/');
    await page.getByRole('button', { name: tasks[0].title, exact: true }).click();
    const toolbar = page.locator('.mindmap-button'); await toolbar.click();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => (window as unknown as { mindmapCalls: unknown[] }).mindmapCalls)).toEqual([{ id: tasks[0].id, action: 'open' }]);
    await page.getByRole('button', { name: tasks[1].title, exact: true }).click(); await toolbar.click();
    await page.getByRole('button', { name: '新建思维导图', exact: true }).click();
    await expect(page.getByRole('dialog')).toHaveCount(0); await toolbar.click();
    await expect.poll(() => page.evaluate(() => (window as unknown as { mindmapCalls: unknown[] }).mindmapCalls)).toEqual([
      { id: tasks[0].id, action: 'open' }, { id: tasks[1].id, action: 'create' }, { id: tasks[1].id, action: 'open' },
    ]);
  });

test('old widget URLs show the main board without any floating-window controls', async ({ page }) => {
  await page.goto('/?widget');
  await expect(page.locator('.board .quadrant')).toHaveCount(4);
  await expect(page.locator('.widget-shell')).toHaveCount(0);
  await expect(page.getByRole('button', { name: '显示悬浮小组件', exact: true })).toHaveCount(0);
  await expect(page.locator('.window-actions button')).toHaveCount(1);
});
