import { useId, useState } from 'react';

import type { ItemSummary } from '../../api/types';
import { TextField } from '../../components/TextField';
import { filterItems, type KindFilter } from './filterItems';
import { ITEM_KINDS, KIND_LABELS, KIND_PLURALS } from './kinds';

/** Props of {@link ItemList}. */
export interface ItemListProps {
  /** Every item of the vault. */
  readonly items: readonly ItemSummary[];
  /** The item whose details are shown, if any. */
  readonly selectedId: string | null;
  /** Shows an item's details. */
  readonly onSelect: (id: string) => void;
}

function countLabel(shown: number, total: number): string {
  const noun = total === 1 ? 'item' : 'items';
  return shown === total
    ? `${String(total)} ${noun}`
    : `${String(shown)} of ${String(total)} ${noun}`;
}

/** The vault's items, with a title search and a type filter. */
export function ItemList({ items, selectedId, onSelect }: ItemListProps) {
  const headingId = useId();
  const kindId = useId();
  const [query, setQuery] = useState('');
  const [kind, setKind] = useState<KindFilter>('all');
  const visible = filterItems(items, query, kind);

  return (
    <section className="card items__list" aria-labelledby={headingId}>
      <h2 id={headingId}>Items</h2>
      <div role="search" aria-label="Filter items" className="items__filters">
        <TextField
          label="Search by title"
          type="search"
          value={query}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
        <div className="field">
          <label className="field__label" htmlFor={kindId}>
            Type
          </label>
          <select
            id={kindId}
            className="select"
            value={kind}
            onChange={(event) => {
              setKind(event.target.value as KindFilter);
            }}
          >
            <option value="all">All types</option>
            {ITEM_KINDS.map((option) => (
              <option key={option} value={option}>
                {KIND_PLURALS[option]}
              </option>
            ))}
          </select>
        </div>
      </div>
      <p role="status" className="muted items__count">
        {countLabel(visible.length, items.length)}
      </p>
      {items.length === 0 ? (
        <p>Your vault is empty. Add a password, note or secret to get started.</p>
      ) : visible.length === 0 ? (
        <p>No items match your search.</p>
      ) : (
        <ul className="item-list">
          {visible.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                className="item-list__item"
                aria-current={item.id === selectedId ? 'true' : undefined}
                onClick={() => {
                  onSelect(item.id);
                }}
              >
                <span className="item-list__title">{item.title || 'Untitled'}</span>
                <span className="item-list__kind">{KIND_LABELS[item.kind]}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
