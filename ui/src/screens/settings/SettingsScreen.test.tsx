import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { mockCommands, shellError, TEST_RECOVERY_CODE, type CommandHandlers } from '../../test/ipc';
import { ThemeProvider } from '../../theme/ThemeProvider';
import { SettingsScreen } from './SettingsScreen';

const STRONG = 'violet anchor gravel summit 42 lantern';

function renderSettings(handlers: CommandHandlers, onLocked = vi.fn()) {
  const calls = mockCommands({
    get_settings: () => ({ autoLockMinutes: 5, clipboardClearSeconds: 30 }),
    ...handlers,
  });
  render(
    <ThemeProvider>
      <SettingsScreen onLocked={onLocked} />
    </ThemeProvider>,
  );
  return { calls, onLocked, user: userEvent.setup() };
}

function section(name: string) {
  return within(screen.getByRole('region', { name }));
}

describe('SettingsScreen: master password', () => {
  async function fillChange(user: ReturnType<typeof userEvent.setup>, current: string) {
    const form = section('Master password');
    await user.type(form.getByLabelText('Current master password'), current);
    await user.type(form.getByLabelText('New master password'), STRONG);
    await user.type(form.getByLabelText('Confirm new master password'), STRONG);
    expect(await form.findByText(/Very strong/, {}, { timeout: 10_000 })).toBeInTheDocument();
    await user.click(form.getByRole('button', { name: 'Change master password' }));
  }

  it('changes the password after re-authentication', async () => {
    const { calls, user } = renderSettings({ change_master_password: () => null });
    await fillChange(user, 'old secret');
    expect(await screen.findByText(/Master password changed/)).toBeInTheDocument();
    expect(calls.find((call) => call.command === 'change_master_password')?.args).toEqual({
      currentCredential: { method: 'password', secret: 'old secret' },
      newPassword: STRONG,
    });
    expect(section('Master password').getByLabelText('New master password')).toHaveValue('');
  });

  it('reports a wrong current password', async () => {
    const { user } = renderSettings({
      change_master_password: () => {
        throw shellError('invalid_credentials');
      },
    });
    await fillChange(user, 'wrong');
    expect(await section('Master password').findByRole('alert')).toHaveTextContent(
      'Nothing was changed',
    );
    expect(section('Master password').getByLabelText('Current master password')).toHaveValue('');
  });

  it('accepts the recovery code as the current credential', async () => {
    const { calls, user } = renderSettings({ change_master_password: () => null });
    const form = section('Master password');
    await user.click(form.getByRole('radio', { name: 'Recovery code' }));
    await user.type(form.getByLabelText('Current recovery code'), TEST_RECOVERY_CODE);
    await user.type(form.getByLabelText('New master password'), STRONG);
    await user.type(form.getByLabelText('Confirm new master password'), STRONG);
    await form.findByText(/Very strong/, {}, { timeout: 10_000 });
    await user.click(form.getByRole('button', { name: 'Change master password' }));
    await screen.findByText(/Master password changed/);
    expect(calls.find((call) => call.command === 'change_master_password')?.args).toMatchObject({
      currentCredential: { method: 'recoveryCode', secret: TEST_RECOVERY_CODE },
    });
  });
});

describe('SettingsScreen: recovery code', () => {
  it('regenerates and confirms a new code', async () => {
    const { calls, user } = renderSettings({
      regenerate_recovery_code: () => ({ recoveryCode: TEST_RECOVERY_CODE }),
    });
    const form = section('Recovery code');
    expect(form.getByText(/stops working/)).toBeInTheDocument();
    await user.type(form.getByLabelText('Current master password'), 'pw');
    await user.click(form.getByRole('button', { name: 'Generate new recovery code' }));

    expect(
      await form.findByRole('heading', { level: 3, name: 'Save your recovery code' }),
    ).toHaveFocus();
    expect(calls.find((call) => call.command === 'regenerate_recovery_code')?.args).toEqual({
      currentCredential: { method: 'password', secret: 'pw' },
    });
    await user.click(form.getByLabelText(/I have saved my recovery code/));
    await user.click(form.getByRole('button', { name: 'Continue' }));
    await user.type(form.getByLabelText('Recovery code'), TEST_RECOVERY_CODE);
    await user.click(form.getByRole('button', { name: 'Confirm' }));

    expect(form.getByText(/saved and confirmed/)).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent('ABCD');
  });

  it('keeps the old code when re-authentication fails', async () => {
    const { user } = renderSettings({
      regenerate_recovery_code: () => {
        throw shellError('invalid_credentials');
      },
    });
    const form = section('Recovery code');
    await user.type(form.getByLabelText('Current master password'), 'x');
    await user.click(form.getByRole('button', { name: 'Generate new recovery code' }));
    expect(await form.findByRole('alert')).toHaveTextContent('was not changed');
  });

  it('requires a current credential before calling the shell', async () => {
    const { calls, user } = renderSettings({});
    await user.click(
      section('Recovery code').getByRole('button', { name: 'Generate new recovery code' }),
    );
    expect(section('Recovery code').getByRole('alert')).toHaveTextContent('Enter your current');
    expect(calls.map((call) => call.command)).not.toContain('regenerate_recovery_code');
  });
});

describe('SettingsScreen: auto-lock and appearance', () => {
  it('loads and saves the auto-lock timeout', async () => {
    const { calls, user } = renderSettings({ set_auto_lock_timeout: () => null });
    const select = await section('Auto-lock').findByLabelText('Lock after inactivity');
    expect(select).toHaveValue('5');
    await user.selectOptions(select, '15');
    await user.click(section('Auto-lock').getByRole('button', { name: 'Save' }));
    expect(await section('Auto-lock').findByRole('status')).toHaveTextContent('15 minutes');
    expect(calls.at(-1)).toEqual({ command: 'set_auto_lock_timeout', args: { minutes: 15 } });
  });

  it('returns to the unlock screen if the vault is locked', async () => {
    const { user, onLocked } = renderSettings({
      set_auto_lock_timeout: () => {
        throw shellError('locked');
      },
    });
    await user.selectOptions(
      await section('Auto-lock').findByLabelText('Lock after inactivity'),
      '30',
    );
    await user.click(section('Auto-lock').getByRole('button', { name: 'Save' }));
    await vi.waitFor(() => {
      expect(onLocked).toHaveBeenCalledOnce();
    });
  });

  it('loads and saves the clipboard clear delay', async () => {
    const { calls, user } = renderSettings({ set_clipboard_clear_delay: () => null });
    const select = await section('Clipboard').findByLabelText('Clear the clipboard after');
    expect(select).toHaveValue('30');
    await user.selectOptions(select, '1 minute');
    await user.click(section('Clipboard').getByRole('button', { name: 'Save' }));
    expect(await section('Clipboard').findByRole('status')).toHaveTextContent('after 1 minute');
    expect(calls.at(-1)).toEqual({ command: 'set_clipboard_clear_delay', args: { seconds: 60 } });
  });

  it('switches the theme', async () => {
    const { user } = renderSettings({});
    await user.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    await user.click(screen.getByRole('radio', { name: 'Match system' }));
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });
});
