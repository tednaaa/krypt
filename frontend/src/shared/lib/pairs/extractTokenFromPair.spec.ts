import { extractTokenFromPair } from './extractTokenFromPair';

describe('extractTokenFromPair', () => {
  it('should extract token from pair', () => {
    expect(extractTokenFromPair('BTCUSDT')).toBe('BTC');
    expect(extractTokenFromPair('ETHUSDT')).toBe('ETH');
    expect(extractTokenFromPair('TRADOORUSDT')).toBe('TRADOOR');
  });
});
