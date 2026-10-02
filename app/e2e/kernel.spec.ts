import { test, expect } from '@playwright/test';
test('real WASM worker solves, detects parse errors and restores definitions after termination', async ({ page }) => {
  await page.goto('/');
  const output = await page.evaluate(async () => {
    const modulePath = '/src/kernel/wasmClient.ts';
    const { WasmClient } = await import(/* @vite-ignore */ modulePath);
    const client = new WasmClient(); await client.ready;
    try {
      await client.request({ type: 'upsert_cell', cell: { id: 'b', kind: 'Math', source: 'let b=a+1', dialect: 'Modern' } });
      await client.request({ type: 'upsert_cell', cell: { id: 'a', kind: 'Math', source: 'let a=9', dialect: 'Modern' } });
      await client.request({ type: 'upsert_cell', cell: { id: 'slow', kind: 'Math', source: 'factorial(10000000)', dialect: 'Modern' } });
      const solved = await client.request({ type: 'evaluate', cell_id: 'solve', source: 'solve(x^2==4,x)', dialect: 'Modern' });
      const bad = await client.request({ type: 'evaluate', cell_id: 'bad', source: 'solve(', dialect: 'Modern' });
      const cfg = await client.request({ type: 'get_config' });
      if (cfg.type !== 'config') throw new Error('config'); cfg.config.general.eval_timeout_ms = 1_000_000;
      await client.request({ type: 'set_config', config: cfg.config });
      const failed = client.request({ type: 'evaluate', cell_id: 'current', source: 'Factorial[1000000]', dialect: 'Wolfram' }).then(() => false, () => true);
      await new Promise(resolve => setTimeout(resolve, 20)); await client.interrupt();
      const interrupted = await failed;
      const state = await client.request({ type: 'get_notebook_state' });
      const value = await client.request({ type: 'evaluate', cell_id: 'value', source: 'b+1', dialect: 'Modern' });
      return { solved, bad, interrupted, state, value };
    } finally { client.dispose(); }
  });
  expect(output.solved.type).toBe('evaluated');
  expect(output.solved.output.items[0].view.solutions).toHaveLength(2);
  expect(output.bad.output.messages.some((m: { level: string }) => m.level === 'Error')).toBe(true);
  expect(output.interrupted).toBe(true);
  expect(output.state.state.file.cells.find((c: { id: string }) => c.id === 'current').source).toBe('Factorial[1000000]');
  expect(output.state.state.cells.find((c: { id: string }) => c.id === 'slow').status).toBe('Stale');
  expect(output.value.output.items[0].input_form).toBe('11');
});
test('actual browser fetch streams provider fixture through WASM and returns a checked card', async ({ page }) => {
  await page.route('https://mock-provider.invalid/**', async route => {
    const text = JSON.stringify({ wolfram: 'Solve[x^2==4,x]', explanation: '真实建议' });
    const data = `data: ${JSON.stringify({ choices: [{ index: 0, delta: { content: text }, finish_reason: 'stop' }] })}\n\ndata: [DONE]\n\n`;
    await route.fulfill({ status: 200, contentType: 'text/event-stream', body: data, headers: { 'access-control-allow-origin': '*' } });
  });
  await page.goto('/');
  const result = await page.evaluate(async () => {
    const modulePath = '/src/kernel/wasmClient.ts';
    const { WasmClient } = await import(/* @vite-ignore */ modulePath);
    const client = new WasmClient(); await client.ready;
    try {
      const cfg = await client.request({ type: 'get_config' }); if (cfg.type !== 'config') throw new Error('config');
      cfg.config.llm.profiles[0].base_url = 'https://mock-provider.invalid/v1'; cfg.config.llm.profiles[0].api_key = 'synthetic-browser-key';
      await client.request({ type: 'set_config', config: cfg.config });
      const events: unknown[] = [];
      const completed = new Promise<void>((resolve, reject) => client.onEvent((e: { type: string; request_id?: string }) => {
        events.push(e); if (e.request_id !== 'translate') return;
        if (e.type === 'llm_done') resolve(); if (e.type === 'llm_error') reject(new Error('provider failure'));
      }));
      const started = await client.request({ type: 'llm_translate', request_id: 'translate', text: 'solve x squared = 4', cell_id: null });
      await completed;
      const state = await client.request({ type: 'save_notebook' });
      return { started, events, state };
    } finally { client.dispose(); }
  });
  expect(result.started.http).toBeNull(); expect(result.events.some((e: unknown) => (e as { type: string }).type === 'llm_suggestion')).toBe(true);
  expect(result.state.file.cells).toHaveLength(0);
  expect(JSON.stringify(result)).not.toContain('synthetic-browser-key');
});
