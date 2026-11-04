import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { mockCommands, shellError, type CommandHandlers } from '../test/ipc';
import { SecretField } from './SecretField';

const ID = 'a'.repeat(32);

function renderField(handlers: CommandHandlers, hasValue = true, onLocked = vi.fn()) {
  const calls = mockCommands(handlers);
  const view = render(
    <dl>
      <SecretField
        label="Password"
        itemId={ID}
        field="password"
        hasValue={hasValue}
        onLocked={onLocked}
      />
    </dl>,
  );
  return { calls, onLocked, view, user: userEvent.setup() };
}

describe('SecretField', () => {
  it('is hidden by default and fetches nothing', () => {
    const { calls } = renderField({});
    expect(screen.getByText('Hidden')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reveal password' })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
    expect(calls).toEqual([]);
  });

  it('reveals the value on demand and drops it when hidden', async () => {
    const { calls, user } = renderField({ reveal_item_field: () => ({ value: 'hunter2' }) });
    const toggle = screen.getByRole('button', { name: 'Reveal password' });

    await user.click(toggle);
    expect(await screen.findByText('hunter2')).toBeInTheDocument();
    expect(toggle).toHaveAttribute('aria-pressed', 'true');
    expect(toggle).toHaveTextContent('Hide');
    expect(calls).toEqual([{ command: 'reveal_item_field', args: { id: ID, field: 'password' } }]);

    await user.click(toggle);
    expect(screen.queryByText('hunter2')).not.toBeInTheDocument();
    expect(screen.getByText('Hidden')).toBeInTheDocument();
  });

  it('copies through the shell without revealing the value', async () => {
    const { calls, user } = renderField({ copy_item_field: () => ({ clearAfterSeconds: 30 }) });
    await user.click(screen.getByRole('button', { name: 'Copy password' }));
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Copied. The clipboard clears in 30 seconds.',
    );
    expect(calls).toEqual([{ command: 'copy_item_field', args: { id: ID, field: 'password' } }]);
    expect(screen.getByText('Hidden')).toBeInTheDocument();
  });

  it('reports a clipboard that is not available', async () => {
    const { user } = renderField({
      copy_item_field: () => {
        throw shellError('clipboard_unavailable');
      },
    });
    await user.click(screen.getByRole('button', { name: 'Copy password' }));
    expect(await screen.findByRole('status')).toHaveTextContent('clipboard is not available');
  });

  it('says when the field is empty and offers no reveal', () => {
    renderField({}, false);
    expect(screen.getByText('Empty')).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('reports a failure to reveal', async () => {
    const { user } = renderField({
      reveal_item_field: () => {
        throw shellError('item_not_found');
      },
    });
    await user.click(screen.getByRole('button', { name: 'Reveal password' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('no longer exists');
  });

  it('returns to the unlock screen if the vault is locked', async () => {
    const { onLocked, user } = renderField({
      reveal_item_field: () => {
        throw shellError('locked');
      },
    });
    await user.click(screen.getByRole('button', { name: 'Reveal password' }));
    await vi.waitFor(() => {
      expect(onLocked).toHaveBeenCalled();
    });
  });
});
