import { useCallback, useEffect, useState } from 'react';

import { listItems } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import type { ItemSummary } from '../../api/types';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { ScreenHeading } from '../../components/ScreenHeading';
import { ItemList } from './ItemList';

/** Props of {@link ItemsScreen}. */
export interface ItemsScreenProps {
  /** Called if a command reports that the vault is locked. */
  readonly onLocked: () => void;
}

/** The unlocked vault's main screen: the item list and the selected item. */
export function ItemsScreen({ onLocked }: ItemsScreenProps) {
  const [items, setItems] = useState<readonly ItemSummary[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // Bumped to reload the list: on retry, and after every change.
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let active = true;
    listItems()
      .then((loaded) => {
        if (active) setItems(loaded);
      })
      .catch((reason: unknown) => {
        if (!active) return;
        if (isCommandError(reason, 'locked')) {
          onLocked();
          return;
        }
        setLoadError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
  }, [revision, onLocked]);

  const reload = useCallback(() => {
    setLoadError(null);
    setRevision((current) => current + 1);
  }, []);

  return (
    <div className="screen screen--wide">
      <ScreenHeading>Your vault</ScreenHeading>
      {loadError === null ? null : (
        <div className="stack">
          <Alert tone="error">{loadError}</Alert>
          <Button onClick={reload}>Try again</Button>
        </div>
      )}
      {items === null ? (
        loadError === null ? (
          <p role="status">Loading items…</p>
        ) : null
      ) : (
        <div className="items">
          <ItemList items={items} selectedId={selectedId} onSelect={setSelectedId} />
          <div className="items__detail">
            <p className="muted">Select an item to see its details.</p>
          </div>
        </div>
      )}
    </div>
  );
}
