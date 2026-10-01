import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { stepsEn } from './steps.en';
import { stepsZhCN } from './steps.zh-CN';

describe('solver step templates', () => {
  it('covers exactly the exported stable rule IDs in both languages', () => {
    const ids = readFileSync(resolve(process.cwd(), '../crates/om-solve/rule_ids.txt'), 'utf8')
      .trim().split('\n').map((id) => `step.${id}`).sort();
    for (const templates of [stepsEn, stepsZhCN]) {
      expect(Object.keys(templates).sort()).toEqual(ids);
      expect(Object.values(templates).every((text) => text.trim().length > 0)).toBe(true);
    }
  });
});
