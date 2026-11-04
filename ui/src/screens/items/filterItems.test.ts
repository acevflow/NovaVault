import { describe, expect, it } from 'vitest';

import type { ItemSummary } from '../../api/types';
import { filterItems } from './filterItems';

function item(id: string, kind: ItemSummary['kind'], title: string): ItemSummary {
  return { id, kind, title, createdAtMs: 0, updatedAtMs: 0 };
}

const ITEMS = [
  item('1', 'password', 'GitHub'),
  item('2', 'note', 'Café receipts'),
  item('3', 'secret', 'github token'),
  item('4', 'password', 'Bank'),
];

function titles(items: readonly ItemSummary[]): string[] {
  return items.map((entry) => entry.title);
}

describe('filterItems', () => {
  it('returns every item sorted by title when nothing is filtered', () => {
    expect(titles(filterItems(ITEMS, '', 'all'))).toEqual([
      'Bank',
      'Café receipts',
      'GitHub',
      'github token',
    ]);
  });

  it('searches titles ignoring case, accents and surrounding spaces', () => {
    expect(titles(filterItems(ITEMS, '  GITHUB ', 'all'))).toEqual(['GitHub', 'github token']);
    expect(titles(filterItems(ITEMS, 'cafe', 'all'))).toEqual(['Café receipts']);
    expect(filterItems(ITEMS, 'nothing', 'all')).toEqual([]);
  });

  it('filters by type and combines with the search', () => {
    expect(titles(filterItems(ITEMS, '', 'password'))).toEqual(['Bank', 'GitHub']);
    expect(titles(filterItems(ITEMS, 'git', 'secret'))).toEqual(['github token']);
    expect(filterItems(ITEMS, 'bank', 'note')).toEqual([]);
  });

  it('does not modify its input', () => {
    const copy = [...ITEMS];
    filterItems(ITEMS, '', 'all');
    expect(ITEMS).toEqual(copy);
  });
});
