import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { App } from '../../App';
import { mockCommands, shellError, TEST_RECOVERY_CODE } from '../../test/ipc';
import { UnlockScreen } from './UnlockScreen';

// The tab panel is labelled by its tab, so field queries target the input.
const INPUT = { selector: 'input' };

describe('UnlockScreen', () => {
  it('unlocks with the master password', async () => {
    const calls = mockCommands({ unlock: () => null });
    const onUnlocked = vi.fn();
    render(<UnlockScreen onUnlocked={onUnlocked} />);
    expect(screen.getByRole('heading', { name: 'Unlock your vault' })).toHaveFocus();

    await userEvent.type(screen.getByLabelText('Master password', INPUT), 'my secret{Enter}');
    expect(calls).toEqual([
      { command: 'unlock', args: { credential: { method: 'password', secret: 'my secret' } } },
    ]);
    expect(onUnlocked).toHaveBeenCalledOnce();
  });

  it('unlocks with the recovery code from its tab', async () => {
    const calls = mockCommands({ unlock: () => null });
    const user = userEvent.setup();
    render(<UnlockScreen onUnlocked={vi.fn()} />);

    const passwordTab = screen.getByRole('tab', { name: 'Master password' });
    expect(passwordTab).toHaveAttribute('aria-selected', 'true');
    // Keyboard navigation between tabs.
    passwordTab.focus();
    await user.keyboard('{ArrowRight}');
    const codeTab = screen.getByRole('tab', { name: 'Recovery code' });
    expect(codeTab).toHaveFocus();
    expect(codeTab).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Recovery code');

    await user.type(screen.getByLabelText('Recovery code', INPUT), `${TEST_RECOVERY_CODE}{Enter}`);
    expect(calls[0]?.args).toEqual({
      credential: { method: 'recoveryCode', secret: TEST_RECOVERY_CODE },
    });
  });

  it('reports a wrong password and clears the field', async () => {
    mockCommands({
      unlock: () => {
        throw shellError('invalid_credentials');
      },
    });
    const onUnlocked = vi.fn();
    render(<UnlockScreen onUnlocked={onUnlocked} />);
    const field = screen.getByLabelText('Master password', INPUT);
    await userEvent.type(field, 'wrong{Enter}');

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'That master password is not correct.',
    );
    expect(field).toHaveValue('');
    expect(field).toHaveFocus();
    expect(onUnlocked).not.toHaveBeenCalled();
  });

  it.each([
    ['invalid_recovery_code', 'Check it for typos'],
    ['unsupported_vault', 'not supported by this version'],
    ['corrupted', 'damaged'],
  ] as const)('explains %s errors', async (code, text) => {
    mockCommands({
      unlock: () => {
        throw shellError(code);
      },
    });
    const user = userEvent.setup();
    render(<UnlockScreen onUnlocked={vi.fn()} />);
    await user.click(screen.getByRole('tab', { name: 'Recovery code' }));
    await user.type(screen.getByLabelText('Recovery code', INPUT), 'abcd{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent(text);
  });

  it('asks for a secret before calling the shell', async () => {
    const calls = mockCommands({});
    render(<UnlockScreen onUnlocked={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Unlock' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Enter your master password.');
    expect(calls).toHaveLength(0);
  });
});

describe('lock and unlock in the app', () => {
  it('goes from unlock to the vault and back on lock', async () => {
    mockCommands({
      vault_status: () => ({ exists: true, unlocked: false }),
      unlock: () => null,
      lock: () => null,
    });
    const user = userEvent.setup();
    render(<App />);
    await user.type(await screen.findByLabelText('Master password', INPUT), 'pw{Enter}');
    expect(await screen.findByRole('heading', { name: 'Your vault' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Lock' }));
    expect(await screen.findByRole('heading', { name: 'Unlock your vault' })).toBeInTheDocument();
    expect(screen.getByLabelText('Master password', INPUT)).toHaveValue('');
  });
});
