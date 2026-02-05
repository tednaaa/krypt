import { getEnv } from './get-env';

describe('getEnv', () => {
  beforeEach(() => {
    vi.unstubAllEnvs();
  });

  it('should return the environment variable value when it exists and required', () => {
    vi.stubEnv('VITE_TEST_ENV', 'test-value');

    const result = getEnv('VITE_TEST_ENV');
    expect(result).toBe('test-value');
  });

  it('should throw TypeError when required variable is undefined', () => {
    expect(() => getEnv('VITE_MISSING_VAR')).toThrow(TypeError);
  });

  it('should throw TypeError when required variable is empty string', () => {
    vi.stubEnv('VITE_EMPTY_VAR', '   ');

    expect(() => getEnv('VITE_EMPTY_VAR')).toThrow(TypeError);
  });

  it('should not throw error when required false variable does not exist', () => {
    const env = getEnv('VITE_MISSING_VAR', { required: false });

    expect(() => env).not.toThrow(TypeError);
    expect(env).toBeUndefined();
  });

  describe('production environment', () => {
    beforeEach(() => {
      vi.stubEnv('PROD', true);
      vi.stubEnv('DEV', false);
    });

    it('should throw TypeError when optionalForDevelopment variable does not exist', () => {
      expect(() => getEnv('VITE_TEST_ENV', { optionalForDevelopment: true })).toThrow(TypeError);
    });

    it('should return value when optionalForDevelopment variable exists', () => {
      vi.stubEnv('VITE_TEST_ENV', 'test-value');
      const result = getEnv('VITE_TEST_ENV', { optionalForDevelopment: true });
      expect(result).toBe('test-value');
    });
  });

  describe('development environment', () => {
    beforeEach(() => {
      vi.stubEnv('DEV', true);
      vi.stubEnv('PROD', false);
    });

    it('should return value when optionalForDevelopment variable exists', () => {
      vi.stubEnv('VITE_TEST_ENV', 'test-value');
      const result = getEnv('VITE_TEST_ENV', { optionalForDevelopment: true });
      expect(result).toBe('test-value');
    });

    it('should not throw error when optionalForDevelopment variable does not exist', () => {
      const env = getEnv('VITE_TEST_ENV', { optionalForDevelopment: true });
      expect(() => env).not.toThrow(TypeError);
      expect(env).toBeUndefined();
    });
  });
});
