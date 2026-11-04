import { describe, expect, it } from 'vitest';

import { normalizeRecoveryCode, recoveryCodeGroups, recoveryCodesMatch } from './recoveryCode';

describe('recovery code helpers', () => {
  const code = 'ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789';

  it('ignores case, spaces and dashes when comparing', () => {
    expect(normalizeRecoveryCode(' ab-cd ef ')).toBe('ABCDEF');
    expect(recoveryCodesMatch(code, code.toLowerCase().replaceAll('-', ' '))).toBe(true);
    expect(recoveryCodesMatch(code, code.slice(0, -1))).toBe(false);
    expect(recoveryCodesMatch('', '')).toBe(false);
  });

  it('splits the display form into groups', () => {
    expect(recoveryCodeGroups(code)).toHaveLength(8);
    expect(recoveryCodeGroups(code)[0]).toBe('ABCD');
  });
});
