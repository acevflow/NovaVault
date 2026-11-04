/**
 * Searching and filtering the item list.
 *
 * Runs entirely in the UI over the titles from `listItems`: titles are
 * already in the webview to draw the list, and no secret field is ever
 * searched.
 */
import type { ItemKind, ItemSummary } from '../../api/types';

/** Which kinds of item to show: one kind, or all of them. */
export type KindFilter = 'all' | ItemKind;

/**
 * The items whose title contains `query` (ignoring case, accents and
 * surrounding spaces) and whose kind matches `kind`, sorted by title.
 */
export function filterItems(
  items: readonly ItemSummary[],
  query: string,
  kind: KindFilter,
): ItemSummary[] {
  const needle = normalize(query.trim());
  return items
    .filter((item) => kind === 'all' || item.kind === kind)
    .filter((item) => needle === '' || normalize(item.title).includes(needle))
    .sort(
      (a, b) =>
        a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }) ||
        a.id.localeCompare(b.id),
    );
}

/** Lower case without diacritics, so "cafe" finds "Café". */
function normalize(text: string): string {
  return text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase();
}
