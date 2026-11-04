import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { App } from './App';
import { mockCommands, shellError } from './test/ipc';

describe('App', () => {
  it('shows the unlocked vault with navigation when already unlocked', async () => {
    mockCommands({ vault_status: () => ({ exists: true, unlocked: true }), list_items: () => [] });
    render(<App />);
    expect(await screen.findByRole('heading', { level: 1, name: 'Your vault' })).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Vault' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Skip to main content' })).toHaveAttribute(
      'href',
      '#main',
    );
  });

  it('shows a startup error and retries', async () => {
    let attempts = 0;
    mockCommands({
      vault_status: () => {
        attempts += 1;
        if (attempts === 1) throw shellError('storage');
        return { exists: true, unlocked: true };
      },
      list_items: () => [],
    });
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('could not be read or written');
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Your vault' })).toBeInTheDocument();
  });

  it('locks the vault from the header', async () => {
    const calls = mockCommands({
      vault_status: () => ({ exists: true, unlocked: true }),
      list_items: () => [],
      lock: () => null,
    });
    render(<App />);
    await userEvent.click(await screen.findByRole('button', { name: 'Lock' }));
    expect(calls.map((call) => call.command)).toContain('lock');
    expect(screen.queryByRole('button', { name: 'Lock' })).not.toBeInTheDocument();
    expect(screen.queryByRole('navigation')).not.toBeInTheDocument();
  });
});
