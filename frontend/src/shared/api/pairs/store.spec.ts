import { buildPairsSortParam } from './store';

describe('buildPairsSortParam', () => {
  it('returns empty string when no sorting is provided', () => {
    expect(buildPairsSortParam([])).toBe('');
  });

  it('serializes sorting order and direction', () => {
    const sorting = [
      { id: 'mfi_1h', desc: true },
      { id: 'mfi_4h', desc: false },
    ];

    expect(buildPairsSortParam(sorting)).toBe('mfi_1h:desc,mfi_4h:asc');
  });

  it('supports price sorting', () => {
    const sorting = [
      { id: 'price', desc: true },
    ];

    expect(buildPairsSortParam(sorting)).toBe('price:desc');
  });

  it('skips unsupported fields', () => {
    const sorting = [
      { id: 'pair', desc: true },
      { id: 'mfi_1d', desc: false },
    ];

    expect(buildPairsSortParam(sorting)).toBe('mfi_1d:asc');
  });
});
