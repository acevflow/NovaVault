import { useCallback, useEffect, useState } from 'react';

import { listItems } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import type { ItemDetails as Details, ItemKind, ItemSummary } from '../../api/types';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { ScreenHeading } from '../../components/ScreenHeading';
import { ItemDetails } from './ItemDetails';
import { ItemForm } from './ItemForm';
import { ItemList } from './ItemList';
import { ITEM_KINDS, KIND_LABELS } from './kinds';

/** Props of {@link ItemsScreen}. */
export interface ItemsScreenProps {
  /** Called if a command reports that the vault is locked. */
  readonly onLocked: () => void;
}

/** What the panel next to the list shows. */
type Panel =
  | { readonly mode: 'none' }
  | { readonly mode: 'view'; readonly id: string }
  | { readonly mode: 'edit'; readonly details: Details }
  | { readonly mode: 'create'; readonly kind: ItemKind };

/** The unlocked vault's main screen: the item list and the selected item. */
export function ItemsScreen({ onLocked }: ItemsScreenProps) {
  const [items, setItems] = useState<readonly ItemSummary[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [panel, setPanel] = useState<Panel>({ mode: 'none' });
  const [notice, setNotice] = useState<string | null>(null);
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

  function show(next: Panel, message: string | null = null) {
    setNotice(message);
    setPanel(next);
  }

  const selectedId =
    panel.mode === 'view' ? panel.id : panel.mode === 'edit' ? panel.details.id : null;

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
        <>
          <div className="actions items__new">
            {ITEM_KINDS.map((kind) => (
              <Button
                key={kind}
                variant={kind === 'password' ? 'primary' : 'secondary'}
                onClick={() => {
                  show({ mode: 'create', kind });
                }}
              >
                New {KIND_LABELS[kind].toLowerCase()}
              </Button>
            ))}
          </div>
          <div className="items">
            <ItemList
              items={items}
              selectedId={selectedId}
              onSelect={(id) => {
                show({ mode: 'view', id });
              }}
            />
            <div className="items__detail">
              {notice === null ? null : <Alert tone="success">{notice}</Alert>}
              <PanelContent panel={panel} onLocked={onLocked} onShow={show} onChanged={reload} />
            </div>
          </div>
        </>
      )}
    </div>
  );
}

interface PanelContentProps {
  readonly panel: Panel;
  readonly onLocked: () => void;
  readonly onShow: (next: Panel, message?: string | null) => void;
  readonly onChanged: () => void;
}

function PanelContent({ panel, onLocked, onShow, onChanged }: PanelContentProps) {
  switch (panel.mode) {
    case 'none':
      return <p className="muted">Select an item to see its details, or add a new one.</p>;
    case 'view':
      return (
        <ItemDetails
          // A new key per item drops revealed values and resets focus.
          key={panel.id}
          itemId={panel.id}
          onLocked={onLocked}
          onEdit={(details) => {
            onShow({ mode: 'edit', details });
          }}
          onDeleted={() => {
            onChanged();
            onShow({ mode: 'none' }, 'Item deleted.');
          }}
        />
      );
    case 'edit':
      return (
        <ItemForm
          key={panel.details.id}
          kind={panel.details.kind}
          existing={panel.details}
          onLocked={onLocked}
          onCancel={() => {
            onShow({ mode: 'view', id: panel.details.id });
          }}
          onSaved={(id) => {
            onChanged();
            onShow({ mode: 'view', id }, 'Changes saved.');
          }}
        />
      );
    case 'create':
      return (
        <ItemForm
          key={panel.kind}
          kind={panel.kind}
          existing={null}
          onLocked={onLocked}
          onCancel={() => {
            onShow({ mode: 'none' });
          }}
          onSaved={(id) => {
            onChanged();
            onShow({ mode: 'view', id }, 'Item added.');
          }}
        />
      );
  }
}
