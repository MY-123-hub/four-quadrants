import { expect, test, type Page } from '@playwright/test';
const zone = (page: Page, id: string) => page.locator(`[data-quadrant="${id}"]`);
async function add(page: Page, id: string, title: string) {
  await zone(page, id).locator('.add-task').click();
  const input = zone(page, id).getByRole('textbox');
  await input.fill(title); await input.press('Enter');
  await expect(zone(page, id).getByRole('button', { name: title, exact: true })).toBeVisible();
}
async function drag(page: Page, title: string, target: { x: number; y: number }) {
  const handle = page.getByRole('button', { name: `拖动 ${title}`, exact: true });
  const box = (await handle.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 8, box.y + box.height / 2, { steps: 3 });
  await page.mouse.move(target.x, target.y, { steps: 18 });
  await page.mouse.up();
}
test.beforeEach(async ({ page }) => { await page.goto('/'); await expect(page.locator('.quadrant')).toHaveCount(4); });
test('create, inline edit, finish, undo, delete and reload', async ({ page }) => {
  await add(page, 'do', '核对 Mac 交付清单');
  await page.getByRole('button', { name: '核对 Mac 交付清单', exact: true }).click();
  await page.getByRole('textbox').fill('核对 Mac 最终清单');
  await page.getByRole('textbox').press('Enter');
  const check = page.getByRole('checkbox', { name: '完成 核对 Mac 最终清单' });
  await check.click();
  await expect(page.getByRole('checkbox', { name: '取消完成 核对 Mac 最终清单' })).toHaveAttribute('aria-checked', 'true');
  await page.reload();
  await page.getByRole('checkbox', { name: '取消完成 核对 Mac 最终清单' }).click();
  await expect(page.getByRole('checkbox', { name: '完成 核对 Mac 最终清单' })).toHaveAttribute('aria-checked', 'false');
  await page.getByRole('button', { name: '核对 Mac 最终清单', exact: true }).hover();
  await page.getByRole('button', { name: '删除 核对 Mac 最终清单' }).click();
  await page.reload();
  await expect(zone(page, 'do').locator('.task-row')).toHaveCount(0);
});
test('empty input, escape, blur and Chinese composition', async ({ page }) => {
  await zone(page, 'plan').locator('.add-task').click();
  await page.getByRole('textbox').fill('   '); await page.getByRole('textbox').press('Enter');
  await expect(zone(page, 'plan').locator('.task-row')).toHaveCount(0);
  await zone(page, 'plan').locator('.add-task').click();
  const input = page.getByRole('textbox');
  await input.fill('中文输入法');
  await input.evaluate(el => el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true })));
  await expect(input).toBeVisible();
  await page.locator('h2').first().click();
  await expect(zone(page, 'plan').getByRole('button', { name: '中文输入法', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '中文输入法', exact: true }).click();
  await page.getByRole('textbox').fill('不保存这个修改'); await page.getByRole('textbox').press('Escape');
  await expect(zone(page, 'plan').getByRole('button', { name: '中文输入法', exact: true })).toBeVisible();
});
test('drag across quadrants and reorder within the same quadrant', async ({ page }) => {
  await add(page, 'do', '第一件事'); await add(page, 'do', '第二件事'); await add(page, 'do', '第三件事');
  const third = (await page.getByRole('button', { name: '第三件事', exact: true }).boundingBox())!;
  await drag(page, '第一件事', { x: third.x + 60, y: third.y + 12 });
  await expect(zone(page, 'do').locator('.task-title')).toHaveText(['第二件事', '第三件事', '第一件事']);
  const plan = (await zone(page, 'plan').boundingBox())!;
  await drag(page, '第三件事', { x: plan.x + 150, y: plan.y + 120 });
  await expect(zone(page, 'plan').locator('.task-title')).toHaveText(['第三件事']);
  await page.reload();
  await expect(zone(page, 'do').locator('.task-title')).toHaveText(['第二件事', '第一件事']);
  await expect(zone(page, 'plan').locator('.task-title')).toHaveText(['第三件事']);
});
test('layout stays 2 by 2 at minimum size, long lists scroll, themes follow the system', async ({ page }) => {
  await page.evaluate(() => localStorage.setItem('four-quadrants-browser-preview-v1', JSON.stringify(Array.from({ length: 45 }, (_, i) => ({ id: `long-${i}`, title: i === 0 ? '中文 English 混排的长任务名称 '.repeat(8) : `要完成的任务 ${i}`, quadrant: 'do', completed: false })))));
  await page.reload(); await page.setViewportSize({ width: 620, height: 440 });
  const boxes = await page.locator('.quadrant').evaluateAll(elements => elements.map(el => { const r = el.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height }; }));
  expect(boxes[0].y).toBe(boxes[1].y); expect(boxes[2].y).toBe(boxes[3].y);
  expect(boxes[0].width).toBe(boxes[1].width); expect(boxes[2].y).toBeGreaterThan(boxes[0].y);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  expect(await zone(page, 'do').locator('.task-scroll').evaluate(el => el.scrollHeight > el.clientHeight)).toBe(true);
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('body')).toHaveCSS('color', 'rgb(223, 223, 216)');
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('body')).toHaveCSS('color', 'rgb(48, 49, 47)');
});
test('screenshots with representative content, empty, light, dark, compact', async ({ page }, info) => {
  await page.screenshot({ path: info.outputPath('empty.png') });
  await add(page, 'do', '把四象限应用交付到 Mac'); await add(page, 'do', '确认合同里的验收标准');
  await add(page, 'plan', '整理下一版硬件的测试清单'); await add(page, 'plan', '留一点时间读书');
  await add(page, 'delegate', '回复供应商的交期邮件'); await add(page, 'eliminate', '整理暂时不用的下载文件');
  await page.getByRole('checkbox', { name: '完成 确认合同里的验收标准' }).click();
  await expect(zone(page, 'do').locator('.completed .task-title')).toHaveCSS('color', 'rgb(144, 146, 139)');
  await page.mouse.move(510, 22);
  await page.screenshot({ path: info.outputPath('light.png') });
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(zone(page, 'plan').locator('.task-title').first()).toHaveCSS('color', 'rgb(223, 223, 216)');
  await page.screenshot({ path: info.outputPath('dark.png') });
  await page.setViewportSize({ width: 620, height: 440 }); await page.screenshot({ path: info.outputPath('compact.png') });
});
test('keyboard dragging reorders and crosses quadrants', async ({ page }) => {
  await add(page, 'do', '键盘任务 A'); await add(page, 'do', '键盘任务 B');
  const handle = page.getByRole('button', { name: '拖动 键盘任务 A', exact: true });
  await handle.focus(); await page.keyboard.press('Space');
  const overlay = page.locator('.drag-preview');
  await expect(overlay).toBeVisible();
  const initialY = (await overlay.boundingBox())!.y;
  await page.keyboard.press('ArrowDown');
  await expect.poll(async () => (await overlay.boundingBox())?.y ?? 0).toBeGreaterThan(initialY + 20);
  await page.keyboard.press('Space');
  await expect(zone(page, 'do').locator('.task-title')).toHaveText(['键盘任务 B', '键盘任务 A']);
  await expect(overlay).toHaveCount(0);
  await handle.focus(); await page.keyboard.press('Space');
  await expect(overlay).toBeVisible();
  const initialX = (await overlay.boundingBox())!.x;
  await page.keyboard.press('ArrowRight');
  await expect.poll(async () => (await overlay.boundingBox())?.x ?? 0).toBeGreaterThan(initialX + 100);
  await page.keyboard.press('Space');
  await expect(zone(page, 'plan').locator('.task-title')).toHaveText(['键盘任务 A']);
});
test('dropping outside the board does not reclassify a task', async ({ page }) => {
  await add(page, 'do', '保留原象限');
  await drag(page, '保留原象限', { x: 800, y: 22 });
  await expect(zone(page, 'do').locator('.task-title')).toHaveText(['保留原象限']);
  await expect(zone(page, 'plan').locator('.task-title')).toHaveCount(0);
});
