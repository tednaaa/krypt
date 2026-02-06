interface Options {
  required?: boolean;
  optionalForDevelopment?: boolean;
}

type VariableName = `VITE_${string}`;

export function getEnv(variableName: VariableName): string;
export function getEnv(variableName: VariableName, options: { required: false }): string;
export function getEnv(variableName: VariableName, options: { optionalForDevelopment: true }): string | undefined;

export function getEnv(
  variableName: VariableName,
  options?: Options,
): string | undefined {
  const { required = true, optionalForDevelopment = false } = options || {};

  const value = import.meta.env[variableName]?.trim();

  if (optionalForDevelopment && import.meta.env.DEV && !value) {
    return undefined;
  }

  if (required && !value) {
    throw new TypeError(`Environment variable ${variableName} is required`);
  }

  return value;
}
