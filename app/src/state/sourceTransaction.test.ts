import { expect, test, vi } from 'vitest';
import { projectAcceptedSource } from './sourceTransaction';
import { NotebookController } from './notebookStore';
import { MockKernel } from '../test/mockKernel';
import type { NotebookFile } from '../kernel/generated/NotebookFile';
import type { KernelClient, Response } from '../kernel/client';
import { sourceMutation } from '../kernel/mirror';
const file = (cells: [string, string][], title = ''): NotebookFile => ({ version: 1, title, cells: cells.map(([id, source]) => ({ id, source, kind: 'Math', dialect: 'Auto' })) });
test('accepted source projects additions while preserving later edits, deletes and explicit user order', () => {
  const before = file([['a', '2'], ['b', '3']]);
  const accepted = file([['new', '9'], ['a', '5']], 'agent title');
  expect(projectAcceptedSource(before, accepted, file([['a', '7'], ['b', 'manual'], ['user', '10']], 'user title'))).toEqual(file([['new', '9'], ['a', '7'], ['b', 'manual'], ['user', '10']], 'user title'));
  expect(projectAcceptedSource(before, accepted, file([['b', '3']]))).toEqual(file([['new', '9']], 'agent title'));
  expect(before).toEqual(file([['a', '2'], ['b', '3']]));
});
test('rejected source commits leave draft/busy state intact and do not bypass admission',async()=>{
  const kernel=new MockKernel(()=>({type:'ok'})) as MockKernel&{commitSourceFile:NonNullable<KernelClient['commitSourceFile']>};
  kernel.commitSourceFile=vi.fn(async()=>({type:'error' as const,message:'STALE_SNAPSHOT'}));
  const controller=new NotebookController(kernel);controller.add('Math','2');await controller.flush();
  const before=controller.file(),desired={...before,title:'changed'};
  await expect(controller.commitSource(before,desired,()=>false)).rejects.toThrow('STALE_SNAPSHOT');
  expect(kernel.commitSourceFile).not.toHaveBeenCalled();
  await expect(controller.commitSource(before,desired,()=>true)).rejects.toThrow('STALE_SNAPSHOT');
  expect(controller.file()).toEqual(before);expect(controller.store.getState().busy).toBe(false);
  controller.dispose();
});
for (const mode of ['edit', 'move'] as const) test(`source commits share the manual queue and preserve ${mode} during ACK`, async () => {
  let current = file([]), release!: () => void, commits = 0;
  const state = (): Extract<Response, { type: 'notebook_state' }> => ({ type: 'notebook_state', state: { file: structuredClone(current), cells: current.cells.map(cell => ({ id: cell.id, status: 'Stale', defines: [], uses: [] })), definition_order: [], cycles: [] } });
  const kernel = new MockKernel(request => {
    if (request.type === 'get_notebook_state') return state();
    current = sourceMutation(current, request); return { type: 'ok' };
  }) as MockKernel & { commitSourceFile: NonNullable<KernelClient['commitSourceFile']> };
  kernel.commitSourceFile = async (expected, desired) => {
    if (++commits === 1) await new Promise<void>(resolve => { release = resolve; });
    expect(current).toEqual(expected); current = structuredClone(desired); return state();
  };
  const controller = new NotebookController(kernel);
  const a = controller.add('Math', '2'), b = controller.add('Math', '3'); await controller.flush();
  const before = controller.file(), desired = file([['new', '9'], [a, '5'], [b, '3']]);
  const committing = controller.commitSource(before, desired, () => true, 'operation-queue');
  await vi.waitFor(() => expect(release).toBeTypeOf('function'));
  if (mode === 'edit') { controller.edit(a, { source: '7' }); controller.rename('user title'); }
  else controller.move(a, 1);
  release(); await committing; await controller.flush();
  const expected = mode === 'edit' ? file([['new', '9'], [a, '7'], [b, '3']], 'user title') : file([[b, '3'], ['new', '9'], [a, '5']]);
  expect(controller.file()).toEqual(expected); expect(current).toEqual(expected);
  expect(controller.store.getState().busy).toBe(false);
  expect(kernel.requests.some(request => request.type === 'evaluate' || request.type === 'run_all')).toBe(false);
  expect(commits).toBe(mode === 'edit' ? 1 : 2);
  expect(controller.sourceWasAccepted('operation-queue',before,desired)).toBe(true);
  const count=commits;await controller.commitSource(before,desired,()=>false,'operation-queue');expect(commits).toBe(count);
  await expect(controller.commitSource(before,{...desired,title:'different'},()=>true,'operation-queue')).rejects.toThrow('PREVIEW_MISMATCH');
  controller.store.setState({documentGeneration:1});expect(controller.sourceWasAccepted('operation-queue',before,desired)).toBe(false);
  controller.dispose();
});
